//! `TomlPhoneRecord` keeps the one enrolled phone in `phone.toml` under the
//! configuration directory — round-tripping the record with its secret as
//! hex, creating the file readable by the user alone, reading a missing file
//! as no phone, setting a malformed file aside as `phone.toml.unreadable`,
//! clearing the record, and reading the hand-editable `[listen]` override
//! beside it.

mod common;

use std::fs;

use common::TempDir;
use idle_manager_core::{EnrolledPhone, PhoneRecordError, PhoneRecordStore};
use idle_manager_store::TomlPhoneRecord;

fn a_phone() -> EnrolledPhone {
    EnrolledPhone {
        device_id: "device-3f9a".to_owned(),
        secret: (0..32).collect(),
        enrolled_on: "2026-09-19T10:00:00Z".to_owned(),
    }
}

#[test]
fn a_written_phone_reads_back_equal() {
    let dir = TempDir::new("phone-round-trip");
    let store = TomlPhoneRecord::under(dir.path());

    store.write(&a_phone()).expect("the write completes");

    let read = store.read().expect("the read completes");
    assert_eq!(read, Some(a_phone()));
}

#[test]
fn the_secret_is_written_as_hex_under_a_phone_table() {
    let dir = TempDir::new("phone-hex");
    let store = TomlPhoneRecord::under(dir.path());

    store.write(&a_phone()).expect("the write completes");

    let on_disk = fs::read_to_string(store.path()).expect("read the written file");
    assert!(
        on_disk.contains("[phone]")
            && on_disk.contains(
                "secret = \"000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f\""
            ),
        "unexpected file body:\n{on_disk}"
    );
}

