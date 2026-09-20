//! The phone server: the second adapter that drives the application from
//! outside, beside the shell (roadmap item 13).
//!
//! It turns network messages into [`idle_manager_core::RemoteIntent`]s and
//! [`idle_manager_core::RemoteState`] into network messages, serving three
//! routes over plain `std::net` sockets on its own threads — no async runtime
//! and no GTK. It depends on `idle-manager-core` alone and never on the shell,
//! the store or metrics (architecture rule 2); `make arch-check` fails the
//! build if that changes.
//!
//! [`RemoteServer::start`] binds the address, spawns the accept thread and
//! hands back a [`RemoteHandle`] — the shell's [`PhoneLink`] — and the
//! channel the phone's intents arrive on. Time reaches the server through a
//! [`Clock`], so a test can drive the fifteen-second silence limit without
//! waiting for it (architecture rule 9).

mod address;
mod frames;
mod http;
mod protocol;
mod session;
mod websocket;

use std::fmt;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use idle_manager_core::{
    DEFAULT_MOBILE_VIEWPORT, EnrolledPhone, EnrolmentOffer, Frame, PhoneLink, PhoneRecordStore,
    PhoneStatus, Presence, PresenceChange, RemoteIntent, RemoteState,
};

pub use address::{DEFAULT_PHONE_PORT, bind_address};
use protocol::{ByeReason, ServerMessage, WireState, encode_hex};
use session::{ActiveSocket, Outbox};

/// How long an enrolment code stays valid from the moment it is minted: ten
/// minutes, long enough to fetch a phone and scan a picture.
pub const ENROLMENT_CODE_TTL_SECS: u64 = 600;

/// The enrolment code: 32 random bytes, hex on the wire.
const ENROLMENT_CODE_BYTES: usize = 32;

/// The device id the phone presents in its cookie: 16 random bytes, hex.
const DEVICE_ID_BYTES: usize = 16;

/// The shared secret both sides prove they hold.
const SECRET_BYTES: usize = 32;

/// How long the accept thread pauses after a failed accept, so a persistent
/// failure does not spin a core.
const ACCEPT_RETRY_MILLIS: u64 = 100;

/// Where the server reads the time: milliseconds on a monotonic scale whose
/// origin does not matter, only its differences. Passed in rather than read
/// from `Instant` directly so a test can move it by hand.
pub trait Clock: fmt::Debug + Send + Sync {
    /// Milliseconds elapsed since the clock's own origin.
    fn now_millis(&self) -> u64;
}

/// The real clock: milliseconds since it was created.
#[derive(Debug)]
pub struct MonotonicClock {
    started: Instant,
}

impl MonotonicClock {
    /// A clock whose origin is now.
    #[must_use]
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
        }
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

impl Clock for MonotonicClock {
    fn now_millis(&self) -> u64 {
        u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX)
    }
}

/// What [`RemoteServer::start`] needs to know.
#[derive(Debug, Clone)]
pub struct RemoteConfig {
    /// The address to listen on; port 0 lets the system choose, and
    /// [`RemoteHandle::local_addr`] then names the port it chose.
    pub bind: SocketAddr,
    /// The clock the silence limit and the code expiry are judged by.
    pub clock: Arc<dyn Clock>,
}

impl RemoteConfig {
    /// A configuration listening on `bind` with the real clock.
    #[must_use]
    pub fn new(bind: SocketAddr) -> Self {
        Self {
            bind,
            clock: Arc::new(MonotonicClock::new()),
        }
    }
}

