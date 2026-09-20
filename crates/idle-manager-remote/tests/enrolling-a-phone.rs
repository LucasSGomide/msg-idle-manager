//! `GET /enrol/{code}` consumes a live code once: it answers the phone page
//! with the credential in two meta tags and the device cookie, writes the
//! record, and refuses the same code, an unknown code, an expired code and a
//! cancelled offer with an empty 404.

mod common;

use common::{COOKIE_NAME, Server, hex, http_get, meta_content};
use idle_manager_core::PhoneLink as _;
use idle_manager_remote::ENROLMENT_CODE_TTL_SECS;

#[test]
fn the_offer_names_the_bound_address_a_64_hex_code_and_the_ten_minute_ttl() {
    let server = Server::start();

    let offer = server.handle.begin_enrolment();

    let prefix = format!("http://{}/enrol/", server.addr());
    let code = offer.address.strip_prefix(&prefix).unwrap_or_default();
    assert!(
        code.len() == 64
            && code.chars().all(|c| c.is_ascii_hexdigit())
            && offer.expires_in_secs == ENROLMENT_CODE_TTL_SECS,
        "{offer:?}"
    );
}

#[test]
fn a_live_code_answers_200_with_both_credential_tags() {
    let server = Server::start();

    let response = http_get(server.addr(), &server.enrolment_path(), &[]);

    let page = response.body_text();
    let device_id = meta_content(&page, "idle-manager-device-id").unwrap_or_default();
    let secret = meta_content(&page, "idle-manager-secret").unwrap_or_default();
    assert!(
        response.status == 200
            && response.header("Content-Type") == Some("text/html; charset=utf-8")
            && page.contains("<title>Idle Manager</title>")
            && device_id.len() == 32
            && secret.len() == 64,
        "{}\n{page}",
        response.head
    );
}

#[test]
fn a_live_code_sets_the_device_cookie_for_a_year() {
    let server = Server::start();

    let response = http_get(server.addr(), &server.enrolment_path(), &[]);

    let device_id =
        meta_content(&response.body_text(), "idle-manager-device-id").unwrap_or_default();
    assert_eq!(
        response.header("Set-Cookie"),
        Some(
            format!(
                "{COOKIE_NAME}={device_id}; Path=/; Max-Age=31536000; SameSite=Strict; HttpOnly"
            )
            .as_str()
        )
    );
}

#[test]
fn a_live_code_writes_the_record_the_page_carries() {
    let server = Server::start();

    let credential = server.enrol();

    let phone = server.record.phone().expect("the record was written");
    assert_eq!(
        (phone.device_id, hex(&phone.secret), phone.secret.len()),
        (credential.device_id, hex(&credential.secret), 32)
    );
}

#[test]
fn the_same_code_a_second_time_answers_404_with_an_empty_body() {
    let server = Server::start();
    let path = server.enrolment_path();
    http_get(server.addr(), &path, &[]);

    let second = http_get(server.addr(), &path, &[]);

    assert_eq!(
        (second.status, second.body.len()),
        (404, 0),
        "{}",
        second.head
    );
}

#[test]
fn an_unknown_code_answers_404() {
    let server = Server::start();
    server.handle.begin_enrolment();

    let response = http_get(server.addr(), &format!("/enrol/{}", "0".repeat(64)), &[]);

    assert_eq!((response.status, response.body.len()), (404, 0));
}

#[test]
fn an_expired_code_answers_404() {
    let server = Server::start();
    let path = server.enrolment_path();
    server.clock.advance_secs(ENROLMENT_CODE_TTL_SECS);

    let response = http_get(server.addr(), &path, &[]);

    assert_eq!((response.status, server.record.phone()), (404, None));
}

#[test]
fn a_cancelled_offer_is_refused() {
    let server = Server::start();
    let path = server.enrolment_path();
    server.handle.cancel_enrolment();

    let response = http_get(server.addr(), &path, &[]);

    assert_eq!(response.status, 404);
}

#[test]
fn a_second_enrolment_replaces_the_first_phone() {
    let server = Server::start();
    let first = server.enrol();

    let second = server.enrol();

    let first_cookie = http_get(server.addr(), "/", &[("Cookie", &first.cookie())]);
    assert_eq!(
        (
            server.record.phone().map(|phone| phone.device_id),
            first_cookie.status
        ),
        (Some(second.device_id), 404)
    );
}

#[test]
fn phone_status_moves_from_not_enrolled_to_enrolled_and_not_attached() {
    let server = Server::start();
    let before = server.handle.phone_status();

    server.enrol();

    assert_eq!(
        (before, server.handle.phone_status()),
        (
            idle_manager_core::PhoneStatus::NotEnrolled,
            idle_manager_core::PhoneStatus::Enrolled { attached: false }
        )
    );
}
