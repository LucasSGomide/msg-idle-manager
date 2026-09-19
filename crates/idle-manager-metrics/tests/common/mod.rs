//! A synthetic `/proc` on disk for the process-tree walk, removed on drop.
//!
//! The walk reads a live process table, which `cargo test` must never require
//! (code standards rule 25), so the tests build the table they need as files:
//! one directory per pid holding a `status` (for the `Name:`/`PPid:` walk) and
//! whichever of `smaps_rollup` / `smaps` the case under test calls for.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);

pub(crate) struct FakeProc {
    root: PathBuf,
}

impl FakeProc {
    pub(crate) fn new(tag: &str) -> Self {
        let serial = COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "idle-manager-metrics-{tag}-{}-{serial}",
            std::process::id()
        ));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(&root).expect("create the fake /proc root");
        Self { root }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    /// Adds a process: its `status` naming `command` and `parent`, and a
    /// `smaps_rollup` reporting `pss_kib`.
    pub(crate) fn process(&self, pid: u32, command: &str, parent: u32, pss_kib: u64) -> &Self {
        self.write_status(pid, command, parent);
        self.write(
            pid,
            "smaps_rollup",
            &format!("Rss:  {} kB\nPss:  {pss_kib} kB\n", pss_kib * 2),
        );
        self
    }

    /// Adds a process whose `smaps_rollup` is absent, so the probe must fall
    /// back to summing the `smaps` text given.
    pub(crate) fn process_smaps_only(
        &self,
        pid: u32,
        command: &str,
        parent: u32,
        smaps: &str,
    ) -> &Self {
        self.write_status(pid, command, parent);
        self.write(pid, "smaps", smaps);
        self
    }

    /// Adds a process that is in the table but has no memory files at all —
    /// what a process that exits between the walk and the read looks like.
    pub(crate) fn process_gone(&self, pid: u32, command: &str, parent: u32) -> &Self {
        self.write_status(pid, command, parent);
        self
    }

    /// Adds a zombie: exited but not yet reaped, so its `status` is still
    /// there — `State: Z` — while the kernel refuses its `smaps_rollup` with
    /// `ESRCH` and serves its `smaps` as an empty file. Captured against a live
    /// one (`xdg-terminal-ex`, pid 306036, 2026-09-19); the refused rollup is
    /// modelled as absent, which the probe treats the same way.
    pub(crate) fn process_zombie(&self, pid: u32, command: &str, parent: u32) -> &Self {
        self.write(
            pid,
            "status",
            &format!("Name:\t{command}\nState:\tZ (zombie)\nPPid:\t{parent}\n"),
        );
        self.write(pid, "smaps", "");
        self
    }

    /// Adds a process whose `smaps` is there but cannot be read for a reason
    /// other than being absent. It is made a directory so the read fails with
    /// `EISDIR` whoever runs the tests; a mode-000 file would still read as
    /// root.
    pub(crate) fn process_with_unreadable_smaps(
        &self,
        pid: u32,
        command: &str,
        parent: u32,
    ) -> &Self {
        self.write_status(pid, command, parent);
        fs::create_dir_all(self.root.join(pid.to_string()).join("smaps"))
            .expect("create the directory standing in for smaps");
        self
    }

    /// Adds a process with the given raw `smaps_rollup` contents.
    pub(crate) fn process_with_rollup(
        &self,
        pid: u32,
        command: &str,
        parent: u32,
        rollup: &str,
    ) -> &Self {
        self.write_status(pid, command, parent);
        self.write(pid, "smaps_rollup", rollup);
        self
    }

    fn write_status(&self, pid: u32, command: &str, parent: u32) {
        self.write(
            pid,
            "status",
            &format!("Name:\t{command}\nState:\tS (sleeping)\nPPid:\t{parent}\n"),
        );
    }

    fn write(&self, pid: u32, file: &str, contents: &str) {
        let dir = self.root.join(pid.to_string());
        fs::create_dir_all(&dir).expect("create the pid directory");
        fs::write(dir.join(file), contents).expect("write a fake /proc file");
    }
}

impl Drop for FakeProc {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).ok();
    }
}

pub(crate) fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read fixture {name}: {error}"))
}
