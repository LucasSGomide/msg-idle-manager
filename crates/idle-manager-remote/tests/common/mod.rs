//! A plain TCP client for the server's integration tests — an HTTP `GET`, a
//! WebSocket client with the RFC 6455 handshake and client-side masking, the
//! HMAC both sides compute — plus a phone record kept in memory and a clock
//! the test moves by hand, so the fifteen-second silence limit is crossed in
//! a call rather than waited for.

#![allow(dead_code)]

use std::fmt::Write as _;
use std::io::{self, Read as _, Write as _};
use std::net::{SocketAddr, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::prelude::{BASE64_STANDARD, Engine as _};
use hmac::{Hmac, KeyInit as _, Mac as _};
use idle_manager_core::{
    DEFAULT_MOBILE_VIEWPORT, EnrolledPhone, PhoneLink as _, PhoneRecordError, PhoneRecordStore,
    RemoteIntent, RemoteState,
};
use idle_manager_remote::{Clock, RemoteConfig, RemoteHandle, RemoteServer, bind_address};
use sha1::{Digest as _, Sha1};
use sha2::Sha256;

/// How long a test waits for something the server should send promptly.
pub(crate) const PROMPT: Duration = Duration::from_secs(3);

/// How long a test waits to be sure nothing is coming.
pub(crate) const SILENCE: Duration = Duration::from_millis(400);

/// The cookie the server sets and expects.
pub(crate) const COOKIE_NAME: &str = "idle-manager-phone";

/// The RFC 6455 example key, whose accept value is known.
const CLIENT_KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";
const ACCEPT_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// The phone's own challenge in `auth`: 32 bytes, hex.
pub(crate) const PHONE_CHALLENGE: &str =
    "abababababababababababababababababababababababababababababababab";

/// The one enrolled phone, kept in memory.
#[derive(Debug, Default)]
pub(crate) struct MemoryPhoneRecord {
    phone: Mutex<Option<EnrolledPhone>>,
}

impl MemoryPhoneRecord {
    pub(crate) fn phone(&self) -> Option<EnrolledPhone> {
        self.phone
            .lock()
            .expect("the record lock is not poisoned")
            .clone()
    }
}

impl PhoneRecordStore for MemoryPhoneRecord {
    fn read(&self) -> Result<Option<EnrolledPhone>, PhoneRecordError> {
        Ok(self.phone())
    }

    fn write(&self, phone: &EnrolledPhone) -> Result<(), PhoneRecordError> {
        *self.phone.lock().expect("the record lock is not poisoned") = Some(phone.clone());
        Ok(())
    }

    fn clear(&self) -> Result<(), PhoneRecordError> {
        *self.phone.lock().expect("the record lock is not poisoned") = None;
        Ok(())
    }
}

/// A clock that moves only when the test says so.
#[derive(Debug, Default)]
pub(crate) struct ManualClock {
    millis: AtomicU64,
}

impl ManualClock {
    pub(crate) fn advance_secs(&self, secs: u64) {
        self.millis.fetch_add(secs * 1000, Ordering::SeqCst);
    }
}

impl Clock for ManualClock {
    fn now_millis(&self) -> u64 {
        self.millis.load(Ordering::SeqCst)
    }
}

/// What `GET /enrol/{code}` handed the phone.
#[derive(Debug, Clone)]
pub(crate) struct Credential {
    pub(crate) device_id: String,
    pub(crate) secret: Vec<u8>,
}

impl Credential {
    /// The `Cookie` header value the phone sends back.
    pub(crate) fn cookie(&self) -> String {
        format!("{COOKIE_NAME}={}", self.device_id)
    }

    /// The `Set-Cookie` value enrolment answers with, and the root answers
    /// with again when named by the device query.
    pub(crate) fn set_cookie(&self) -> String {
        format!(
            "{COOKIE_NAME}={}; Path=/; Max-Age=31536000; SameSite=Strict; HttpOnly",
            self.device_id
        )
    }
}

/// A running server on a loopback port of the system's choosing.
#[derive(Debug)]
pub(crate) struct Server {
    pub(crate) handle: RemoteHandle,
    pub(crate) intents: async_channel::Receiver<RemoteIntent>,
    pub(crate) record: Arc<MemoryPhoneRecord>,
    pub(crate) clock: Arc<ManualClock>,
}

impl Server {
    pub(crate) fn start() -> Self {
        let record = Arc::new(MemoryPhoneRecord::default());
        let clock = Arc::new(ManualClock::default());
        let bind = bind_address(Some("127.0.0.1:0")).expect("the loopback override parses");
        let config = RemoteConfig {
            bind,
            clock: Arc::clone(&clock) as Arc<dyn Clock>,
        };
        let (handle, intents) =
            RemoteServer::start(config, Arc::clone(&record) as Arc<dyn PhoneRecordStore>)
                .expect("the server binds a loopback port");
        Self {
            handle,
            intents,
            record,
            clock,
        }
    }

    pub(crate) fn addr(&self) -> SocketAddr {
        self.handle.local_addr()
    }

    /// Mints an offer and returns the path of its code.
    pub(crate) fn enrolment_path(&self) -> String {
        let offer = self.handle.begin_enrolment();
        let code = offer
            .address
            .rsplit('/')
            .next()
            .expect("the address ends in the code");
        format!("/enrol/{code}")
    }

    /// Enrols a phone through the real route and returns what the page gave it.
    pub(crate) fn enrol(&self) -> Credential {
        let response = http_get(self.addr(), &self.enrolment_path(), &[]);
        assert_eq!(
            response.status, 200,
            "enrolment answered:\n{}",
            response.head
        );
        credential_from(&response.body_text())
    }

    /// The next intent, waiting up to [`PROMPT`].
    pub(crate) fn next_intent(&self) -> Option<RemoteIntent> {
        let deadline = Instant::now() + PROMPT;
        loop {
            match self.intents.try_recv() {
                Ok(intent) => return Some(intent),
                Err(async_channel::TryRecvError::Empty) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(_) => return None,
            }
        }
    }

    /// Whatever arrives on the channel within [`SILENCE`]; empty when nothing did.
    pub(crate) fn intents_within_silence(&self) -> Vec<RemoteIntent> {
        std::thread::sleep(SILENCE);
        let mut intents = Vec::new();
        while let Ok(intent) = self.intents.try_recv() {
            intents.push(intent);
        }
        intents
    }

    /// Connects, proves the secret and attaches, leaving the phone watching.
    pub(crate) fn attached_phone(&self, credential: &Credential) -> WsClient {
        let mut phone = WsClient::connect(self.addr(), Some(&credential.cookie()))
            .expect("the enrolled phone's socket upgrades");
        phone.authenticate(&credential.secret);
        phone.send_json(&serde_json::json!({
            "type": "attach",
            "viewport": {"width": 412, "height": 915}
        }));
        assert_eq!(
            self.next_intent(),
            Some(RemoteIntent::Attach {
                viewport: DEFAULT_MOBILE_VIEWPORT
            })
        );
        phone
    }
}

/// A state with mobile mode on and no accounts, the least that lets frames flow.
pub(crate) fn mobile_state() -> RemoteState {
    RemoteState {
        mobile_mode: true,
        viewport: DEFAULT_MOBILE_VIEWPORT,
        current: None,
        workspaces: Vec::new(),
    }
}

/// One HTTP response, head and body.
#[derive(Debug)]
pub(crate) struct Response {
    pub(crate) status: u16,
    pub(crate) head: String,
    pub(crate) body: Vec<u8>,
}

impl Response {
    /// The first header named `name`, case-insensitively.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.head
            .split("\r\n")
            .skip(1)
            .filter_map(|line| line.split_once(':'))
            .find(|(candidate, _)| candidate.trim().eq_ignore_ascii_case(name))
            .map(|(_, value)| value.trim())
    }

    pub(crate) fn body_text(&self) -> String {
        String::from_utf8(self.body.clone()).expect("the body is UTF-8")
    }
}

