//! RFC 6455 on a blocking `TcpStream`: the handshake's accept value, and the
//! framing and unframing of text, binary, ping, pong and close messages with
//! the client-side masking the RFC requires.
//!
//! Reads honour the stream's read timeout: a timeout before the first byte of
//! a frame is reported as [`Read::Quiet`] so the caller can run its heartbeat
//! check, while a timeout inside a frame is retried up to a bound.

use std::io::{self, Read as _, Write as _};
use std::net::TcpStream;

use base64::prelude::{BASE64_STANDARD, Engine as _};
use sha1::{Digest as _, Sha1};

/// The fixed string RFC 6455 appends to the client's key before hashing.
const ACCEPT_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// How many consecutive read timeouts inside one frame are tolerated before
/// the connection counts as stalled. With the 1 s read timeout the session
/// sets, this is 30 s.
const MAX_STALLED_READS: u32 = 30;

const OPCODE_CONTINUATION: u8 = 0x0;
const OPCODE_TEXT: u8 = 0x1;
const OPCODE_BINARY: u8 = 0x2;
const OPCODE_CLOSE: u8 = 0x8;
const OPCODE_PING: u8 = 0x9;
const OPCODE_PONG: u8 = 0xA;

/// Why reading from the socket stopped.
#[derive(Debug, thiserror::Error)]
pub(crate) enum WsError {
    /// The peer closed the TCP connection.
    #[error("the peer closed the connection")]
    Closed,
    /// The peer stopped sending in the middle of a frame.
    #[error("the peer stalled inside a frame")]
    Stalled,
    /// The socket failed.
    #[error("socket error: {0}")]
    Io(#[from] io::Error),
    /// A frame arrived without the mask every client frame must carry.
    #[error("the frame was not masked")]
    Unmasked,
    /// A frame's payload exceeds the caller's limit.
    #[error("a frame of {length} bytes exceeds the limit of {limit} bytes")]
    TooLarge { length: u64, limit: usize },
    /// A frame's opcode is none the RFC defines, or a continuation arrived
    /// with nothing to continue.
    #[error("frame opcode {opcode:#x} is not valid here")]
    BadOpcode { opcode: u8 },
    /// A text message is not UTF-8.
    #[error("a text message was not UTF-8")]
    NotUtf8,
}

/// One complete message from the peer, fragments already joined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Message {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close,
}

/// What one read produced.
#[derive(Debug)]
pub(crate) enum Read {
    /// A whole message.
    Message(Message),
    /// The read timeout passed with nothing on the wire.
    Quiet,
}

/// The `Sec-WebSocket-Accept` value for a client's `Sec-WebSocket-Key`:
/// base64 of SHA-1 of the key followed by the RFC's GUID.
pub(crate) fn accept_key(client_key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(client_key.trim().as_bytes());
    hasher.update(ACCEPT_GUID.as_bytes());
    BASE64_STANDARD.encode(hasher.finalize())
}

/// A text message as the server frames it: final, unmasked.
pub(crate) fn frame_text(text: &str) -> Vec<u8> {
    frame(OPCODE_TEXT, text.as_bytes())
}

/// A binary message as the server frames it.
pub(crate) fn frame_binary(payload: &[u8]) -> Vec<u8> {
    frame(OPCODE_BINARY, payload)
}

/// A pong answering a ping, echoing its payload.
pub(crate) fn frame_pong(payload: &[u8]) -> Vec<u8> {
    frame(OPCODE_PONG, payload)
}

/// A close frame with no status code.
pub(crate) fn frame_close() -> Vec<u8> {
    frame(OPCODE_CLOSE, &[])
}

/// Writes one already-framed message.
///
/// # Errors
///
/// The socket's own error.
pub(crate) fn send(stream: &mut TcpStream, framed: &[u8]) -> io::Result<()> {
    stream.write_all(framed)?;
    stream.flush()
}

/// Reads the next message, joining fragments and surfacing control frames as
/// they come.
///
/// # Errors
///
/// [`WsError::Closed`] at end of stream, [`WsError::Stalled`] when the peer
/// stops inside a frame, and the protocol violations listed on [`WsError`].
pub(crate) fn read_message(stream: &mut TcpStream, max_len: usize) -> Result<Read, WsError> {
    let mut pending: Option<(u8, Vec<u8>)> = None;
    loop {
        let Some(first) = read_first_byte(stream)? else {
            if pending.is_none() {
                return Ok(Read::Quiet);
            }
            continue;
        };
        let frame = read_frame_after(stream, first, max_len)?;
        match frame.opcode {
            OPCODE_PING => return Ok(Read::Message(Message::Ping(frame.payload))),
            OPCODE_PONG => return Ok(Read::Message(Message::Pong(frame.payload))),
            OPCODE_CLOSE => return Ok(Read::Message(Message::Close)),
            OPCODE_TEXT | OPCODE_BINARY if pending.is_none() => {
                pending = Some((frame.opcode, frame.payload));
            }
            OPCODE_CONTINUATION if pending.is_some() => {
                if let Some((_, buffer)) = pending.as_mut() {
                    buffer.extend_from_slice(&frame.payload);
                }
            }
            opcode => return Err(WsError::BadOpcode { opcode }),
        }
        if !frame.is_final {
            continue;
        }
        let Some((opcode, payload)) = pending.take() else {
            continue;
        };
        return Ok(Read::Message(if opcode == OPCODE_TEXT {
            Message::Text(String::from_utf8(payload).map_err(|_| WsError::NotUtf8)?)
        } else {
            Message::Binary(payload)
        }));
    }
}

