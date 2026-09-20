//! On `/ws` the server says `hello {challenge}` and nothing else until the
//! phone proves the secret: a wrong proof closes the socket silently, a right
//! one is answered with `welcome` carrying the desktop's own proof and the
//! last published state, and a second right one replaces the first.

mod common;

use common::{PHONE_CHALLENGE, Received, Server, WsClient, hmac_hex};
use idle_manager_core::{
    Liveness, PhoneLink as _, RemoteAccount, RemoteState, RemoteWorkspace, SessionId, Viewport,
};
use serde_json::json;

#[test]
fn the_first_message_is_hello_with_a_64_hex_challenge() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");

    let hello = phone.recv_json();

    let challenge = hello["challenge"].as_str().unwrap_or_default();
    assert!(
        hello["type"] == "hello"
            && challenge.len() == 64
            && challenge.chars().all(|c| c.is_ascii_hexdigit()),
        "{hello}"
    );
}

#[test]
fn a_wrong_proof_closes_the_socket_with_no_message() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");
    let hello = phone.recv_json();
    let challenge = hello["challenge"].as_str().unwrap_or_default();

    phone.send_json(&json!({
        "type": "auth",
        "proof": hmac_hex(b"not the secret", &format!("phone|{challenge}")),
        "challenge": PHONE_CHALLENGE,
    }));

    assert_eq!(phone.recv(), Received::Closed);
}

#[test]
fn a_right_proof_receives_a_welcome_whose_proof_verifies_against_the_phones_challenge() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");

    let welcome = phone.authenticate(&credential.secret);

    assert_eq!(
        welcome["proof"],
        hmac_hex(&credential.secret, &format!("desktop|{PHONE_CHALLENGE}"))
    );
}

#[test]
fn a_welcome_before_any_publication_carries_the_empty_state() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");

    let welcome = phone.authenticate(&credential.secret);

    assert_eq!(
        welcome["state"],
        json!({
            "mobileMode": false,
            "viewport": {"width": 412, "height": 915},
            "current": null,
            "workspaces": []
        })
    );
}

#[test]
fn the_welcome_carries_the_last_published_state() {
    let server = Server::start();
    let credential = server.enrol();
    server.handle.publish_state(&RemoteState {
        mobile_mode: true,
        viewport: Viewport {
            width: 390,
            height: 844,
        },
        current: Some(SessionId::new("s1")),
        workspaces: vec![RemoteWorkspace {
            name: "Main".to_owned(),
            accounts: vec![
                RemoteAccount {
                    id: SessionId::new("s1"),
                    name: "Alpha".to_owned(),
                    liveness: Liveness::Live,
                },
                RemoteAccount {
                    id: SessionId::new("s2"),
                    name: "Beta".to_owned(),
                    liveness: Liveness::Parked,
                },
            ],
        }],
    });
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");

    let welcome = phone.authenticate(&credential.secret);

    assert_eq!(
        welcome["state"],
        json!({
            "mobileMode": true,
            "viewport": {"width": 390, "height": 844},
            "current": "s1",
            "workspaces": [{
                "name": "Main",
                "accounts": [
                    {"id": "s1", "name": "Alpha", "liveness": "live"},
                    {"id": "s2", "name": "Beta", "liveness": "parked"}
                ]
            }]
        })
    );
}

#[test]
fn a_second_authenticated_socket_is_welcomed_while_the_first_gets_bye_replaced_and_is_closed() {
    let server = Server::start();
    let credential = server.enrol();
    let mut first = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");
    first.authenticate(&credential.secret);
    let mut second =
        WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");

    let welcome = second.authenticate(&credential.secret);

    let on_first = [first.recv(), first.recv()];
    assert_eq!(
        (welcome["type"].as_str(), on_first),
        (
            Some("welcome"),
            [
                Received::Text(r#"{"type":"bye","reason":"replaced"}"#.to_owned()),
                Received::Close
            ]
        )
    );
}

#[test]
fn a_message_before_auth_is_dropped_and_the_socket_still_waits_for_the_proof() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");
    let hello = phone.recv_json();
    let challenge = hello["challenge"].as_str().unwrap_or_default().to_owned();
    phone.send_json(&json!({"type": "tap", "x": 1, "y": 2}));

    phone.send_json(&json!({
        "type": "auth",
        "proof": hmac_hex(&credential.secret, &format!("phone|{challenge}")),
        "challenge": PHONE_CHALLENGE,
    }));

    let welcome = phone.recv_json();
    assert_eq!(
        (welcome["type"].as_str(), server.intents_within_silence()),
        (Some("welcome"), Vec::new())
    );
}