/// `GET path` with `extra` headers, read to the server's close.
pub(crate) fn http_get(addr: SocketAddr, path: &str, extra: &[(&str, &str)]) -> Response {
    let mut stream = TcpStream::connect(addr).expect("the server accepts a connection");
    stream
        .set_read_timeout(Some(PROMPT))
        .expect("the read timeout is set");
    let mut request = format!("GET {path} HTTP/1.1\r\nHost: {addr}\r\nConnection: close\r\n");
    for (name, value) in extra {
        write!(request, "{name}: {value}\r\n").expect("writing to a String cannot fail");
    }
    request.push_str("\r\n");
    stream
        .write_all(request.as_bytes())
        .expect("the request is written");
    let mut raw = Vec::new();
    stream
        .read_to_end(&mut raw)
        .expect("the server closes after answering");
    parse_response(&raw)
}

fn parse_response(raw: &[u8]) -> Response {
    let end = raw
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .expect("the response has a head");
    let head = String::from_utf8(raw[..end].to_vec()).expect("the head is ASCII");
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .expect("the status line carries a code");
    Response {
        status,
        head,
        body: raw[end + 4..].to_vec(),
    }
}

/// The `content` of `<meta name="{name}">` in `page`.
pub(crate) fn meta_content(page: &str, name: &str) -> Option<String> {
    let start = page.find(&format!(r#"<meta name="{name}" content=""#))?;
    let rest = &page[start..];
    let value_start = rest.find("content=\"")? + "content=\"".len();
    let value = &rest[value_start..];
    let value_end = value.find('"')?;
    Some(value[..value_end].to_owned())
}

/// The credential the enrolment page carries in its two meta tags.
pub(crate) fn credential_from(page: &str) -> Credential {
    let device_id =
        meta_content(page, "idle-manager-device-id").expect("the device id tag is there");
    let secret = meta_content(page, "idle-manager-secret").expect("the secret tag is there");
    Credential {
        device_id,
        secret: unhex(&secret),
    }
}

/// Lowercase hex of `bytes`.
pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, byte| {
        write!(out, "{byte:02x}").expect("writing to a String cannot fail");
        out
    })
}

