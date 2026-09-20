//! One HTTP/1.1 request per connection, answered from exactly three routes —
//! `GET /enrol/{code}`, `GET /` and `GET /ws` as `openapi.json` describes
//! them. Everything else, every request without the enrolled phone's cookie
//! and every failure is a `404` with an empty body and no `Server` header, so
//! a stranger on the network learns nothing (`FR.6.1`).

use std::io::{self, Read as _, Write as _};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use idle_manager_core::EnrolledPhone;

use crate::protocol::encode_hex;
use crate::{Shared, session, websocket};

/// How long a connection may take to send its request headers before it is
/// dropped, so a stalled stranger does not pin a thread.
const REQUEST_TIMEOUT_SECS: u64 = 5;

/// The most header bytes one request may carry.
const MAX_REQUEST_BYTES: usize = 16 * 1024;

/// The cookie that names the enrolled phone.
pub(crate) const COOKIE_NAME: &str = "idle-manager-phone";

/// The query parameter that names the enrolled phone when the cookie is not
/// there: `/?d=<device id>`, the address the page gives itself after
/// enrolment.
pub(crate) const DEVICE_QUERY: &str = "d";

/// How long the phone keeps its cookie: one year.
const COOKIE_MAX_AGE_SECS: u64 = 31_536_000;

/// The phone page, with placeholders for the two credential tags.
const PAGE: &str = include_str!("../assets/phone.html");
const DEVICE_ID_PLACEHOLDER: &str = "{{idle-manager-device-id}}";
const SECRET_PLACEHOLDER: &str = "{{idle-manager-secret}}";

/// Why a request could not be read.
#[derive(Debug, thiserror::Error)]
pub(crate) enum HttpError {
    /// The socket failed or timed out before the headers ended.
    #[error("the request could not be read: {0}")]
    Io(#[from] io::Error),
    /// The peer closed before the headers ended.
    #[error("the peer closed before finishing its request")]
    Closed,
    /// The headers ran past [`MAX_REQUEST_BYTES`].
    #[error("the request headers exceed {MAX_REQUEST_BYTES} bytes")]
    TooLarge,
    /// The request line or a header is not HTTP/1.1.
    #[error("the request is not HTTP/1.1: {reason}")]
    Malformed {
        /// What was wrong.
        reason: &'static str,
    },
}

/// The parts of a request the three routes look at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Request {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
}

impl Request {
    /// Reads one request's head from the stream.
    ///
    /// # Errors
    ///
    /// [`HttpError`] for a socket failure, a peer that stops early, an
    /// oversized head or a malformed request line.
    pub(crate) fn read(stream: &mut TcpStream) -> Result<Self, HttpError> {
        let mut head = Vec::with_capacity(1024);
        let mut chunk = [0u8; 1024];
        loop {
            let count = stream.read(&mut chunk)?;
            if count == 0 {
                return Err(HttpError::Closed);
            }
            head.extend_from_slice(&chunk[..count]);
            if let Some(end) = head.windows(4).position(|window| window == b"\r\n\r\n") {
                let text = std::str::from_utf8(&head[..end]).map_err(|_| HttpError::Malformed {
                    reason: "the head is not UTF-8",
                })?;
                return Self::parse(text);
            }
            if head.len() > MAX_REQUEST_BYTES {
                return Err(HttpError::TooLarge);
            }
        }
    }