/// Why the server could not start. The composition root turns any of these
/// into [`PhoneStatus::NotListening`] rather than a failed launch (`FR.5.1`).
#[derive(Debug, thiserror::Error)]
pub enum StartError {
    /// No interface carries an address inside `100.64.0.0/10`.
    #[error("no address in 100.64.0.0/10 to listen on; is the mesh network up?")]
    NoMeshAddress,
    /// The `[listen] address` override is not an address.
    #[error("the listen override {text:?} is not an address")]
    BadOverride {
        /// The override as written.
        text: String,
    },
    /// The network interfaces could not be listed.
    #[error("the network interfaces could not be listed: {reason}")]
    Interfaces {
        /// The system's reason.
        reason: String,
    },
    /// The address could not be bound.
    #[error("could not listen on {address}: {reason}")]
    Bind {
        /// The address that was tried.
        address: SocketAddr,
        /// The system's reason.
        reason: String,
    },
    /// The accept thread could not be spawned.
    #[error("the accept thread could not be spawned: {reason}")]
    Thread {
        /// The system's reason.
        reason: String,
    },
}

/// The server. It has no instance: [`RemoteServer::start`] spawns its threads
/// and hands back the handle that reaches them.
#[derive(Debug)]
pub struct RemoteServer;

impl RemoteServer {
    /// Binds `config.bind`, starts accepting connections on a thread of its
    /// own, and returns the handle the shell holds as its [`PhoneLink`]
    /// together with the channel the phone's intents arrive on. The threads
    /// live as long as the process.
    ///
    /// # Errors
    ///
    /// [`StartError::Bind`] when the address is taken or not local, and
    /// [`StartError::Thread`] when no thread could be spawned.
    pub fn start(
        config: RemoteConfig,
        record: Arc<dyn PhoneRecordStore>,
    ) -> Result<(RemoteHandle, async_channel::Receiver<RemoteIntent>), StartError> {
        let listener = TcpListener::bind(config.bind).map_err(|error| StartError::Bind {
            address: config.bind,
            reason: error.to_string(),
        })?;
        let bound = listener.local_addr().map_err(|error| StartError::Bind {
            address: config.bind,
            reason: error.to_string(),
        })?;
        let (intents, receiver) = async_channel::unbounded();
        let shared = Arc::new(Shared {
            bind: bound,
            record,
            clock: config.clock,
            intents,
            state: Mutex::new(State::default()),
        });
        let accept_shared = Arc::clone(&shared);
        thread::Builder::new()
            .name("remote-accept".to_owned())
            .spawn(move || accept_loop(&listener, &accept_shared))
            .map_err(|error| StartError::Thread {
                reason: error.to_string(),
            })?;
        tracing::info!(address = %bound, "the phone server is listening");
        Ok((RemoteHandle { shared }, receiver))
    }
}

fn accept_loop(listener: &TcpListener, shared: &Arc<Shared>) {
    for connection in listener.incoming() {
        match connection {
            Ok(stream) => spawn_connection(stream, Arc::clone(shared)),
            Err(error) => {
                tracing::warn!(%error, "a connection could not be accepted");
                thread::sleep(Duration::from_millis(ACCEPT_RETRY_MILLIS));
            }
        }
    }
}

fn spawn_connection(stream: TcpStream, shared: Arc<Shared>) {
    let spawned = thread::Builder::new()
        .name("remote-connection".to_owned())
        .spawn(move || http::serve(stream, &shared));
    if let Err(error) = spawned {
        tracing::error!(%error, "a connection thread could not be spawned");
    }
}

/// The shell's side of the server: [`PhoneLink`] over the state the server's
/// threads share. Cheap to clone; every clone reaches the same server.
#[derive(Debug, Clone)]
pub struct RemoteHandle {
    shared: Arc<Shared>,
}

impl RemoteHandle {
    /// The address the server is actually listening on — the configured one,
    /// with the port the system chose when it was 0.
    #[must_use]
    pub fn local_addr(&self) -> SocketAddr {
        self.shared.bind
    }
}

impl PhoneLink for RemoteHandle {
    fn publish_state(&self, state: &RemoteState) {
        self.shared.publish_state(state);
    }

    fn publish_frame(&self, frame: Frame) {
        self.shared.publish_frame(frame);
    }

    fn begin_enrolment(&self) -> EnrolmentOffer {
        self.shared.begin_enrolment()
    }

