//! After `welcome` every message in the closed set becomes the matching
//! `RemoteIntent` on the channel, in order; any other `type` yields nothing
//! and gets no reply; and `publish_state` reaches the phone as exactly one
//! `state` message.

mod common;

use common::{Received, SILENCE, Server, WsClient, mobile_state};
use idle_manager_core::{PhoneLink as _, RemoteIntent, SessionId, Viewport};
use serde_json::json;

#[test]
fn attach_tap_and_park_arrive_as_the_three_intents_in_order() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");
    phone.authenticate(&credential.secret);

    phone.send_json(&json!({"type": "attach", "viewport": {"width": 412, "height": 915}}));
    phone.send_json(&json!({"type": "tap", "x": 1, "y": 2}));
    phone.send_json(&json!({"type": "park", "account": "session-0007"}));

    let intents = [
        server.next_intent(),
        server.next_intent(),
        server.next_intent(),
    ];
    assert_eq!(
        intents,
        [
            Some(RemoteIntent::Attach {
                viewport: Viewport {
                    width: 412,
                    height: 915
                }
            }),
            Some(RemoteIntent::Tap { x: 1, y: 2 }),
            Some(RemoteIntent::Park(SessionId::new("session-0007"))),
        ]
    );
}

#[test]
fn choose_start_mobile_scroll_and_leave_are_forwarded_too() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = server.attached_phone(&credential);

    phone.send_json(&json!({"type": "choose", "account": "a"}));
    phone.send_json(&json!({"type": "start", "account": "b"}));
    phone.send_json(&json!({"type": "mobile", "on": true}));
    phone.send_json(&json!({"type": "scroll", "x": 3, "y": 4, "dx": -5, "dy": 6}));
    phone.send_json(&json!({"type": "leave"}));

    let intents: Vec<_> = (0..5).map(|_| server.next_intent()).collect();
    assert_eq!(
        intents,
        vec![
            Some(RemoteIntent::ChooseAccount(SessionId::new("a"))),
            Some(RemoteIntent::Start(SessionId::new("b"))),
            Some(RemoteIntent::SetMobileMode(true)),
            Some(RemoteIntent::Scroll {
                x: 3,
                y: 4,
                dx: -5,
                dy: 6
            }),
            Some(RemoteIntent::Leave),
        ]
    );
}

#[test]
fn an_unknown_type_yields_no_intent_and_no_reply() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");
    phone.authenticate(&credential.secret);

    phone.send_json(&json!({"type": "rename", "account": "a", "name": "x"}));
    phone.send_text("not even json");

    assert_eq!(
        (server.intents_within_silence(), phone.recv_within(SILENCE)),
        (Vec::new(), Received::Quiet)
    );
}

#[test]
fn a_malformed_known_type_yields_nothing_and_the_next_message_still_arrives() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");
    phone.authenticate(&credential.secret);

    phone.send_json(&json!({"type": "tap", "x": 1}));
    phone.send_json(&json!({"type": "tap", "x": 9, "y": 8}));

    assert_eq!(server.next_intent(), Some(RemoteIntent::Tap { x: 9, y: 8 }));
}

#[test]
fn publish_state_arrives_as_exactly_one_state_message() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");
    phone.authenticate(&credential.secret);

    server.handle.publish_state(&mobile_state());

    let first = phone.recv_json();
    let then = phone.recv_within(SILENCE);
    assert_eq!(
        (first, then),
        (
            json!({
                "type": "state",
                "state": {
                    "mobileMode": true,
                    "viewport": {"width": 412, "height": 915},
                    "current": null,
                    "workspaces": []
                }
            }),
            Received::Quiet
        )
    );
}

#[test]
fn publish_state_with_no_phone_connected_is_kept_for_the_next_welcome() {
    let server = Server::start();
    let credential = server.enrol();

    server.handle.publish_state(&mobile_state());

    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");
    let welcome = phone.authenticate(&credential.secret);
    assert_eq!(welcome["state"]["mobileMode"], true);
}

#[test]
fn a_websocket_ping_is_answered_with_a_pong_echoing_its_payload() {
    let server = Server::start();
    let credential = server.enrol();
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");
    phone.authenticate(&credential.secret);

    phone.send_ping(b"beat");

    assert_eq!(phone.recv(), Received::Pong(b"beat".to_vec()));
}
