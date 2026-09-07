//! `TomlWorkspaceStore` writes the workspace to one TOML file and reads it
//! back, round-tripping every field, telling a missing file from a malformed
//! one from an unknown version, keeping an unreadable file aside intact, and
//! never leaving a temporary behind.

mod common;

use common::TempDir;
use idle_manager_core::{
    Account, Layout, SavedLiveness, SessionId, SlotId, Visibility, Workspace, WorkspaceStore,
    ZoomLevel,
};
use idle_manager_store::{SessionFileError, TomlWorkspaceStore};

fn account(id: &str, name: &str) -> Account {
    Account {
        id: SessionId::new(id),
        display_name: name.to_owned(),
        start_address: format!("https://{id}.test/"),
        liveness: SavedLiveness::Running,
        visibility: Visibility::OffGrid,
        remembered_slot: None,
        is_kept_awake: false,
        browser_identity: None,
        zoom: ZoomLevel::DEFAULT,
    }
}

fn in_slot(account: Account, index: usize) -> Account {
    Account {
        visibility: Visibility::InSlot(SlotId::new(index)),
        remembered_slot: Some(SlotId::new(index)),
        ..account
    }
}

#[test]
fn writing_a_workspace_and_reading_it_back_yields_the_same_accounts_and_layout() {
    let dir = TempDir::new("workspace-round-trip");
    let store = TomlWorkspaceStore::under(dir.path());
    let workspace = Workspace {
        accounts: vec![
            in_slot(
                Account {
                    is_kept_awake: true,
                    browser_identity: Some("ProbeUA/1.0".to_owned()),
                    zoom: ZoomLevel::new(0.8).expect("0.8 is an accepted multiplier"),
                    ..account("session-0001", "Main account")
                },
                0,
            ),
            in_slot(
                Account {
                    liveness: SavedLiveness::Parked,
                    ..account("session-0002", "Alt")
                },
                1,
            ),
            account("session-0003", "Farm"),
        ],
        layout: Layout::Grid,
    };

    store.save(&workspace).expect("write the workspace");
    let loaded = store.load().expect("read the workspace back");

    assert_eq!(loaded, workspace);
}

#[test]
fn the_written_file_matches_its_snapshot_with_the_version_key() {
    let dir = TempDir::new("workspace-snapshot");
    let store = TomlWorkspaceStore::under(dir.path());
    let workspace = Workspace {
        accounts: vec![
            in_slot(
                Account {
                    is_kept_awake: true,
                    browser_identity: Some("ProbeUA/1.0".to_owned()),
                    zoom: ZoomLevel::new(0.9).expect("0.9 is an accepted multiplier"),
                    ..account("session-0001", "Main account")
                },
                0,
            ),
            in_slot(
                Account {
                    liveness: SavedLiveness::Parked,
                    ..account("session-0002", "Alt")
                },
                3,
            ),
            account("session-0003", "Farm"),
        ],
        layout: Layout::Grid,
    };
    store.save(&workspace).expect("write the workspace");

    let on_disk = std::fs::read_to_string(store.path()).expect("read the written file");

    insta::assert_snapshot!("workspace-file", on_disk);
}

#[test]
fn reading_a_directory_with_no_workspace_file_reports_the_missing_case() {
    let dir = TempDir::new("workspace-missing");
    let store = TomlWorkspaceStore::under(dir.path());

    let load = store.load();
    let read = store.read();

    assert_eq!(
        (
            matches!(load, Err(SessionFileError::Missing { .. })),
            matches!(read, Ok(None)),
        ),
        (true, true),
    );
}

#[test]
fn an_invalid_toml_file_reports_malformed_and_is_kept_aside_with_its_bytes_intact() {
    let dir = TempDir::new("workspace-malformed");
    let original = "this is not = valid toml [[[";
    dir.write("sessions.toml", original);
    let store = TomlWorkspaceStore::under(dir.path());

    let outcome = store.load();

    let SessionFileError::Malformed { kept, .. } = outcome.expect_err("a malformed file") else {
        panic!("expected Malformed, got a different variant");
    };
    assert_eq!(
        (
            std::fs::read_to_string(&kept).expect("the kept file is still readable"),
            store.path().exists(),
        ),
        (original.to_owned(), false),
    );
}

#[test]
fn a_file_with_an_unknown_version_reports_its_own_case_and_is_kept_aside() {
    let dir = TempDir::new("workspace-version");
    dir.write("sessions.toml", "version = 999\nlayout = \"single\"\n");
    let store = TomlWorkspaceStore::under(dir.path());

    let outcome = store.load();

    let SessionFileError::UnknownVersion { found, kept, .. } =
        outcome.expect_err("an unknown version")
    else {
        panic!("expected UnknownVersion, got a different variant");
    };
    assert_eq!(
        (found, kept.exists(), store.path().exists()),
        (999, true, false)
    );
}

#[test]
fn an_unrepresentable_liveness_is_a_parse_failure_not_a_default() {
    let dir = TempDir::new("workspace-bad-liveness");
    dir.write(
        "sessions.toml",
        "version = 1\nlayout = \"grid\"\n\n[[account]]\nid = \"session-0001\"\n\
         name = \"A\"\nurl = \"https://a.test/\"\nliveness = \"hibernating\"\n\
         keep_awake = false\nzoom = 1.0\n",
    );
    let store = TomlWorkspaceStore::under(dir.path());

    assert!(matches!(
        store.load(),
        Err(SessionFileError::Malformed { .. })
    ));
}

#[test]
fn an_unknown_key_is_a_parse_failure_not_a_silently_ignored_value() {
    let dir = TempDir::new("workspace-unknown-key");
    dir.write(
        "sessions.toml",
        "version = 1\nlayout = \"grid\"\n\n[[account]]\nid = \"session-0001\"\n\
         name = \"A\"\nurl = \"https://a.test/\"\nliveness = \"running\"\n\
         keep_awake = false\nzoom = 1.0\nfavourite = true\n",
    );
    let store = TomlWorkspaceStore::under(dir.path());

    assert!(matches!(
        store.load(),
        Err(SessionFileError::Malformed { .. })
    ));
}

#[test]
fn writing_over_an_existing_file_leaves_no_temporary_behind() {
    let dir = TempDir::new("workspace-temp");
    let store = TomlWorkspaceStore::under(dir.path());
    let workspace = Workspace {
        accounts: vec![account("session-0001", "A")],
        layout: Layout::Single,
    };

    store.save(&workspace).expect("first write");
    store.save(&workspace).expect("second write over the first");

    let mut entries: Vec<String> = std::fs::read_dir(dir.path())
        .expect("read the configuration directory")
        .map(|entry| {
            entry
                .expect("a directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    entries.sort();
    assert_eq!(entries, vec!["sessions.toml".to_owned()]);
}

#[test]
fn an_account_saved_off_grid_reads_back_off_grid_and_one_in_a_slot_reads_back_in_it() {
    let dir = TempDir::new("workspace-visibility");
    let store = TomlWorkspaceStore::under(dir.path());
    let workspace = Workspace {
        accounts: vec![
            in_slot(account("session-0001", "Shown"), 2),
            account("session-0002", "Hidden"),
        ],
        layout: Layout::Grid,
    };
    store.save(&workspace).expect("write the workspace");

    let loaded = store.load().expect("read the workspace back");

    assert_eq!(
        (loaded.accounts[0].visibility, loaded.accounts[1].visibility,),
        (Visibility::InSlot(SlotId::new(2)), Visibility::OffGrid),
    );
}