struct Frame {
    is_final: bool,
    opcode: u8,
    payload: Vec<u8>,
}

/// The first byte of the next frame, or `None` when the read timeout passed
/// with nothing to read.
fn read_first_byte(stream: &mut TcpStream) -> Result<Option<u8>, WsError> {
    let mut first = [0u8; 1];
    loop {
        return match stream.read(&mut first) {
            Ok(0) => Err(WsError::Closed),
            Ok(_) => Ok(Some(first[0])),
            Err(error) if is_timeout(&error) => Ok(None),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => Err(WsError::Io(error)),
        };
    }
}

fn read_frame_after(stream: &mut TcpStream, first: u8, max_len: usize) -> Result<Frame, WsError> {
    let mut second = [0u8; 1];
    read_fully(stream, &mut second)?;
    let is_masked = second[0] & 0x80 != 0;
    if !is_masked {
        return Err(WsError::Unmasked);
    }
    let length = read_length(stream, second[0] & 0x7F)?;
    if length > max_len as u64 {
        return Err(WsError::TooLarge {
            length,
            limit: max_len,
        });
    }
    let mut mask = [0u8; 4];
    read_fully(stream, &mut mask)?;
    let mut payload = vec![
        0u8;
        usize::try_from(length).map_err(|_| WsError::TooLarge {
            length,
            limit: max_len
        })?
    ];
    read_fully(stream, &mut payload)?;
    for (index, byte) in payload.iter_mut().enumerate() {
        *byte ^= mask[index % 4];
    }
    Ok(Frame {
        is_final: first & 0x80 != 0,
        opcode: first & 0x0F,
        payload,
    })
}

fn read_length(stream: &mut TcpStream, short: u8) -> Result<u64, WsError> {
    match short {
        126 => {
            let mut bytes = [0u8; 2];
            read_fully(stream, &mut bytes)?;
            Ok(u64::from(u16::from_be_bytes(bytes)))
        }
        127 => {
            let mut bytes = [0u8; 8];
            read_fully(stream, &mut bytes)?;
            Ok(u64::from_be_bytes(bytes))
        }
        length => Ok(u64::from(length)),
    }
}

/// Fills `buffer` from the stream, riding out read timeouts inside a frame up
/// to [`MAX_STALLED_READS`].
fn read_fully(stream: &mut TcpStream, buffer: &mut [u8]) -> Result<(), WsError> {
    let mut filled = 0;
    let mut stalls = 0;
    while filled < buffer.len() {
        match stream.read(&mut buffer[filled..]) {
            Ok(0) => return Err(WsError::Closed),
            Ok(count) => filled += count,
            Err(error) if is_timeout(&error) => {
                stalls += 1;
                if stalls >= MAX_STALLED_READS {
                    return Err(WsError::Stalled);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(WsError::Io(error)),
        }
    }
    Ok(())
}

/// A read timeout is `WouldBlock` on Unix and `TimedOut` on Windows.
fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

fn frame(opcode: u8, payload: &[u8]) -> Vec<u8> {
    let mut framed = Vec::with_capacity(payload.len() + 10);
    framed.push(0x80 | opcode);
    let length = payload.len();
    match (u8::try_from(length), u16::try_from(length)) {
        (Ok(short), _) if short <= 125 => framed.push(short),
        (_, Ok(medium)) => {
            framed.push(126);
            framed.extend_from_slice(&medium.to_be_bytes());
        }
        _ => {
            framed.push(127);
            framed.extend_from_slice(&u64::try_from(length).unwrap_or(u64::MAX).to_be_bytes());
        }
    }
    framed.extend_from_slice(payload);
    framed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_accept_key_matches_the_rfc_6455_example() {
        let accept = accept_key("dGhlIHNhbXBsZSBub25jZQ==");

        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn a_short_text_frame_is_final_unmasked_with_a_one_byte_length() {
        let framed = frame_text("hi");

        assert_eq!(framed, vec![0x81, 2, b'h', b'i']);
    }

    #[test]
    fn a_payload_over_125_bytes_takes_the_two_byte_length() {
        let payload = vec![7u8; 300];

        let framed = frame_binary(&payload);

        assert_eq!(&framed[..4], &[0x82, 126, 0x01, 0x2C]);
    }

    #[test]
    fn a_payload_over_65535_bytes_takes_the_eight_byte_length() {
        let payload = vec![7u8; 70_000];

        let framed = frame_binary(&payload);

        assert_eq!(&framed[..10], &[0x82, 127, 0, 0, 0, 0, 0, 0x01, 0x11, 0x70]);
    }
}
