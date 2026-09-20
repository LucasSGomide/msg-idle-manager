//! One phone socket, from `hello` to the end: the proof exchange, the
//! forwarding of the phone's messages as intents, the pictures and state
//! going back, and the heartbeat that decides when a silent phone is gone.
//!
//! Two threads serve a socket. The connection thread reads: it demands the
//! proof, then turns each text message into a [`RemoteIntent`] or a presence
//! event. A pump thread writes: it drains the socket's [`Outbox`], encoding
//! frames on the way (architecture rule 10), so a slow phone never blocks the
//! window's `publish_state` or the reader's heartbeat check.

use std::collections::VecDeque;
use std::net::{Shutdown, TcpStream};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use hmac::{Hmac, KeyInit as _, Mac as _};
use idle_manager_core::{Frame, Presence, PresenceChange, RemoteIntent};
use sha2::Sha256;

use crate::frames::{self, Dedup};
use crate::protocol::{ByeReason, PhoneMessage, ServerMessage, decode_hex, encode_hex};
use crate::websocket::{self, Message, Read, WsError};
use crate::{Shared, random_bytes};

/// How long a read waits before the heartbeat is re-evaluated. Short enough
/// that a phone gone silent is noticed within a second of the limit.
pub(crate) const READ_TIMEOUT_SECS: u64 = 1;

/// How long an upgraded socket may wait for its `auth` before it is dropped,
/// so a socket that never proves anything does not pin a thread.
const AUTH_TIMEOUT_SECS: u64 = 10;

/// The most bytes one message from the phone may carry.
const MAX_MESSAGE_BYTES: usize = 64 * 1024;

/// The server's challenge: 32 random bytes, hex on the wire.
const CHALLENGE_BYTES: usize = 32;

/// What the phone signs to prove the secret, and what the desktop signs back.
const PHONE_PROOF_PREFIX: &[u8] = b"phone|";
const DESKTOP_PROOF_PREFIX: &[u8] = b"desktop|";

type HmacSha256 = Hmac<Sha256>;

/// The queue of everything waiting to go down one socket, drained by its pump
/// thread: framed messages in order, and one slot for the latest frame —
/// a newer picture always replaces an unsent older one.
#[derive(Debug, Default)]
pub(crate) struct Outbox {
    inner: Mutex<OutboxState>,
    wake: Condvar,
}

#[derive(Debug, Default)]
struct OutboxState {
    queue: VecDeque<Vec<u8>>,
    frame: Option<Frame>,
    closing: bool,
    abandoned: bool,
}

/// What the pump thread should do next.
#[derive(Debug)]
enum Next {
    Send(Vec<u8>),
    Encode(Frame),
    Finish,
    Abandoned,
}

impl Outbox {
    /// Queues an already-framed message.
    pub(crate) fn push(&self, framed: Vec<u8>) {
        self.inner().queue.push_back(framed);
        self.wake.notify_one();
    }

    /// Offers a frame; an unsent earlier one is dropped in its favour.
    pub(crate) fn offer_frame(&self, frame: Frame) {
        self.inner().frame = Some(frame);
        self.wake.notify_one();
    }

    /// Asks the pump to send everything queued, then a close frame, then
    /// shut the socket.
    pub(crate) fn close_after_flush(&self) {
        self.inner().closing = true;
        self.wake.notify_one();
    }

    /// Tells the pump the reader is gone and nothing more will be sent.
    pub(crate) fn abandon(&self) {
        self.inner().abandoned = true;
        self.wake.notify_one();
    }