    /// Parses a request head: the request line then `Name: value` lines,
    /// without the blank line that ends it.
    ///
    /// # Errors
    ///
    /// [`HttpError::Malformed`] when the request line has not three parts or a
    /// header line has no colon.
    pub(crate) fn parse(head: &str) -> Result<Self, HttpError> {
        let mut lines = head.split("\r\n");
        let request_line = lines.next().ok_or(HttpError::Malformed {
            reason: "no request line",
        })?;
        let mut parts = request_line.split(' ');
        let (Some(method), Some(target), Some(version), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(HttpError::Malformed {
                reason: "the request line has not three parts",
            });
        };
        if !version.starts_with("HTTP/1.") {
            return Err(HttpError::Malformed {
                reason: "not an HTTP/1.x request",
            });
        }
        let headers = lines
            .filter(|line| !line.is_empty())
            .map(|line| {
                line.split_once(':')
                    .map(|(name, value)| (name.trim().to_owned(), value.trim().to_owned()))
                    .ok_or(HttpError::Malformed {
                        reason: "a header line has no colon",
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            method: method.to_owned(),
            target: target.to_owned(),
            headers,
        })
    }

    pub(crate) fn method(&self) -> &str {
        &self.method
    }

    /// The path without any query string.
    pub(crate) fn path(&self) -> &str {
        self.target
            .split_once('?')
            .map_or(self.target.as_str(), |(path, _)| path)
    }

    /// The value of query parameter `name`, from the first `name=value`
    /// pair naming it. No percent-decoding: the one parameter served here
    /// is a hex device id.
    pub(crate) fn query(&self, name: &str) -> Option<&str> {
        self.target
            .split_once('?')
            .into_iter()
            .flat_map(|(_, query)| query.split('&'))
            .filter_map(|pair| pair.split_once('='))
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, value)| value)
    }

    /// The first header named `name`, case-insensitively.
    pub(crate) fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
            .map(|(_, value)| value.as_str())
    }

    /// Whether the comma-separated header `name` lists `token`, as
    /// `Connection: keep-alive, Upgrade` does.
    pub(crate) fn header_has_token(&self, name: &str, token: &str) -> bool {
        self.headers
            .iter()
            .filter(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
            .flat_map(|(_, value)| value.split(','))
            .any(|candidate| candidate.trim().eq_ignore_ascii_case(token))
    }

    /// The value of cookie `name`, from any `Cookie` header and any position
    /// in it.
    pub(crate) fn cookie(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .filter(|(candidate, _)| candidate.eq_ignore_ascii_case("cookie"))
            .flat_map(|(_, value)| value.split(';'))
            .filter_map(|pair| pair.trim().split_once('='))
            .find(|(candidate, _)| *candidate == name)
            .map(|(_, value)| value.trim())
    }
}

/// Serves one connection: reads its request, answers it, and for `/ws` hands
/// the upgraded stream to the session. Runs on the connection's own thread.
pub(crate) fn serve(mut stream: TcpStream, shared: &Arc<Shared>) {
    if let Err(error) = stream.set_read_timeout(Some(Duration::from_secs(REQUEST_TIMEOUT_SECS))) {
        tracing::warn!(%error, "could not set the request timeout");
        return;
    }
    let request = match Request::read(&mut stream) {
        Ok(request) => request,
        Err(error) => {
            tracing::debug!(%error, "dropping a connection whose request did not read");
            respond(&mut stream, &not_found());
            return;
        }
    };
    match route(&request, shared) {
        Route::Respond(response) => respond(&mut stream, &response),
        Route::Upgrade { accept } => {
            if stream.write_all(&switching_protocols(&accept)).is_ok() {
                session::run(stream, shared);
            }
        }
    }
}

enum Route {
    Respond(Vec<u8>),
    Upgrade { accept: String },
}

fn route(request: &Request, shared: &Shared) -> Route {
    if request.method() != "GET" {
        return Route::Respond(not_found());
    }
    let path = request.path();
    if let Some(code) = path.strip_prefix("/enrol/") {
        return Route::Respond(enrol(code, shared));
    }
    // The phone names itself by cookie, or by the `d` query the page puts in
    // its own address after enrolment — the same id, for a browser that
    // dropped the cookie or keeps a jar of its own (a home-screen web app).
    // Either way it is only a name: the socket still demands the secret's
    // proof, and an id that is not the enrolled phone's is a 404 like any
    // other unknown request.
    let by_cookie = request.cookie(COOKIE_NAME);
    let by_query = request.query(DEVICE_QUERY);
    let is_enrolled_phone = by_cookie
        .or(by_query)
        .is_some_and(|device_id| shared.is_enrolled_device(device_id));
    match path {
        "/" if is_enrolled_phone => {
            // Named by the query alone: hand this jar the cookie too, so the
            // socket upgrade that follows carries it.
            let cookie = by_cookie
                .is_none()
                .then(|| by_query.map(device_cookie))
                .flatten();
            Route::Respond(ok_html(&render_page(None), cookie.as_deref()))
        }
        "/ws" if is_enrolled_phone => upgrade(request),
        _ => Route::Respond(not_found()),
    }
}