// File modes are a Unix concept; Windows' ACL model has no equivalent.
#[cfg(unix)]
#[test]
fn the_file_is_created_readable_and_writable_by_the_owner_alone() {
    use std::os::unix::fs::PermissionsExt;

    let dir = TempDir::new("phone-mode");
    let store = TomlPhoneRecord::under(dir.path());

    store.write(&a_phone()).expect("the write completes");

    let mode = fs::metadata(store.path())
        .expect("metadata")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[test]
fn a_second_write_replaces_the_first_phone() {
    let dir = TempDir::new("phone-replace");
    let store = TomlPhoneRecord::under(dir.path());
    store.write(&a_phone()).expect("the first write completes");
    let second = EnrolledPhone {
        device_id: "device-later".to_owned(),
        ..a_phone()
    };

    store.write(&second).expect("the second write completes");

    let read = store.read().expect("the read completes");
    assert_eq!(
        read.map(|phone| phone.device_id),
        Some("device-later".to_owned())
    );
}

#[test]
fn a_missing_file_reads_as_no_phone_and_no_error() {
    let dir = TempDir::new("phone-missing");
    let store = TomlPhoneRecord::under(dir.path());

    let read = store.read().expect("a missing file is not an error");

    assert_eq!(read, None);
}

#[test]
fn a_malformed_file_is_moved_aside_and_reported_as_unreadable() {
    let dir = TempDir::new("phone-malformed");
    dir.write("phone.toml", "this is not = = toml [[[");
    let store = TomlPhoneRecord::under(dir.path());

    let outcome = store.read();

    let Err(PhoneRecordError::Unreadable { kept, reason }) = outcome else {
        panic!("expected Unreadable, got {outcome:?}");
    };
    assert_eq!(
        (
            kept.file_name().and_then(|name| name.to_str()),
            kept.is_file(),
            store.path().exists(),
            reason.is_empty(),
        ),
        (Some("phone.toml.unreadable"), true, false, false),
    );
}

#[test]
fn a_secret_that_is_not_hex_is_unreadable_too() {
    let dir = TempDir::new("phone-bad-secret");
    dir.write(
        "phone.toml",
        "[phone]\ndevice_id = \"d\"\nsecret = \"not-hex\"\nenrolled_on = \"today\"\n",
    );
    let store = TomlPhoneRecord::under(dir.path());

    let outcome = store.read();

    assert!(matches!(outcome, Err(PhoneRecordError::Unreadable { .. })));
}

#[test]
fn after_a_malformed_read_the_next_read_finds_no_phone() {
    let dir = TempDir::new("phone-malformed-then-clean");
    dir.write("phone.toml", "garbage");
    let store = TomlPhoneRecord::under(dir.path());
    store.read().expect_err("the first read reports the damage");

    let second = store.read().expect("the file is gone, so nothing is wrong");

    assert_eq!(second, None);
}

#[test]
fn clear_removes_the_record_so_a_following_read_is_none() {
    let dir = TempDir::new("phone-clear");
    let store = TomlPhoneRecord::under(dir.path());
    store.write(&a_phone()).expect("the write completes");

    store.clear().expect("the clear completes");

    let read = store.read().expect("the read completes");
    assert_eq!((read, store.path().exists()), (None, false));
}

#[test]
fn clearing_with_no_phone_enrolled_is_not_an_error() {
    let dir = TempDir::new("phone-clear-nothing");
    let store = TomlPhoneRecord::under(dir.path());

    let outcome = store.clear();

    assert!(outcome.is_ok());
}

#[test]
fn the_listen_override_is_the_address_the_user_wrote() {
    let dir = TempDir::new("phone-listen");
    dir.write("phone.toml", "[listen]\naddress = \"100.64.0.7:7466\"\n");
    let store = TomlPhoneRecord::under(dir.path());

    let override_ = store.listen_override();

    assert_eq!(override_, Some("100.64.0.7:7466".to_owned()));
}

#[test]
fn the_listen_override_is_none_when_the_key_is_absent() {
    let dir = TempDir::new("phone-listen-absent");
    let store = TomlPhoneRecord::under(dir.path());
    store.write(&a_phone()).expect("the write completes");

    let override_ = store.listen_override();

    assert_eq!(override_, None);
}

#[test]
fn the_listen_override_is_none_when_the_address_is_auto() {
    let dir = TempDir::new("phone-listen-auto");
    dir.write("phone.toml", "[listen]\naddress = \"auto\"\n");
    let store = TomlPhoneRecord::under(dir.path());

    let override_ = store.listen_override();

    assert_eq!(override_, None);
}

#[test]
fn a_file_holding_only_the_listen_table_reads_as_no_phone() {
    let dir = TempDir::new("phone-listen-only");
    dir.write("phone.toml", "[listen]\naddress = \"100.64.0.7:7466\"\n");
    let store = TomlPhoneRecord::under(dir.path());

    let read = store.read().expect("the read completes");

    assert_eq!(read, None);
}

#[test]
fn writing_and_clearing_the_phone_keeps_the_users_listen_override() {
    let dir = TempDir::new("phone-listen-kept");
    dir.write("phone.toml", "[listen]\naddress = \"100.64.0.7:7466\"\n");
    let store = TomlPhoneRecord::under(dir.path());

    store.write(&a_phone()).expect("the write completes");
    let after_write = store.listen_override();
    store.clear().expect("the clear completes");
    let after_clear = store.listen_override();

    assert_eq!(
        (
            after_write,
            after_clear,
            store.read().expect("the read completes")
        ),
        (
            Some("100.64.0.7:7466".to_owned()),
            Some("100.64.0.7:7466".to_owned()),
            None
        )
    );
}

// Read-only-directory permissions are a Unix concept; Windows' ACL model has
// no equivalent `set_mode` call.
#[cfg(unix)]
#[test]
fn writing_into_a_directory_the_process_cannot_write_is_inaccessible_and_never_panics() {
    let dir = TempDir::new("phone-readonly");
    let store = TomlPhoneRecord::under(dir.path());
    dir.make_readonly();

    let outcome = store.write(&a_phone());

    assert!(matches!(
        outcome,
        Err(PhoneRecordError::Inaccessible { .. })
    ));
}