    fn cancel_enrolment(&self) {
        self.shared.state().enrolment = None;
    }

    fn revoke_phone(&self) {
        self.shared.revoke_phone();
    }

    fn phone_status(&self) -> PhoneStatus {
        self.shared.phone_status()
    }
}

/// A [`PhoneLink`] for a desktop whose server could not start: every status
/// is [`PhoneStatus::NotListening`] with the reason, and everything else is a
/// no-op. The composition root hands this to the shell when
/// [`RemoteServer::start`] fails (`FR.5.1`).
#[derive(Debug, Clone)]
pub struct NotListeningLink {
    reason: String,
}

impl NotListeningLink {
    /// A link that reports `reason` — one line, ready to show as-is.
    #[must_use]
    pub fn new(reason: impl Into<String>) -> Self {
        Self {
            reason: reason.into(),
        }
    }
}

impl PhoneLink for NotListeningLink {
    fn publish_state(&self, _state: &RemoteState) {}

    fn publish_frame(&self, _frame: Frame) {}

    fn begin_enrolment(&self) -> EnrolmentOffer {
        EnrolmentOffer {
            address: String::new(),
            expires_in_secs: 0,
        }
    }

    fn cancel_enrolment(&self) {}

    fn revoke_phone(&self) {}

    fn phone_status(&self) -> PhoneStatus {
        PhoneStatus::NotListening {
            reason: self.reason.clone(),
        }
    }
}

/// Everything the server's threads and the handle share.
#[derive(Debug)]
pub(crate) struct Shared {
    bind: SocketAddr,
    record: Arc<dyn PhoneRecordStore>,
    clock: Arc<dyn Clock>,
    intents: async_channel::Sender<RemoteIntent>,
    state: Mutex<State>,
}

/// The part of [`Shared`] that changes.
#[derive(Debug, Default)]
struct State {
    enrolment: Option<Enrolment>,
    last_published: Option<RemoteState>,
    active: Option<ActiveSocket>,
    presence: Presence,
    next_socket_id: u64,
}

/// An open enrolment offer.
#[derive(Debug)]
struct Enrolment {
    code: String,
    expires_at_millis: u64,
}

impl Enrolment {
    fn is_live(&self, code: &str, now_millis: u64) -> bool {
        same_secret(self.code.as_bytes(), code.as_bytes()) && now_millis < self.expires_at_millis
    }
}