    fn next(&self) -> Next {
        let mut state = self.inner();
        loop {
            if state.abandoned {
                return Next::Abandoned;
            }
            if let Some(framed) = state.queue.pop_front() {
                return Next::Send(framed);
            }
            if let Some(frame) = state.frame.take() {
                return Next::Encode(frame);
            }
            if state.closing {
                return Next::Finish;
            }
            state = self
                .wake
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    fn inner(&self) -> MutexGuard<'_, OutboxState> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The one socket that has proven the secret, as the shared state holds it.
#[derive(Debug)]
pub(crate) struct ActiveSocket {
    pub(crate) id: u64,
    pub(crate) outbox: Arc<Outbox>,
}

impl ActiveSocket {
    /// Says goodbye for `reason` and has the socket closed once that is sent.
    pub(crate) fn dismiss(&self, reason: ByeReason) {
        self.outbox.push(websocket::frame_text(
            &ServerMessage::Bye { reason }.to_json(),
        ));
        self.outbox.close_after_flush();
    }
}

/// Runs one upgraded socket to its end on the calling thread.
pub(crate) fn run(mut stream: TcpStream, shared: &Arc<Shared>) {
    if let Err(error) = stream.set_read_timeout(Some(Duration::from_secs(READ_TIMEOUT_SECS))) {
        tracing::warn!(%error, "could not set the socket read timeout");
        return;
    }
    let challenge = encode_hex(&random_bytes(CHALLENGE_BYTES));
    let hello = websocket::frame_text(
        &ServerMessage::Hello {
            challenge: challenge.clone(),
        }
        .to_json(),
    );
    if let Err(error) = websocket::send(&mut stream, &hello) {
        tracing::debug!(%error, "hello could not be sent");
        return;
    }
    let Some(secret) = authenticate(&mut stream, shared, &challenge) else {
        shutdown(&stream);
        return;
    };
    let Ok(writer) = stream.try_clone() else {
        tracing::warn!("the socket could not be cloned for its pump thread");
        shutdown(&stream);
        return;
    };
    let outbox = Arc::new(Outbox::default());
    let socket_id = shared.register_socket(Arc::clone(&outbox), secret.welcome_proof);
    let pump_shared = Arc::clone(shared);
    let pump_outbox = Arc::clone(&outbox);
    let spawned = thread::Builder::new()
        .name("remote-pump".to_owned())
        .spawn(move || pump(&pump_outbox, writer, &pump_shared));
    if let Err(error) = spawned {
        tracing::error!(%error, "the pump thread could not be spawned");
        shared.socket_closed(socket_id);
        shutdown(&stream);
        return;
    }
    converse(&mut stream, shared, &outbox);
    shared.socket_closed(socket_id);
    outbox.abandon();
    shutdown(&stream);
}

/// What a right `auth` yields: the desktop's answer, ready for `welcome`.
struct Proven {
    welcome_proof: String,
}

/// Waits for the phone's `auth` and checks it. `None` closes silently: a
/// wrong proof, no enrolled phone, the auth timeout, or the peer going away.
fn authenticate(stream: &mut TcpStream, shared: &Shared, challenge: &str) -> Option<Proven> {
    let started = shared.now_millis();
    loop {
        match websocket::read_message(stream, MAX_MESSAGE_BYTES) {
            Ok(Read::Quiet) => {
                if shared.now_millis().saturating_sub(started) >= AUTH_TIMEOUT_SECS * 1000 {
                    tracing::debug!("no auth within the timeout; closing");
                    return None;
                }
            }
            Ok(Read::Message(Message::Text(text))) => match PhoneMessage::parse(&text) {
                Ok(PhoneMessage::Auth {
                    proof,
                    challenge: phone_challenge,
                }) => return verify(shared, challenge, &proof, &phone_challenge),
                Ok(other) => {
                    tracing::warn!(message = ?other, "a message before auth was dropped");
                }
                Err(error) => tracing::warn!(%error, "a message before auth was dropped"),
            },
            Ok(Read::Message(Message::Ping(payload))) => {
                if let Err(error) = websocket::send(stream, &websocket::frame_pong(&payload)) {
                    tracing::debug!(%error, "pong could not be sent");
                    return None;
                }
            }
            Ok(Read::Message(Message::Pong(_) | Message::Binary(_))) => {}
            Ok(Read::Message(Message::Close)) => return None,
            Err(error) => {
                tracing::debug!(%error, "the socket ended before auth");
                return None;
            }
        }
    }
}

/// Checks `proof` in constant time against the enrolled secret and, if it
/// holds, signs the phone's challenge back.
fn verify(shared: &Shared, challenge: &str, proof: &str, phone_challenge: &str) -> Option<Proven> {
    let secret = shared.enrolled_secret()?;
    let Some(proof_bytes) = decode_hex(proof) else {
        tracing::warn!("the auth proof was not hex; closing");
        return None;
    };
    let mut expected = mac(&secret)?;
    expected.update(PHONE_PROOF_PREFIX);
    expected.update(challenge.as_bytes());
    if expected.verify_slice(&proof_bytes).is_err() {
        tracing::warn!("the auth proof did not verify; closing");
        return None;
    }
    let mut answer = mac(&secret)?;
    answer.update(DESKTOP_PROOF_PREFIX);
    answer.update(phone_challenge.as_bytes());
    Some(Proven {
        welcome_proof: encode_hex(&answer.finalize().into_bytes()),
    })
}

fn mac(secret: &[u8]) -> Option<HmacSha256> {
    match HmacSha256::new_from_slice(secret) {
        Ok(mac) => Some(mac),
        Err(error) => {
            tracing::error!(%error, "the enrolled secret cannot key an HMAC");
            None
        }
    }
}

/// The authenticated loop: every read re-evaluates the heartbeat, every text
/// message becomes an intent or a presence event, and a WebSocket ping gets
/// its pong. Returns when the socket ends or the phone says close.
fn converse(stream: &mut TcpStream, shared: &Shared, outbox: &Outbox) {
    loop {
        shared.update_presence(Presence::observe);
        match websocket::read_message(stream, MAX_MESSAGE_BYTES) {
            Ok(Read::Quiet | Read::Message(Message::Pong(_))) => {}
            Ok(Read::Message(Message::Text(text))) => handle_text(&text, shared),
            Ok(Read::Message(Message::Ping(payload))) => {
                outbox.push(websocket::frame_pong(&payload));
            }
            Ok(Read::Message(Message::Binary(payload))) => {
                tracing::warn!(
                    bytes = payload.len(),
                    "a binary message from the phone was dropped"
                );
            }
            Ok(Read::Message(Message::Close)) => {
                outbox.close_after_flush();
                return;
            }
            Err(WsError::Closed) => return,
            Err(error) => {
                tracing::debug!(%error, "the socket ended");
                return;
            }
        }
    }
}

fn handle_text(text: &str, shared: &Shared) {
    let message = match PhoneMessage::parse(text) {
        Ok(message) => message,
        Err(error) => {
            tracing::warn!(%error, "a message from the phone was dropped");
            return;
        }
    };
    match message {
        PhoneMessage::Ping => shared.update_presence(|presence, now| {
            presence.heartbeat(now);
            PresenceChange::Unchanged
        }),
        PhoneMessage::Attach { viewport } => {
            shared.update_presence(Presence::attach);
            shared.emit(RemoteIntent::Attach {
                viewport: viewport.into(),
            });
        }
        PhoneMessage::Leave => shared.update_presence(|presence, _| presence.leave()),
        PhoneMessage::Auth { .. } => {
            tracing::warn!("an auth after welcome was dropped");
        }
        PhoneMessage::Unknown { kind } => {
            tracing::warn!(
                kind,
                "a message type the desktop does not understand was dropped"
            );
        }
        other => match RemoteIntent::try_from(other) {
            Ok(intent) => shared.emit(intent),
            Err(error) => tracing::warn!(%error, "a message from the phone was dropped"),
        },
    }
}

/// The pump: writes everything the outbox yields, encoding frames on the way,
/// until told to finish or abandoned by the reader.
fn pump(outbox: &Outbox, mut writer: TcpStream, shared: &Shared) {
    let mut dedup = Dedup::default();
    loop {
        match outbox.next() {
            Next::Send(framed) => {
                if let Err(error) = websocket::send(&mut writer, &framed) {
                    tracing::debug!(%error, "a message could not be sent; closing");
                    break;
                }
            }
            Next::Encode(frame) => {
                if !shared.wants_frames() {
                    continue;
                }
                let encoded = match frames::encode(frame) {
                    Ok(encoded) => encoded,
                    Err(error) => {
                        tracing::warn!(%error, "a frame was dropped");
                        continue;
                    }
                };
                if !dedup.admit(&encoded.jpeg) {
                    continue;
                }
                if let Err(error) =
                    websocket::send(&mut writer, &websocket::frame_binary(&encoded.message()))
                {
                    tracing::debug!(%error, "a frame could not be sent; closing");
                    break;
                }
            }
            Next::Finish => {
                if let Err(error) = websocket::send(&mut writer, &websocket::frame_close()) {
                    tracing::debug!(%error, "the close frame could not be sent");
                }
                break;
            }
            Next::Abandoned => break,
        }
    }
    shutdown(&writer);
}

fn shutdown(stream: &TcpStream) {
    if let Err(error) = stream.shutdown(Shutdown::Both) {
        tracing::trace!(%error, "the socket was already shut down");
    }
}
