//! `XdgProfileRemoval` deletes exactly one account's profile folder, leaves
//! every sibling untouched, treats an already-missing folder as success, and
//! reports a one-line reason when it genuinely cannot remove one.

mod common;

use common::TempDir;
use idle_manager_core::{ProfileLocator, ProfileRemoval, SessionId};
use idle_manager_store::{XdgProfileLocator, XdgProfileRemoval};

#[test]
fn remove_deletes_the_profile_folder_with_its_nested_files_and_subfolders() {
    let dir = TempDir::new("profile-removal");
    let locator = XdgProfileLocator::under(dir.path());
    let removal = XdgProfileRemoval::under(dir.path());
    let id = SessionId::new("session-0001");
    let directories = locator.locate(&id).expect("locate the profile");
    std::fs::write(directories.data.join("cookies.sqlite"), b"data").expect("seed a file");

    removal.remove(&id).expect("remove the profile folder");

    assert!(!removal.folder(&id).exists());
}

#[test]
fn removing_one_accounts_folder_leaves_a_siblings_folder_and_its_files_untouched() {
    let dir = TempDir::new("profile-removal-sibling");
    let locator = XdgProfileLocator::under(dir.path());
    let removal = XdgProfileRemoval::under(dir.path());
    let (removed, kept) = (
        SessionId::new("session-0001"),
        SessionId::new("session-0002"),
    );
    locator.locate(&removed).expect("locate the first profile");
    let kept_dirs = locator.locate(&kept).expect("locate the second profile");
    std::fs::write(kept_dirs.data.join("cookies.sqlite"), b"kept").expect("seed a file");

    removal
        .remove(&removed)
        .expect("remove the first profile folder");

    assert_eq!(
        (
            removal.folder(&kept).exists(),
            std::fs::read(kept_dirs.data.join("cookies.sqlite"))
                .expect("the sibling file survives"),
        ),
        (true, b"kept".to_vec()),
    );
}

#[test]
fn remove_on_an_account_whose_folder_does_not_exist_returns_ok() {
    let dir = TempDir::new("profile-removal-missing");
    let removal = XdgProfileRemoval::under(dir.path());

    assert!(removal.remove(&SessionId::new("session-9999")).is_ok());
}

#[test]
fn folder_returns_the_same_path_account_profile_dir_gives() {
    let dir = TempDir::new("profile-removal-folder-path");
    let locator = XdgProfileLocator::under(dir.path());
    let removal = XdgProfileRemoval::under(dir.path());
    let id = SessionId::new("session-0001");
    let directories = locator.locate(&id).expect("locate the profile");

    assert_eq!(
        removal.folder(&id),
        directories
            .data
            .parent()
            .expect("the data directory has a parent")
            .to_owned(),
    );
}

// Read-only-directory permissions are a Unix concept; Windows' ACL model has
// no equivalent `set_mode` call (roadmap item 12, "carried over" note).
#[cfg(unix)]
#[test]
fn removing_a_folder_inside_a_read_only_directory_returns_err_with_a_single_line_display() {
    let dir = TempDir::new("profile-removal-read-only");
    let locator = XdgProfileLocator::under(dir.path());
    let removal = XdgProfileRemoval::under(dir.path());
    let id = SessionId::new("session-0001");
    locator.locate(&id).expect("locate the profile");
    let profiles_dir = dir.join("profiles");
    set_mode(&profiles_dir, 0o555);

    let error = removal
        .remove(&id)
        .expect_err("a read-only parent refuses removal");

    set_mode(&profiles_dir, 0o755);
    let display = error.to_string();
    assert!(
        !display.contains('\n'),
        "Display must be a single line: {display}"
    );
}

#[cfg(unix)]
fn set_mode(path: &std::path::Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).expect("set permissions");
}
