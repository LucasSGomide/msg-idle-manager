//! `GET /` and `GET /ws` answer only a request naming the enrolled phone —
//! by its cookie, or by the same device id in a `d` query, the address the
//! page gives itself for a browser with no cookie; every other path, method,
//! missing or mismatched id is an empty 404 with no `Server` header, so a
//! stranger learns nothing.

mod common;

use std::io::Write as _;
use std::net::TcpStream;

use common::{COOKIE_NAME, Server, WsClient, hex, http_get};
use idle_manager_core::PhoneLink as _;

fn is_empty_404(response: &common::Response) -> bool {
    response.status == 404
        && response.body.is_empty()
        && response.header("Content-Length") == Some("0")
        && response.header("Server").is_none()
}

#[test]
fn without_the_cookie_every_route_is_an_empty_404_with_no_server_header() {
    let server = Server::start();
    server.enrol();

    let answers: Vec<bool> = ["/", "/ws", "/anything"]
        .iter()
        .map(|path| is_empty_404(&http_get(server.addr(), path, &[])))
        .collect();

    assert_eq!(answers, [true, true, true]);
}

#[test]
fn a_mismatched_device_id_is_refused_the_same_way() {
    let server = Server::start();
    server.enrol();
    let wrong = format!("{COOKIE_NAME}={}", "f".repeat(32));

    let answers: Vec<bool> = ["/", "/ws", "/anything"]
        .iter()
        .map(|path| is_empty_404(&http_get(server.addr(), path, &[("Cookie", &wrong)])))
        .collect();

    assert_eq!(answers, [true, true, true]);
}

#[test]
fn before_any_enrolment_the_root_is_an_empty_404() {
    let server = Server::start();

    let response = http_get(server.addr(), "/", &[("Cookie", "idle-manager-phone=abc")]);

    assert!(is_empty_404(&response), "{}", response.head);
}

#[test]
fn the_root_with_the_right_cookie_serves_the_page_without_the_secret() {
    let server = Server::start();
    let credential = server.enrol();

    let response = http_get(server.addr(), "/", &[("Cookie", &credential.cookie())]);

    let page = response.body_text();
    assert!(
        response.status == 200
            && page.contains("<title>Idle Manager</title>")
            && page.contains(r#"<meta name="idle-manager-secret" content="">"#)
            && !page.contains(&hex(&credential.secret))
            && response.header("Set-Cookie").is_none(),
        "{}\n{page}",
        response.head
    );
}

#[test]
fn the_root_named_by_the_device_query_serves_the_page_and_hands_over_the_cookie() {
    let server = Server::start();
    let credential = server.enrol();

    let response = http_get(server.addr(), &format!("/?d={}", credential.device_id), &[]);

    let page = response.body_text();
    assert!(
        response.status == 200
            && page.contains("<title>Idle Manager</title>")
            && !page.contains(&hex(&credential.secret))
            && response.header("Set-Cookie") == Some(credential.set_cookie().as_str()),
        "{}\n{page}",
        response.head
    );
}

#[test]
fn the_root_named_by_a_wrong_device_query_is_an_empty_404() {
    let server = Server::start();
    server.enrol();

    let response = http_get(server.addr(), &format!("/?d={}", "f".repeat(32)), &[]);

    assert!(is_empty_404(&response), "{}", response.head);
}

#[test]
fn the_phone_address_is_the_root_with_the_device_query() {
    let server = Server::start();
    let before = server.handle.phone_address();
    let credential = server.enrol();

    let after = server.handle.phone_address();

    assert_eq!(
        (before, after),
        (
            None,
            Some(format!(
                "http://{}/?d={}",
                server.addr(),
                credential.device_id
            ))
        )
    );
}

#[test]
fn the_socket_upgrade_named_by_the_device_query_answers_101() {
    let server = Server::start();
    let credential = server.enrol();

    let connected = WsClient::connect_at(
        server.addr(),
        &format!("/ws?d={}", credential.device_id),
        None,
    );

    assert!(connected.is_ok());
}

#[test]
fn the_cookie_is_found_among_others_in_one_header() {
    let server = Server::start();
    let credential = server.enrol();
    let cookies = format!("theme=dark; {}; other=1", credential.cookie());

    let response = http_get(server.addr(), "/", &[("Cookie", &cookies)]);

    assert_eq!(response.status, 200);
}

#[test]
fn a_method_other_than_get_is_refused() {
    let server = Server::start();
    let credential = server.enrol();
    let mut stream = TcpStream::connect(server.addr()).expect("connects");
    stream
        .write_all(
            format!(
                "POST / HTTP/1.1\r\nHost: x\r\nCookie: {}\r\nContent-Length: 0\r\n\r\n",
                credential.cookie()
            )
            .as_bytes(),
        )
        .expect("writes");

    let mut raw = Vec::new();
    std::io::Read::read_to_end(&mut stream, &mut raw).expect("the server closes");

    assert!(
        raw.starts_with(b"HTTP/1.1 404 "),
        "{}",
        String::from_utf8_lossy(&raw)
    );
}

#[test]
fn a_request_that_is_not_http_is_answered_with_an_empty_404() {
    let server = Server::start();
    let mut stream = TcpStream::connect(server.addr()).expect("connects");
    stream.write_all(b"hello there\r\n\r\n").expect("writes");

    let mut raw = Vec::new();
    std::io::Read::read_to_end(&mut stream, &mut raw).expect("the server closes");

    assert!(
        raw.starts_with(b"HTTP/1.1 404 "),
        "{}",
        String::from_utf8_lossy(&raw)
    );
}

#[test]
fn the_socket_upgrade_without_the_cookie_is_an_empty_404() {
    let server = Server::start();
    server.enrol();

    let refused = WsClient::connect(server.addr(), None).expect_err("no upgrade");

    assert!(is_empty_404(&refused), "{}", refused.head);
}

#[test]
fn the_socket_upgrade_with_the_cookie_answers_101_with_the_rfc_accept_value() {
    let server = Server::start();
    let credential = server.enrol();

    let connected = WsClient::connect(server.addr(), Some(&credential.cookie()));

    assert!(connected.is_ok());
}
