//! `publish_frame` reaches an attached phone in mobile mode as one binary
//! message — big-endian width and height, then a JPEG — encoded on the
//! server's thread; identical pixels are not sent twice, and nothing is sent
//! before `attach` or while mobile mode is off.

mod common;

use common::{Received, SILENCE, Server, WsClient, mobile_state};
use idle_manager_core::{Frame, PhoneLink as _, RemoteState};

fn solid_frame(width: u32, height: u32, stride: u32, rgba: [u8; 4]) -> Frame {
    let mut bytes = vec![0u8; (stride * height) as usize];
    for row in bytes.chunks_exact_mut(stride as usize) {
        let (pixels, _) = row[..(width * 4) as usize].as_chunks_mut::<4>();
        for pixel in pixels {
            *pixel = rgba;
        }
    }
    Frame::Rgba {
        width,
        height,
        stride,
        bytes,
    }
}

fn binary(received: Received) -> Vec<u8> {
    match received {
        Received::Binary(payload) => payload,
        other => panic!("expected a binary message, got {other:?}"),
    }
}

/// Whether `bytes` start with a JPEG's SOI marker and end with its EOI marker.
fn looks_like_a_jpeg(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8]) && bytes.ends_with(&[0xFF, 0xD9])
}

#[test]
fn a_published_rgba_frame_arrives_as_one_binary_message_with_its_size_and_a_jpeg() {
    let server = Server::start();
    let credential = server.enrol();
    server.handle.publish_state(&mobile_state());
    let mut phone = server.attached_phone(&credential);

    server
        .handle
        .publish_frame(solid_frame(16, 8, 64, [200, 30, 30, 255]));

    let message = binary(phone.recv());
    let then = phone.recv_within(SILENCE);
    assert!(
        message[..8] == [0, 0, 0, 16, 0, 0, 0, 8]
            && looks_like_a_jpeg(&message[8..])
            && then == Received::Quiet,
        "{:?}",
        &message[..12.min(message.len())]
    );
}

#[test]
fn a_padded_stride_is_honoured() {
    let server = Server::start();
    let credential = server.enrol();
    server.handle.publish_state(&mobile_state());
    let mut phone = server.attached_phone(&credential);
    server
        .handle
        .publish_frame(solid_frame(16, 8, 64, [10, 200, 10, 255]));
    let packed = binary(phone.recv());
    server
        .handle
        .publish_frame(solid_frame(16, 8, 96, [10, 200, 10, 255]));
    let after_identical = phone.recv_within(SILENCE);
    server
        .handle
        .publish_frame(solid_frame(16, 8, 96, [10, 200, 11, 255]));

    let padded = binary(phone.recv());

    assert!(
        after_identical == Received::Quiet
            && padded[..8] == packed[..8]
            && looks_like_a_jpeg(&padded[8..]),
        "{after_identical:?}"
    );
}

#[test]
fn the_same_pixels_published_again_send_nothing() {
    let server = Server::start();
    let credential = server.enrol();
    server.handle.publish_state(&mobile_state());
    let mut phone = server.attached_phone(&credential);
    server
        .handle
        .publish_frame(solid_frame(16, 8, 64, [0, 0, 255, 255]));
    binary(phone.recv());

    server
        .handle
        .publish_frame(solid_frame(16, 8, 64, [0, 0, 255, 255]));

    assert_eq!(phone.recv_within(SILENCE), Received::Quiet);
}

#[test]
fn different_pixels_are_sent_as_a_new_frame() {
    let server = Server::start();
    let credential = server.enrol();
    server.handle.publish_state(&mobile_state());
    let mut phone = server.attached_phone(&credential);
    server
        .handle
        .publish_frame(solid_frame(16, 8, 64, [0, 0, 255, 255]));
    let first = binary(phone.recv());

    server
        .handle
        .publish_frame(solid_frame(16, 8, 64, [255, 255, 0, 255]));

    let second = binary(phone.recv());
    assert!(second[..8] == first[..8] && second[8..] != first[8..]);
}

#[test]
fn frames_published_before_attach_send_nothing() {
    let server = Server::start();
    let credential = server.enrol();
    server.handle.publish_state(&mobile_state());
    let mut phone = WsClient::connect(server.addr(), Some(&credential.cookie())).expect("upgrades");
    phone.authenticate(&credential.secret);

    server
        .handle
        .publish_frame(solid_frame(16, 8, 64, [1, 2, 3, 255]));

    assert_eq!(phone.recv_within(SILENCE), Received::Quiet);
}

#[test]
fn frames_while_mobile_mode_is_off_send_nothing() {
    let server = Server::start();
    let credential = server.enrol();
    server.handle.publish_state(&RemoteState {
        mobile_mode: false,
        ..mobile_state()
    });
    let mut phone = server.attached_phone(&credential);

    server
        .handle
        .publish_frame(solid_frame(16, 8, 64, [1, 2, 3, 255]));

    assert_eq!(phone.recv_within(SILENCE), Received::Quiet);
}

#[test]
fn a_ready_jpeg_passes_through_with_the_size_from_its_own_header() {
    let server = Server::start();
    let credential = server.enrol();
    server.handle.publish_state(&mobile_state());
    let mut phone = server.attached_phone(&credential);
    server
        .handle
        .publish_frame(solid_frame(20, 12, 80, [9, 9, 9, 255]));
    let jpeg = binary(phone.recv())[8..].to_vec();
    server
        .handle
        .publish_frame(solid_frame(16, 8, 64, [0, 0, 0, 255]));
    binary(phone.recv());

    server.handle.publish_frame(Frame::Jpeg(jpeg.clone()));

    let passed = binary(phone.recv());
    assert_eq!(
        (&passed[..8], &passed[8..]),
        (&[0, 0, 0, 20, 0, 0, 0, 12][..], &jpeg[..])
    );
}