/// The `Set-Cookie` value that names `device_id` as this phone.
fn device_cookie(device_id: &str) -> String {
    format!(
        "{COOKIE_NAME}={device_id}; Path=/; Max-Age={COOKIE_MAX_AGE_SECS}; SameSite=Strict; HttpOnly"
    )
}

/// The page's address for the enrolled phone: the listening address with
/// the device id in the query, so it opens without the cookie.
pub(crate) fn page_address(bind: &SocketAddr, device_id: &str) -> String {
    format!("http://{bind}/?{DEVICE_QUERY}={device_id}")
}

fn enrol(code: &str, shared: &Shared) -> Vec<u8> {
    let Some(phone) = shared.enrol(code) else {
        return not_found();
    };
    let cookie = device_cookie(&phone.device_id);
    ok_html(&render_page(Some(&phone)), Some(&cookie))
}

fn upgrade(request: &Request) -> Route {
    let is_websocket = request.header_has_token("Upgrade", "websocket")
        && request.header("Sec-WebSocket-Version") == Some("13");
    match request.header("Sec-WebSocket-Key") {
        Some(key) if is_websocket => Route::Upgrade {
            accept: websocket::accept_key(key),
        },
        _ => Route::Respond(not_found()),
    }
}

/// The phone page with the credential tags filled for a fresh enrolment, or
/// emptied for the page served later.
fn render_page(credential: Option<&EnrolledPhone>) -> String {
    let (device_id, secret) = credential.map_or((String::new(), String::new()), |phone| {
        (phone.device_id.clone(), encode_hex(&phone.secret))
    });
    PAGE.replace(DEVICE_ID_PLACEHOLDER, &device_id)
        .replace(SECRET_PLACEHOLDER, &secret)
}

fn respond(stream: &mut TcpStream, response: &[u8]) {
    if let Err(error) = stream.write_all(response).and_then(|()| stream.flush()) {
        tracing::debug!(%error, "the response could not be written");
    }
    if let Err(error) = stream.shutdown(Shutdown::Both) {
        tracing::debug!(%error, "the connection did not shut down cleanly");
    }
}

/// The empty answer every stranger gets: no body, no `Server` header.
fn not_found() -> Vec<u8> {
    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_vec()
}

fn ok_html(body: &str, set_cookie: Option<&str>) -> Vec<u8> {
    let mut response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(cookie) = set_cookie {
        response.push_str("Set-Cookie: ");
        response.push_str(cookie);
        response.push_str("\r\n");
    }
    response.push_str("\r\n");
    response.push_str(body);
    response.into_bytes()
}

