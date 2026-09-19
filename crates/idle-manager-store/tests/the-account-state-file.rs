//! `TomlZoomMemory` writes each account's chosen zoom sizes to one small
//! `state.toml` at the root of its profile folder and reads them back —
//! round-tripping only the changed arrangements, tolerating a bad value or an
//! unknown key per entry, reading a missing or unparseable file as nothing
//! remembered, and reporting a write it could not complete.

mod common;

use std::fs;

use common::TempDir;
#[cfg(unix)]
use idle_manager_core::ZoomMemoryError;
use idle_manager_core::{Layout, ProfileLocator, RememberedZoom, SessionId, ZoomLevel, ZoomMemory};
use idle_manager_store::{TomlZoomMemory, XdgProfileLocator};

fn account() -> SessionId {
    SessionId::new("session-0001")
}

fn remembered(pairs: &[(Layout, f64)]) -> RememberedZoom {
    pairs
        .iter()
        .map(|(layout, multiplier)| {
            (
                *layout,
                ZoomLevel::new(*multiplier).expect("an accepted multiplier"),
            )
        })
        .collect()
}

#[test]
fn a_map_written_for_two_arrangements_reads_back_as_exactly_those_two() {
    let dir = TempDir::new("zoom-round-trip");
    let memory = TomlZoomMemory::under(dir.path());
    let id = account();

    memory
        .write(
            &id,
            &remembered(&[(Layout::Single, 1.2), (Layout::Grid, 0.8)]),
        )
        .expect("the write completes");

    let read = memory.read(&id);
    assert_eq!(
        (
            read.get(Layout::Single),
            read.get(Layout::SideBySide),
            read.get(Layout::Grid),
        ),
        (
            Some(ZoomLevel::new(1.2).expect("accepted")),
            None,
            Some(ZoomLevel::new(0.8).expect("accepted")),
        ),
    );
}

#[test]
fn the_file_is_written_as_state_toml_beside_the_accounts_data_and_cache_directories() {
    let dir = TempDir::new("zoom-sibling");
    let locator = XdgProfileLocator::under(dir.path());
    let memory = TomlZoomMemory::under(dir.path());
    let id = account();

    let directories = locator
        .locate(&id)
        .expect("the profile directories are made");
    memory
        .write(&id, &remembered(&[(Layout::Single, 1.1)]))
        .expect("the write completes");

    let state = memory.state_file(&id);
    assert_eq!(
        (
            state.file_name().and_then(|name| name.to_str()),
            state.parent(),
            state.parent(),
            state.is_file(),
        ),
        (
            Some("state.toml"),
            directories.data.parent(),
            directories.cache.parent(),
            true,
        ),
    );
}

#[test]
fn reading_an_account_with_no_state_file_returns_nothing_remembered_and_no_error() {
    let dir = TempDir::new("zoom-missing");
    let memory = TomlZoomMemory::under(dir.path());

    let read = memory.read(&account());

    assert!(read.is_empty());
}

#[test]
fn a_value_outside_the_accepted_range_is_dropped_and_the_other_arrangements_still_read_back() {
    let dir = TempDir::new("zoom-bad-value");
    let id = account();
    seed_state_file(&dir, &id, "[zoom]\nsingle = 1.2\ngrid = 99.0\n");

    let read = TomlZoomMemory::under(dir.path()).read(&id);

    assert_eq!(
        (read.get(Layout::Single), read.get(Layout::Grid)),
        (Some(ZoomLevel::new(1.2).expect("accepted")), None),
    );
}

#[test]
fn an_unrecognised_key_under_the_zoom_table_is_ignored_and_the_recognised_keys_still_read_back() {
    let dir = TempDir::new("zoom-unknown-key");
    let id = account();
    seed_state_file(&dir, &id, "[zoom]\nsingle = 1.2\ntriple = 0.5\n");

    let read = TomlZoomMemory::under(dir.path()).read(&id);

    assert_eq!(
        (read.get(Layout::Single), read.is_empty()),
        (Some(ZoomLevel::new(1.2).expect("accepted")), false),
    );
}

#[test]
fn a_state_file_that_is_not_valid_toml_reads_as_nothing_remembered() {
    let dir = TempDir::new("zoom-garbage");
    let id = account();
    seed_state_file(&dir, &id, "this is not = = toml [[[");

    let read = TomlZoomMemory::under(dir.path()).read(&id);

    assert!(read.is_empty());
}

#[test]
fn a_written_file_shows_only_the_changed_arrangements_under_a_zoom_table() {
    let dir = TempDir::new("zoom-snapshot");
    let memory = TomlZoomMemory::under(dir.path());
    let id = account();

    memory
        .write(
            &id,
            &remembered(&[(Layout::SideBySide, 0.9), (Layout::Grid, 0.75)]),
        )
        .expect("the write completes");

    let on_disk = fs::read_to_string(memory.state_file(&id)).expect("read the written file");
    insta::assert_snapshot!("account-state-file", on_disk);
}

// Read-only-directory permissions are a Unix concept; Windows' ACL model has
// no equivalent `set_mode` call (roadmap item 12, "carried over" note).
#[cfg(unix)]
#[test]
fn writing_into_a_profile_root_the_process_cannot_write_returns_the_error_and_never_panics() {
    let dir = TempDir::new("zoom-readonly");
    let id = account();
    let profile = dir.join("profiles/session-0001");
    fs::create_dir_all(&profile).expect("create the profile folder");
    set_mode(&profile, 0o555);

    let outcome =
        TomlZoomMemory::under(dir.path()).write(&id, &remembered(&[(Layout::Single, 1.1)]));

    set_mode(&profile, 0o755);
    let ZoomMemoryError::NotStored { reason } = outcome.expect_err("the write fails");
    assert!(!reason.is_empty());
}

/// Writes a `state.toml` by hand for `id`, creating the profile folder first.
fn seed_state_file(dir: &TempDir, id: &SessionId, contents: &str) {
    let profile = dir.join(&format!("profiles/{}", id.as_str()));
    fs::create_dir_all(&profile).expect("create the profile folder");
    fs::write(profile.join("state.toml"), contents).expect("write the state file");
}

#[cfg(unix)]
fn set_mode(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).expect("metadata").permissions();
    perms.set_mode(mode);
    fs::set_permissions(path, perms).expect("set permissions");
}