/// The bytes `text` spells in hex.
pub(crate) fn unhex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&text[at..at + 2], 16).expect("hex digits"))
        .collect()
}

/// Hex HMAC-SHA256 of `message` keyed by `secret`.
pub(crate) fn hmac_hex(secret: &[u8], message: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).expect("HMAC takes any key length");
    mac.update(message.as_bytes());
    hex(&mac.finalize().into_bytes())
}

/// What one read from the socket produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Received {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    /// The server's close frame.
    Close,
    /// The TCP connection ended.
    Closed,
    /// Nothing arrived within the wait.
    Quiet,
}

/// A WebSocket client over one `TcpStream`, masking as RFC 6455 requires.
#[derive(Debug)]
pub(crate) struct WsClient {
    stream: TcpStream,
}

impl WsClient {
    /// Opens `/ws` with `cookie`. `Err` carries the HTTP answer when the
    /// server did not upgrade; a 101 is checked for the right accept value.
    pub(crate) fn connect(addr: SocketAddr, cookie: Option<&str>) -> Result<Self, Response> {
        Self::connect_at(addr, "/ws", cookie)
    }

    /// [`WsClient::connect`] against `target` — `/ws` with a query, for the
    /// device id the page puts there in place of the cookie.
    pub(crate) fn connect_at(
        addr: SocketAddr,
        target: &str,
        cookie: Option<&str>,
    ) -> Result<Self, Response> {
        let mut stream = TcpStream::connect(addr).expect("the server accepts a connection");
        stream
            .set_read_timeout(Some(PROMPT))
            .expect("the read timeout is set");
        let mut request = format!(
            "GET {target} HTTP/1.1\r\nHost: {addr}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: {CLIENT_KEY}\r\nSec-WebSocket-Version: 13\r\n"
        );
        if let Some(cookie) = cookie {
            write!(request, "Cookie: {cookie}\r\n").expect("writing to a String cannot fail");
        }
        request.push_str("\r\n");
        stream
            .write_all(request.as_bytes())
            .expect("the request is written");
        let head = read_head(&mut stream);
        let response = parse_response(&head);
        if response.status != 101 {
            let mut rest = Vec::new();
            let mut stream = stream;
            stream.read_to_end(&mut rest).ok();
            return Err(Response {
                body: rest,
                ..response
            });
        }
        let mut hasher = Sha1::new();
        hasher.update(CLIENT_KEY.as_bytes());
        hasher.update(ACCEPT_GUID.as_bytes());
        let expected = BASE64_STANDARD.encode(hasher.finalize());
        assert_eq!(
            response.header("Sec-WebSocket-Accept"),
            Some(expected.as_str()),
            "the accept value is SHA-1 of key and GUID"
        );
        Ok(Self { stream })
    }

    pub(crate) fn send_text(&mut self, text: &str) {
        self.send_frame(0x1, text.as_bytes());
    }

    pub(crate) fn send_json(&mut self, value: &serde_json::Value) {
        self.send_text(&value.to_string());
    }

