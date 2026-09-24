//! A throwaway directory for the update crate's integration tests, removed
//! on drop — the same shape as `idle-manager-store`'s own `tests/common`.

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
            "idle-manager-update-{tag}-{}-{serial}",
            std::process::id()
        ));
        fs::remove_dir_all(&path).ok();
        fs::create_dir_all(&path).expect("create the temporary directory");
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) fn join(&self, relative: &str) -> PathBuf {
        self.path.join(relative)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).ok();
    }
}