impl Shared {
    pub(crate) fn now_millis(&self) -> u64 {
        self.clock.now_millis()
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Puts an intent on the channel; a dropped receiver is logged, not fatal.
    pub(crate) fn emit(&self, intent: RemoteIntent) {
        if let Err(error) = self.intents.try_send(intent) {
            tracing::debug!(%error, "an intent had nobody to receive it");
        }
    }

    /// Consumes `code` if it is the live offer: mints the credential, writes
    /// the record (replacing any earlier phone) and only then spends the code.
    /// `None` for an unknown, used or expired code, or a record that could not
    /// be written — in which case the code stays live for a retry.
    pub(crate) fn enrol(&self, code: &str) -> Option<EnrolledPhone> {
        let mut state = self.state();
        let now = self.now_millis();
        if !state
            .enrolment
            .as_ref()
            .is_some_and(|offer| offer.is_live(code, now))
        {
            tracing::debug!("an enrolment request named no live code");
            return None;
        }
        let phone = EnrolledPhone {
            device_id: encode_hex(&random_bytes(DEVICE_ID_BYTES)),
            secret: random_bytes(SECRET_BYTES),
            enrolled_on: rfc3339_now(),
        };
        if let Err(error) = self.record.write(&phone) {
            tracing::error!(%error, "the phone record could not be written; the code stays live");
            return None;
        }
        state.enrolment = None;
        tracing::info!(device_id = %phone.device_id, "a phone was enrolled");
        Some(phone)
    }

    /// Whether `device_id` is the enrolled phone's, compared in constant time.
    pub(crate) fn is_enrolled_device(&self, device_id: &str) -> bool {
        self.enrolled_phone()
            .is_some_and(|phone| same_secret(phone.device_id.as_bytes(), device_id.as_bytes()))
    }

    /// The enrolled phone's secret, if one is enrolled.
    pub(crate) fn enrolled_secret(&self) -> Option<Vec<u8>> {
        self.enrolled_phone().map(|phone| phone.secret)
    }

    fn enrolled_phone(&self) -> Option<EnrolledPhone> {
        match self.record.read() {
            Ok(phone) => phone,
            Err(error) => {
                tracing::warn!(%error, "the phone record could not be read");
                None
            }
        }
    }

    /// Makes `outbox` the phone's socket, dismissing the previous one with
    /// `bye {replaced}`, and queues its `welcome` carrying the last published
    /// state under the same lock so no `state` can overtake it.
    pub(crate) fn register_socket(&self, outbox: Arc<Outbox>, welcome_proof: String) -> u64 {
        let mut state = self.state();
        if let Some(previous) = state.active.take() {
            tracing::info!(
                socket = previous.id,
                "a newer phone socket replaces the current one"
            );
            previous.dismiss(ByeReason::Replaced);
        }
        state.next_socket_id += 1;
        let id = state.next_socket_id;
        let welcome = ServerMessage::Welcome {
            proof: welcome_proof,
            state: WireState::from(state.last_published.as_ref().unwrap_or(&empty_state())),
        };
        outbox.push(websocket::frame_text(&welcome.to_json()));
        state.active = Some(ActiveSocket { id, outbox });
        id
    }

    /// The socket `id` ended. If it was still the phone's, the phone is gone:
    /// `Leave` goes out once if it was attached.
    pub(crate) fn socket_closed(&self, id: u64) {
        let mut state = self.state();
        if state.active.as_ref().is_none_or(|active| active.id != id) {
            return;
        }
        state.active = None;
        let change = state.presence.leave();
        drop(state);
        self.report(change);
    }

    /// Applies `update` to the presence at the current time and emits `Leave`
    /// if it reports the phone gone.
    pub(crate) fn update_presence(
        &self,
        update: impl FnOnce(&mut Presence, u64) -> PresenceChange,
    ) {
        let now = self.now_millis();
        let mut state = self.state();
        let change = update(&mut state.presence, now);
        drop(state);
        self.report(change);
    }

    fn report(&self, change: PresenceChange) {
        match change {
            PresenceChange::Gone => {
                tracing::info!("the phone is no longer attached");
                self.emit(RemoteIntent::Leave);
            }
            PresenceChange::Attached => tracing::info!("the phone is attached"),
            PresenceChange::Unchanged => {}
        }
    }

    /// Whether a frame published now would be sent: the phone is attached and
    /// the last published state has mobile mode on (`FR.3.3`, `FR.4.2`).
    pub(crate) fn wants_frames(&self) -> bool {
        let state = self.state();
        state.presence.is_attached()
            && state
                .last_published
                .as_ref()
                .is_some_and(|last| last.mobile_mode)
    }

    fn publish_state(&self, remote_state: &RemoteState) {
        let mut state = self.state();
        state.last_published = Some(remote_state.clone());
        if let Some(active) = &state.active {
            let message = ServerMessage::State {
                state: WireState::from(remote_state),
            };
            active
                .outbox
                .push(websocket::frame_text(&message.to_json()));
        }
    }

    fn publish_frame(&self, frame: Frame) {
        if !self.wants_frames() {
            return;
        }
        if let Some(active) = &self.state().active {
            active.outbox.offer_frame(frame);
        }
    }

    fn begin_enrolment(&self) -> EnrolmentOffer {
        let code = encode_hex(&random_bytes(ENROLMENT_CODE_BYTES));
        let address = format!("http://{}/enrol/{code}", self.bind);
        self.state().enrolment = Some(Enrolment {
            code,
            expires_at_millis: self.now_millis() + ENROLMENT_CODE_TTL_SECS * 1000,
        });
        EnrolmentOffer {
            address,
            expires_in_secs: ENROLMENT_CODE_TTL_SECS,
        }
    }

    /// Forgets the phone: clears the record, says `bye {revoked}` to its
    /// socket and closes it, and emits `Leave` once if a socket was open or
    /// the phone was attached (`FR.6.2`).
    fn revoke_phone(&self) {
        let mut state = self.state();
        if let Err(error) = self.record.clear() {
            tracing::error!(%error, "the phone record could not be cleared");
        }
        let had_socket = state
            .active
            .take()
            .inspect(|socket| socket.dismiss(ByeReason::Revoked));
        let was_attached = state.presence.leave() == PresenceChange::Gone;
        drop(state);
        tracing::info!("the phone was revoked");
        if had_socket.is_some() || was_attached {
            self.emit(RemoteIntent::Leave);
        }
    }

    fn phone_status(&self) -> PhoneStatus {
        match self.enrolled_phone() {
            Some(_) => PhoneStatus::Enrolled {
                attached: self.state().presence.is_attached(),
            },
            None => PhoneStatus::NotEnrolled,
        }
    }
}

/// The state `welcome` carries when the window has published none yet.
fn empty_state() -> RemoteState {
    RemoteState {
        mobile_mode: false,
        viewport: DEFAULT_MOBILE_VIEWPORT,
        current: None,
        workspaces: Vec::new(),
    }
}

/// `count` bytes from the system's random source.
pub(crate) fn random_bytes(count: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; count];
    rand::fill(&mut bytes[..]);
    bytes
}

