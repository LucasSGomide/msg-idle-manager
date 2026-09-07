//! A throwaway directory for the store's integration tests, removed on drop.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

pub(crate) struct TempDir {
    path: PathBuf,
}

impl TempDir {
    pub(crate) fn new(tag: &str) -> Self {
        let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "idle-manager-{tag}-{}-{serial}",
            std::process::id()
        ));
        fs::remove_dir_all(&path).ok();
        fs::create_dir_all(&path).expect("create the temporary directory");
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// A path inside this directory. The component may itself contain
    /// separators, so a caller can name a nested location that does not exist
    /// yet.
    pub(crate) fn join(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }

    pub(crate) fn write(&self, name: &str, contents: &str) {
        fs::write(self.path.join(name), contents).expect("write a fixture file");
    }

    /// Drops write permission on the directory itself, so creating a child of
    /// it fails. Restored on drop so the directory can still be removed.
    #[cfg(unix)]
    pub(crate) fn make_readonly(&self) {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(&self.path).expect("metadata").permissions();
        perms.set_mode(0o555);
        fs::set_permissions(&self.path, perms).expect("make the directory read-only");
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if let Ok(metadata) = fs::metadata(&self.path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&self.path, perms).ok();
            }
        }
        fs::remove_dir_all(&self.path).ok();
    }
}
