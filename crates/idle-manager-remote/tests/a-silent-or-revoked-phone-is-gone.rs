//! A phone that stops sending `ping` for fifteen seconds is gone — one
//! `Leave`, decided by the clock the test drives — and revoking the phone
//! from the desktop says `bye {revoked}`, closes the socket, yields `Leave`
//! and leaves the status at not enrolled.

mod common;

use common::{Received, SILENCE, Server};
use idle_manager_core::{PhoneLink as _, PhoneStatus, RemoteIntent, SILENCE_LIMIT_SECS};
use serde_json::json;

#[test]
fn an_attached_phone_is_reported_attached() {
    let server = Server::start();
    let credential = server.enrol();

    let _phone = server.attached_phone(&credential);

    assert_eq!(
        server.handle.phone_status(),
        PhoneStatus::Enrolled { attached: true }
    );
}

#[test]
fn fifteen_seconds_without_a_ping_yield_one_leave() {
    let server = Server::start();
    let credential = server.enrol();
    let _phone = server.attached_phone(&credential);

    server.clock.advance_secs(SILENCE_LIMIT_SECS);

    let first = server.next_intent();
    server.clock.advance_secs(SILENCE_LIMIT_SECS);
    let then = server.intents_within_silence();
    assert_eq!(
        (first, then, server.handle.phone_status()),
        (
            Some(RemoteIntent::Leave),
            Vec::new(),
            PhoneStatus::Enrolled { attached: false }
        )
    );
}

#[test]
fn a_ping_within_the_limit_keeps_the_phone_attached() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = server.attached_phone(&credential);
    server.clock.advance_secs(SILENCE_LIMIT_SECS - 1);
    phone.send_json(&json!({"type": "ping"}));
    std::thread::sleep(SILENCE);

    server.clock.advance_secs(SILENCE_LIMIT_SECS - 1);

    assert_eq!(
        (
            server.intents_within_silence(),
            server.handle.phone_status()
        ),
        (Vec::new(), PhoneStatus::Enrolled { attached: true })
    );
}

#[test]
fn a_ping_from_the_phone_gets_no_reply() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = server.attached_phone(&credential);

    phone.send_json(&json!({"type": "ping"}));

    assert_eq!(phone.recv_within(SILENCE), Received::Quiet);
}

#[test]
fn closing_the_socket_while_attached_yields_one_leave() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = server.attached_phone(&credential);

    phone.send_close();
    drop(phone);

    let first = server.next_intent();
    assert_eq!(
        (first, server.intents_within_silence()),
        (Some(RemoteIntent::Leave), Vec::new())
    );
}

#[test]
fn revoking_the_phone_says_bye_revoked_and_closes_the_socket() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = server.attached_phone(&credential);

    server.handle.revoke_phone();

    assert_eq!(
        [phone.recv(), phone.recv()],
        [
            Received::Text(r#"{"type":"bye","reason":"revoked"}"#.to_owned()),
            Received::Close
        ]
    );
}

#[test]
fn revoking_the_phone_yields_one_leave_and_the_status_not_enrolled() {
    let server = Server::start();
    let credential = server.enrol();
    let _phone = server.attached_phone(&credential);

    server.handle.revoke_phone();

    let first = server.next_intent();
    assert_eq!(
        (
            first,
            server.intents_within_silence(),
            server.handle.phone_status()
        ),
        (
            Some(RemoteIntent::Leave),
            Vec::new(),
            PhoneStatus::NotEnrolled
        )
    );
}

#[test]
fn revoking_with_no_phone_connected_clears_the_record_without_a_leave() {
    let server = Server::start();
    server.enrol();

    server.handle.revoke_phone();

    assert_eq!(
        (server.intents_within_silence(), server.record.phone()),
        (Vec::new(), None)
    );
}

#[test]
fn after_revocation_the_old_cookie_is_refused() {
    let server = Server::start();
    let credential = server.enrol();
    server.handle.revoke_phone();

    let response = common::http_get(server.addr(), "/", &[("Cookie", &credential.cookie())]);

    assert_eq!(response.status, 404);
}
