//! `XdgProfileLocator` creates isolated, stable, XDG-rooted profile
//! directories, and reports an unwritable root distinctly from an unusable one.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use idle_manager_core::{ProfileError, ProfileLocator, SessionId};
use idle_manager_store::XdgProfileLocator;

static COUNTER: AtomicU32 = AtomicU32::new(0);

struct TempRoot {
    path: PathBuf,
}

impl TempRoot {
    fn new(tag: &str) -> Self {
        let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "idle-manager-{tag}-{}-{serial}",
            std::process::id()
        ));
        fs::remove_dir_all(&path).ok();
        fs::create_dir_all(&path).expect("create the temporary data root");
        Self { path }
    }
}

impl Drop for TempRoot {
    fn drop(&mut self) {
        #[cfg(unix)]
        restore_writable(&self.path);
        fs::remove_dir_all(&self.path).ok();
    }
}

#[cfg(unix)]
fn restore_writable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    if let Ok(metadata) = fs::metadata(path) {
        let mut perms = metadata.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).ok();
    }
}

#[test]
fn locating_a_new_identifier_creates_a_data_and_a_cache_directory() {
    let root = TempRoot::new("new");
    let locator = XdgProfileLocator::under(&root.path);

    let dirs = locator
        .locate(&SessionId::new("session-0001"))
        .expect("locate the new profile");

    assert!(dirs.data.is_dir(), "the data directory exists");
    assert!(dirs.cache.is_dir(), "the cache directory exists");
}

#[test]
fn locating_twice_for_one_identifier_returns_the_same_paths() {
    let root = TempRoot::new("stable");
    let locator = XdgProfileLocator::under(&root.path);
    let id = SessionId::new("session-0007");

    let first = locator.locate(&id).expect("first locate");
    let second = locator.locate(&id).expect("second locate");

    assert_eq!(first, second);
}

#[test]
fn two_identifiers_get_two_different_data_directories() {
    let root = TempRoot::new("distinct");
    let locator = XdgProfileLocator::under(&root.path);

    let one = locator
        .locate(&SessionId::new("session-0001"))
        .expect("one");
    let two = locator
        .locate(&SessionId::new("session-0002"))
        .expect("two");

    assert_ne!(one.data, two.data);
}

#[test]
fn every_returned_path_is_inside_the_data_root() {
    let root = TempRoot::new("rooted");
    let locator = XdgProfileLocator::under(&root.path);

    let dirs = locator
        .locate(&SessionId::new("session-0003"))
        .expect("locate");

    assert!(dirs.data.starts_with(&root.path) && dirs.cache.starts_with(&root.path));
}

#[test]
#[cfg(unix)]
fn a_data_root_that_cannot_be_written_is_reported_as_not_writable() {
    use std::os::unix::fs::PermissionsExt;

    let root = TempRoot::new("readonly");
    let locator = XdgProfileLocator::under(&root.path);
    let mut perms = fs::metadata(&root.path).expect("metadata").permissions();
    perms.set_mode(0o555);
    fs::set_permissions(&root.path, perms).expect("make the root read-only");

    let result = locator.locate(&SessionId::new("session-0004"));

    assert!(matches!(result, Err(ProfileError::NotWritable { .. })));
}

#[test]
fn a_path_blocked_by_a_file_is_reported_as_not_created() {
    let root = TempRoot::new("blocked");
    fs::write(root.path.join("profiles"), b"not a directory").expect("write the blocker");
    let locator = XdgProfileLocator::under(&root.path);

    let result = locator.locate(&SessionId::new("session-0005"));

    assert!(matches!(result, Err(ProfileError::NotCreated { .. })));
}