fn switching_protocols(accept: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n"
    )
    .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_request_line_and_headers_are_parsed_case_insensitively() {
        let request = Request::parse(
            "GET /ws?x=1 HTTP/1.1\r\nHost: 100.64.0.1:7466\r\nupgrade: WebSocket\r\nConnection: keep-alive, Upgrade",
        )
        .expect("parses");

        assert_eq!(
            (
                request.method(),
                request.path(),
                request.header("HOST"),
                request.header_has_token("Upgrade", "websocket"),
                request.header_has_token("connection", "upgrade"),
            ),
            ("GET", "/ws", Some("100.64.0.1:7466"), true, true)
        );
    }

    #[test]
    fn the_phone_cookie_is_found_among_several_in_one_header() {
        let request = Request::parse(
            "GET / HTTP/1.1\r\nCookie: theme=dark; idle-manager-phone=abc123; other=1",
        )
        .expect("parses");

        assert_eq!(request.cookie(COOKIE_NAME), Some("abc123"));
    }

    #[test]
    fn the_device_query_is_read_from_the_target_and_only_by_its_own_name() {
        let request =
            Request::parse("GET /?x=1&d=abc123&dd=zzz HTTP/1.1\r\nHost: h").expect("parses");

        assert_eq!(
            (
                request.query(DEVICE_QUERY),
                request.query("dd"),
                request.query("q")
            ),
            (Some("abc123"), Some("zzz"), None)
        );
    }

    #[test]
    fn the_page_address_carries_the_device_id_in_the_query() {
        let bind: SocketAddr = "100.101.12.7:7466".parse().expect("an address");

        let address = page_address(&bind, "abc123");

        assert_eq!(address, "http://100.101.12.7:7466/?d=abc123");
    }

    #[test]
    fn a_cookie_of_another_name_does_not_match_by_suffix() {
        let request = Request::parse("GET / HTTP/1.1\r\nCookie: x-idle-manager-phone=abc123")
            .expect("parses");

        assert_eq!(request.cookie(COOKIE_NAME), None);
    }

    #[test]
    fn a_request_line_without_three_parts_is_malformed() {
        let error = Request::parse("GET /").expect_err("malformed");

        assert!(matches!(error, HttpError::Malformed { .. }));
    }

    #[test]
    fn the_served_page_carries_empty_credential_tags_and_the_title() {
        let page = render_page(None);

        assert!(
            page.contains(r#"<meta name="idle-manager-device-id" content="">"#)
                && page.contains(r#"<meta name="idle-manager-secret" content="">"#)
                && page.contains("<title>Idle Manager</title>")
                && !page.contains("{{"),
            "{page}"
        );
    }

    #[test]
    fn the_enrolment_page_carries_the_device_id_and_the_secret_in_hex() {
        let phone = EnrolledPhone {
            device_id: "dev1".to_owned(),
            secret: vec![0xAB, 0xCD],
            enrolled_on: String::new(),
        };

        let page = render_page(Some(&phone));

        assert!(
            page.contains(r#"<meta name="idle-manager-device-id" content="dev1">"#)
                && page.contains(r#"<meta name="idle-manager-secret" content="abcd">"#),
            "{page}"
        );
    }

    #[test]
    fn the_page_served_without_a_credential_carries_none_of_the_minted_values() {
        let phone = EnrolledPhone {
            device_id: "0123456789abcdef0123456789abcdef".to_owned(),
            secret: (0x40..0x60).collect(),
            enrolled_on: String::new(),
        };
        let secret_hex = encode_hex(&phone.secret);
        let enrolment_page = render_page(Some(&phone));

        let page = render_page(None);

        assert!(
            enrolment_page.contains(&secret_hex)
                && enrolment_page.contains(&phone.device_id)
                && !page.contains(&secret_hex)
                && !page.contains(&phone.device_id),
            "{page}"
        );
    }

    #[test]
    fn the_page_names_the_socket_path_and_the_two_placeholders_the_server_fills() {
        let names = (
            PAGE.contains("'/ws'"),
            PAGE.contains(DEVICE_ID_PLACEHOLDER),
            PAGE.contains(SECRET_PLACEHOLDER),
        );

        assert_eq!(names, (true, true, true));
    }

    #[test]
    fn the_not_found_answer_has_no_body_and_no_server_header() {
        let response = String::from_utf8(not_found()).expect("ascii");

        assert!(
            response.starts_with("HTTP/1.1 404 ")
                && response.contains("Content-Length: 0\r\n")
                && !response.to_ascii_lowercase().contains("server:")
                && response.ends_with("\r\n\r\n"),
            "{response}"
        );
    }
}