/// Equality that takes the same time whether the inputs differ at the first
/// byte or the last, so a stranger cannot guess a code or an id byte by byte.
fn same_secret(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// The current moment as an RFC 3339 timestamp in UTC, for the record's
/// `enrolled_on`. Computed by hand: a date crate would be the only reason to
/// depend on one.
fn rfc3339_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    rfc3339_from_unix(seconds)
}

fn rfc3339_from_unix(seconds: u64) -> String {
    let days = i64::try_from(seconds / 86_400).unwrap_or(i64::MAX);
    let (year, month, day) = civil_from_days(days);
    let of_day = seconds % 86_400;
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

/// Howard Hinnant's `civil_from_days`: the proleptic Gregorian date of a day
/// count from 1970-01-01.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    (
        year,
        u32::try_from(month).unwrap_or(1),
        u32::try_from(day).unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_and_a_recent_moment_format_as_rfc_3339() {
        let stamps = [rfc3339_from_unix(0), rfc3339_from_unix(1_700_000_000)];

        assert_eq!(stamps, ["1970-01-01T00:00:00Z", "2023-11-14T22:13:20Z"]);
    }

    #[test]
    fn a_leap_day_is_placed_correctly() {
        let stamp = rfc3339_from_unix(1_709_164_800);

        assert_eq!(stamp, "2024-02-29T00:00:00Z");
    }

    #[test]
    fn same_secret_compares_whole_values_only() {
        let outcomes = [
            same_secret(b"abc", b"abc"),
            same_secret(b"abc", b"abd"),
            same_secret(b"abc", b"ab"),
        ];

        assert_eq!(outcomes, [true, false, false]);
    }

    #[test]
    fn random_bytes_are_the_asked_length_and_not_all_zero() {
        let bytes = random_bytes(32);

        assert!(bytes.len() == 32 && bytes.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn a_not_listening_link_reports_its_reason_and_offers_nothing() {
        let link = NotListeningLink::new("no mesh address");

        let status = link.phone_status();
        let offer = link.begin_enrolment();

        assert_eq!(
            (status, offer.address.as_str(), offer.expires_in_secs),
            (
                PhoneStatus::NotListening {
                    reason: "no mesh address".to_owned()
                },
                "",
                0
            )
        );
    }
}