    pub(crate) fn send_ping(&mut self, payload: &[u8]) {
        self.send_frame(0x9, payload);
    }

    pub(crate) fn send_close(&mut self) {
        self.send_frame(0x8, &[]);
    }

    fn send_frame(&mut self, opcode: u8, payload: &[u8]) {
        let mask = [0x12, 0x34, 0x56, 0x78];
        let mut framed = vec![0x80 | opcode];
        match payload.len() {
            length if length < 126 => framed.push(0x80 | u8::try_from(length).expect("< 126")),
            length if length <= 0xFFFF => {
                framed.push(0x80 | 0x7E);
                framed.extend_from_slice(&u16::try_from(length).expect("<= 0xFFFF").to_be_bytes());
            }
            length => {
                framed.push(0x80 | 0x7F);
                framed.extend_from_slice(&(length as u64).to_be_bytes());
            }
        }
        framed.extend_from_slice(&mask);
        framed.extend(
            payload
                .iter()
                .enumerate()
                .map(|(index, byte)| byte ^ mask[index % 4]),
        );
        self.stream
            .write_all(&framed)
            .expect("the frame is written");
    }

    /// The next message, waiting up to [`PROMPT`].
    pub(crate) fn recv(&mut self) -> Received {
        self.recv_within(PROMPT)
    }

    /// The next message, or [`Received::Quiet`] after `wait`.
    pub(crate) fn recv_within(&mut self, wait: Duration) -> Received {
        self.stream
            .set_read_timeout(Some(wait))
            .expect("the read timeout is set");
        let mut first = [0u8; 2];
        match self.stream.read(&mut first[..1]) {
            Ok(0) => return Received::Closed,
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
                ) =>
            {
                return Received::Quiet;
            }
            Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {
                return Received::Closed;
            }
            Err(error) => panic!("reading the socket failed: {error}"),
        }
        if self.stream.read_exact(&mut first[1..]).is_err() {
            return Received::Closed;
        }
        assert_eq!(first[1] & 0x80, 0, "server frames are not masked");
        let length = match first[1] & 0x7F {
            126 => {
                let mut bytes = [0u8; 2];
                self.stream.read_exact(&mut bytes).expect("the length");
                usize::from(u16::from_be_bytes(bytes))
            }
            127 => {
                let mut bytes = [0u8; 8];
                self.stream.read_exact(&mut bytes).expect("the length");
                usize::try_from(u64::from_be_bytes(bytes)).expect("fits")
            }
            short => usize::from(short),
        };
        let mut payload = vec![0u8; length];
        self.stream
            .read_exact(&mut payload)
            .expect("the payload arrives whole");
        match first[0] & 0x0F {
            0x1 => Received::Text(String::from_utf8(payload).expect("text is UTF-8")),
            0x2 => Received::Binary(payload),
            0x8 => Received::Close,
            0x9 => Received::Ping(payload),
            0xA => Received::Pong(payload),
            opcode => panic!("unexpected opcode {opcode:#x}"),
        }
    }

    /// The next message as JSON; panics on anything but text.
    pub(crate) fn recv_json(&mut self) -> serde_json::Value {
        match self.recv() {
            Received::Text(text) => serde_json::from_str(&text).expect("the text is JSON"),
            other => panic!("expected a text message, got {other:?}"),
        }
    }

    /// Reads `hello`, answers `auth` with the right proof and returns the
    /// `welcome`.
    pub(crate) fn authenticate(&mut self, secret: &[u8]) -> serde_json::Value {
        let hello = self.recv_json();
        assert_eq!(hello["type"], "hello", "the first message is hello");
        let challenge = hello["challenge"]
            .as_str()
            .expect("hello carries a challenge");
        self.send_json(&serde_json::json!({
            "type": "auth",
            "proof": hmac_hex(secret, &format!("phone|{challenge}")),
            "challenge": PHONE_CHALLENGE,
        }));
        let welcome = self.recv_json();
        assert_eq!(welcome["type"], "welcome", "a right proof is welcomed");
        welcome
    }
}

/// Reads the response head, byte by byte so nothing after it is swallowed.
fn read_head(stream: &mut TcpStream) -> Vec<u8> {
    let mut head = Vec::new();
    let mut byte = [0u8; 1];
    while !head.ends_with(b"\r\n\r\n") {
        let count = stream.read(&mut byte).expect("the head arrives");
        assert_ne!(count, 0, "the server closed before finishing its head");
        head.push(byte[0]);
    }
    head
}
