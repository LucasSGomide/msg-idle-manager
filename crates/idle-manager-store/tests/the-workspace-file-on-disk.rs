//! `TomlWorkspaceStore` writes every workspace to one TOML file and reads it
//! back, round-tripping every field, migrating a version 1 file, repairing an
//! oversized named workspace, telling a missing file from a malformed one
//! from an unknown version, keeping an unreadable file aside intact, keeping a
//! version 1 file's bytes beside its migrated replacement, and never leaving a
//! temporary behind.

mod common;

use common::TempDir;
use idle_manager_core::{
    Account, Layout, SavedLiveness, SessionId, SlotId, Visibility, Workspace, WorkspaceId,
    WorkspaceList, WorkspaceStore, ZoomLevel,
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

fn ungrouped(accounts: Vec<Account>, layout: Layout) -> Workspace {
    Workspace {
        accounts,
        layout,
        ..Workspace::default()
    }
}

fn named(id: &str, name: &str, accounts: Vec<Account>, layout: Layout) -> Workspace {
    Workspace {
        id: WorkspaceId::new(id),
        name: name.to_owned(),
        accounts,
        layout,
        ..Workspace::default()
    }
}

fn a_list(workspaces: Vec<Workspace>, active: &str) -> WorkspaceList {
    WorkspaceList {
        workspaces,
        active: WorkspaceId::new(active),
        next_account_number: 1,
        next_workspace_number: 1,
    }
}

#[test]
fn writing_a_workspace_list_and_reading_it_back_yields_the_same_workspaces() {
    let dir = TempDir::new("workspace-round-trip");
    let store = TomlWorkspaceStore::under(dir.path());
    let list = a_list(
        vec![
            named(
                "workspace-0001",
                "Party",
                vec![
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
                ],
                Layout::SideBySide,
            ),
            ungrouped(vec![account("session-0003", "Farm")], Layout::Single),
        ],
        "workspace-0001",
    );

    store.save(&list).expect("write the workspace list");
    let loaded = store.load().expect("read the workspace list back");

    assert_eq!(loaded, list);
}

#[test]
fn the_written_file_matches_its_snapshot_with_the_version_key() {
    let dir = TempDir::new("workspace-snapshot");
    let store = TomlWorkspaceStore::under(dir.path());
    let list = a_list(
        vec![
            named(
                "workspace-0001",
                "Party",
                vec![
                    in_slot(
                        Account {
                            is_kept_awake: true,
                            browser_identity: Some("ProbeUA/1.0".to_owned()),
                            zoom: ZoomLevel::new(0.9).expect("0.9 is an accepted multiplier"),
                            ..account("session-0001", "Main account")
                        },
                        0,
                    ),
                    account("session-0002", "Alt"),
                ],
                Layout::Grid,
            ),
            ungrouped(
                vec![Account {
                    liveness: SavedLiveness::Parked,
                    ..account("session-0003", "Farm")
                }],
                Layout::Single,
            ),
        ],
        "workspace-0001",
    );
    let list = WorkspaceList {
        next_account_number: 4,
        next_workspace_number: 2,
        ..list
    };
    store.save(&list).expect("write the workspace list");

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
    let list = a_list(
        vec![ungrouped(
            vec![account("session-0001", "A")],
            Layout::Single,
        )],
        "ungrouped",
    );

    store.save(&list).expect("first write");
    store.save(&list).expect("second write over the first");

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
    let list = a_list(
        vec![ungrouped(
            vec![
                in_slot(account("session-0001", "Shown"), 2),
                account("session-0002", "Hidden"),
            ],
            Layout::Grid,
        )],
        "ungrouped",
    );
    store.save(&list).expect("write the workspace list");

    let loaded = store.load().expect("read the workspace list back");

    let accounts = &loaded.workspaces[0].accounts;
    assert_eq!(
        (accounts[0].visibility, accounts[1].visibility),
        (Visibility::InSlot(SlotId::new(2)), Visibility::OffGrid),
    );
}

#[test]
fn a_version_1_file_reads_as_one_active_expanded_ungrouped_workspace() {
    let dir = TempDir::new("workspace-v1-migration");
    dir.write(
        "sessions.toml",
        "version = 1\nlayout = \"side-by-side\"\n\n\
         [[account]]\nid = \"session-0001\"\nname = \"Main\"\nurl = \"https://a.test/\"\n\
         liveness = \"running\"\nslot = 0\nkeep_awake = false\nzoom = 1.0\n\n\
         [[account]]\nid = \"session-0004\"\nname = \"Alt\"\nurl = \"https://b.test/\"\n\
         liveness = \"parked\"\nkeep_awake = true\nzoom = 0.8\n",
    );
    let store = TomlWorkspaceStore::under(dir.path());

    let loaded = store.load().expect("migrate the version 1 file");

    assert_eq!(loaded.workspaces.len(), 1);
    let workspace = &loaded.workspaces[0];
    assert_eq!(
        (
            &loaded.active,
            &workspace.id,
            workspace.is_expanded,
            workspace.focused,
            workspace.layout,
            workspace
                .accounts
                .iter()
                .map(|a| a.display_name.as_str())
                .collect::<Vec<_>>(),
            loaded.next_account_number,
        ),
        (
            &WorkspaceId::ungrouped(),
            &WorkspaceId::ungrouped(),
            true,
            SlotId::FIRST,
            Layout::SideBySide,
            vec!["Main", "Alt"],
            5,
        ),
    );
}

#[test]
fn the_first_write_over_a_version_1_file_keeps_a_byte_equal_copy_beside_it() {
    let dir = TempDir::new("workspace-v1-backup");
    let original = "version = 1\nlayout = \"single\"\n\n\
         [[account]]\nid = \"session-0001\"\nname = \"Main\"\nurl = \"https://a.test/\"\n\
         liveness = \"running\"\nkeep_awake = false\nzoom = 1.0\n";
    dir.write("sessions.toml", original);
    let store = TomlWorkspaceStore::under(dir.path());
    let migrated = store.load().expect("migrate the version 1 file");

    store.save(&migrated).expect("write the migrated list");

    let backup = dir.join("sessions.v1.toml");
    assert_eq!(
        std::fs::read_to_string(&backup).expect("the v1 backup exists"),
        original,
    );
}

#[test]
fn a_second_write_does_not_replace_the_kept_version_1_copy() {
    let dir = TempDir::new("workspace-v1-backup-once");
    let original = "version = 1\nlayout = \"single\"\n";
    dir.write("sessions.toml", original);
    let store = TomlWorkspaceStore::under(dir.path());
    let migrated = store.load().expect("migrate the version 1 file");
    store.save(&migrated).expect("first write");
    let backup = dir.join("sessions.v1.toml");

    // A hand edit to the kept copy proves the second save leaves it alone —
    // a byte-for-byte re-copy would erase this.
    std::fs::write(&backup, "hand edited").expect("simulate a hand edit to the kept copy");
    store.save(&migrated).expect("second write");

    assert_eq!(
        std::fs::read_to_string(&backup).expect("the v1 backup still exists"),
        "hand edited",
    );
}

#[test]
fn a_duplicate_account_id_across_workspaces_is_malformed() {
    let dir = TempDir::new("workspace-duplicate-id");
    let list = a_list(
        vec![
            named(
                "workspace-0001",
                "Party",
                vec![account("session-0001", "A")],
                Layout::Single,
            ),
            ungrouped(vec![account("session-0001", "B")], Layout::Single),
        ],
        "ungrouped",
    );
    let store = TomlWorkspaceStore::under(dir.path());
    store
        .save(&list)
        .expect("write a file with a duplicate id past the domain's own checks");

    let outcome = store.load();

    assert!(matches!(outcome, Err(SessionFileError::Malformed { .. })));
}

#[test]
fn a_named_workspace_holding_five_accounts_keeps_four_and_moves_the_fifth_to_ungrouped() {
    let dir = TempDir::new("workspace-oversized");
    let accounts: Vec<Account> = (1..=5)
        .map(|n| account(&format!("session-000{n}"), &format!("A{n}")))
        .collect();
    dir.write(
        "sessions.toml",
        &format!(
            "version = 2\nactive = \"workspace-0001\"\nnext_account = 6\nnext_workspace = 2\n\n\
             [[workspace]]\nid = \"workspace-0001\"\nname = \"Party\"\nlayout = \"grid\"\n\
             focused = 0\nexpanded = true\n\n{}\n\
             [[workspace]]\nid = \"ungrouped\"\nname = \"Ungrouped\"\nlayout = \"single\"\n\
             focused = 0\nexpanded = true\n",
            accounts
                .iter()
                .map(|a| format!(
                    "[[workspace.account]]\nid = \"{}\"\nname = \"{}\"\nurl = \"https://a.test/\"\n\
                     liveness = \"running\"\nkeep_awake = false\nzoom = 1.0\n",
                    a.id, a.display_name
                ))
                .collect::<Vec<_>>()
                .join("\n"),
        ),
    );
    let store = TomlWorkspaceStore::under(dir.path());

    let loaded = store.load().expect("read the oversized file");

    let party = loaded
        .workspaces
        .iter()
        .find(|w| w.id == WorkspaceId::new("workspace-0001"))
        .expect("the party workspace is still present");
    let ungrouped = loaded
        .workspaces
        .iter()
        .find(|w| w.id.is_ungrouped())
        .expect("ungrouped is still present");
    assert_eq!(
        (
            party
                .accounts
                .iter()
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>(),
            ungrouped
                .accounts
                .iter()
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>(),
        ),
        (
            vec![
                "session-0001",
                "session-0002",
                "session-0003",
                "session-0004"
            ],
            vec!["session-0005"],
        ),
    );
}
