//! `ProcPssProbe`: the [`MemoryProbe`] port fulfilled by reading `/proc`.
//!
//! The name says `/proc`, says proportional set size, and says nothing about
//! sessions (naming rule 10). Two jobs live here and the harder one is not the
//! parsing: `WebKitGTK` starts each rendering process inside two nested `bwrap`
//! sandboxes, so the process holding most of the memory is a *grandchild* of
//! the application (`FR.19.1`). The probe reads the whole process table once,
//! builds a parent-to-child map, and walks down from its own identifier —
//! everything found that way is counted, whether or not its command name is
//! recognised.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use idle_manager_core::{MemoryProbe, MemoryProbeError, MemoryReading};

use crate::tree::{self, ProcessEntry};

/// Where the process table lives on a running system. Overridden in tests with
/// [`ProcPssProbe::under`], which is the whole reason the walk takes a root
/// rather than hard-coding this.
const PROC_ROOT: &str = "/proc";

/// A reading could not be taken from `/proc`.
///
/// Separates a refusal — a file that is not there, or permission denied — from
/// a malformed read — a file that was there and did not parse — and each
/// carries the path it was reading (architecture rule 11, code standards
/// rule 12). The shell shows the same unavailable footer for both; the log
/// tells them apart.
#[derive(Debug, thiserror::Error)]
pub enum ProcPssError {
    /// A `/proc` file could not be read at all.
    #[error("{path} could not be read: {source}", path = .path.display())]
    Refused {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },
    /// A `/proc` file was read but its contents did not parse — most often a
    /// `smaps_rollup` with no `Pss:` line at all.
    #[error("{path} was read but did not parse: {reason}", path = .path.display())]
    Malformed {
        /// The path whose contents would not parse.
        path: PathBuf,
        /// One line describing what was wrong.
        reason: String,
    },
}

impl From<ProcPssError> for MemoryProbeError {
    fn from(error: ProcPssError) -> Self {
        match error {
            ProcPssError::Refused { path, source } => MemoryProbeError::Refused {
                reason: format!("{}: {source}", path.display()),
            },
            ProcPssError::Malformed { path, reason } => MemoryProbeError::Unreadable {
                reason: format!("{}: {reason}", path.display()),
            },
        }
    }
}

/// What kind of process a descendant is, by its command name.
///
/// The label, never the membership: an engine that renames its processes in a
/// future version should cost this crate its labels, not its total, so the walk
/// decides what is counted and the name decides only how it is described.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessKind {
    /// A `WebKit` rendering process — the one holding most of the memory.
    Rendering,
    /// The single `WebKit` networking process.
    Networking,
    /// A sandbox or bus-proxy helper the engine starts around a web process.
    SandboxHelper,
    /// A descendant whose command name matches none of the above. Still counted
    /// in the total (`FR.19.1`); only its label is unknown.
    Unrecognised,
}

impl ProcessKind {
    fn of(command: &str) -> Self {
        match command {
            "WebKitWebProcess" => ProcessKind::Rendering,
            "WebKitNetworkProcess" => ProcessKind::Networking,
            "bwrap" | "xdg-dbus-proxy" => ProcessKind::SandboxHelper,
            _ => ProcessKind::Unrecognised,
        }
    }
}

/// One process's contribution to a reading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessMemory {
    /// The process identifier.
    pub pid: u32,
    /// The command name, as `/proc/<pid>/status` reports it.
    pub command: String,
    /// What kind of process this is, by its command name.
    pub kind: ProcessKind,
    /// Its proportional set size, in kibibytes.
    pub pss_kib: u64,
}

/// Everything one walk produced: the application's own process and every
/// descendant that was still alive to be read.
///
/// A breakdown by kind is what makes a figure diagnosable rather than merely
/// large. [`ProcessTreeReading::reading`] collapses it to the three facts the
/// [`MemoryReading`] the port returns carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessTreeReading {
    /// The application's own process.
    pub own: ProcessMemory,
    /// Every descendant that was read, in breadth-first order from `own`.
    pub descendants: Vec<ProcessMemory>,
}

impl ProcessTreeReading {
    /// The three facts the domain wants: the own figure, the descendants'
    /// figure summed, and the count of processes that contributed — the number
    /// actually read, not the number found in the walk.
    #[must_use]
    pub fn reading(&self) -> MemoryReading {
        MemoryReading {
            own_kib: self.own.pss_kib,
            descendants_kib: self.descendants.iter().map(|process| process.pss_kib).sum(),
            process_count: 1 + self.descendants.len(),
        }
    }
}

/// Reads proportional set size for the application's own process tree from
/// `/proc`.
///
/// Built with [`ProcPssProbe::new`] against the real `/proc` and this process's
/// own identifier, or [`ProcPssProbe::under`] against a captured process table
/// for tests.
#[derive(Debug, Clone)]
pub struct ProcPssProbe {
    proc_root: PathBuf,
    own_pid: u32,
}

impl ProcPssProbe {
    /// Reads the live `/proc` for this process's own tree.
    #[must_use]
    pub fn new() -> Self {
        Self {
            proc_root: PathBuf::from(PROC_ROOT),
            own_pid: std::process::id(),
        }
    }

    /// Roots the walk at a captured process table with `own_pid` as the tree's
    /// root, for tests that must not read the live machine.
    #[must_use]
    pub fn under(proc_root: impl AsRef<Path>, own_pid: u32) -> Self {
        Self {
            proc_root: proc_root.as_ref().to_owned(),
            own_pid,
        }
    }

    /// Walks the process tree from `own_pid` and reads each process's `Pss`.
    ///
    /// # Errors
    ///
    /// [`ProcPssError::Refused`] if the process table or a needed file cannot
    /// be read, [`ProcPssError::Malformed`] if a `smaps_rollup` or `smaps` was
    /// read and held no `Pss:` line.
    pub fn read_tree(&self) -> Result<ProcessTreeReading, ProcPssError> {
        let table = self.read_process_table()?;
        let order = tree::descend_from(self.own_pid, &table);

        let mut read = Vec::with_capacity(order.len());
        for pid in order {
            let command = table
                .get(&pid)
                .map(|entry| entry.command.clone())
                .unwrap_or_default();
            match self.read_pss_kib(pid)? {
                Some(pss_kib) => read.push(ProcessMemory {
                    pid,
                    kind: ProcessKind::of(&command),
                    command,
                    pss_kib,
                }),
                None => {
                    tracing::debug!(pid, reason = "exited during the walk", "process skipped");
                }
            }
        }

        let mut read = read.into_iter();
        let own = read.next().ok_or_else(|| ProcPssError::Refused {
            path: self.pid_dir(self.own_pid),
            source: std::io::Error::from(std::io::ErrorKind::NotFound),
        })?;

        Ok(ProcessTreeReading {
            own,
            descendants: read.collect(),
        })
    }

    fn pid_dir(&self, pid: u32) -> PathBuf {
        self.proc_root.join(pid.to_string())
    }

    /// Reads every numeric entry in the process table once, collecting each
    /// process's command name and the parent it hangs off.
    fn read_process_table(&self) -> Result<HashMap<u32, ProcessEntry>, ProcPssError> {
        let entries = fs::read_dir(&self.proc_root).map_err(|source| ProcPssError::Refused {
            path: self.proc_root.clone(),
            source,
        })?;

        let mut table = HashMap::new();
        for entry in entries.flatten() {
            let Some(pid) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<u32>().ok())
            else {
                continue;
            };

            let status_path = entry.path().join("status");
            let status = match fs::read_to_string(&status_path) {
                Ok(status) => status,
                // A process that exits between the directory scan and this read
                // is normal, not fatal: skip it (code standards rule 14).
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(source) => {
                    return Err(ProcPssError::Refused {
                        path: status_path,
                        source,
                    });
                }
            };

            let Some(parsed) = ProcessEntry::parse(&status) else {
                continue;
            };
            table.insert(pid, parsed);
        }

        Ok(table)
    }

    /// The `Pss` for one process in kibibytes: the `Pss:` line of
    /// `smaps_rollup`, falling back to summing `Pss:` across `smaps` where the
    /// rollup is absent or unreadable.
    ///
    /// `Ok(None)` means the process vanished between the walk and this read —
    /// neither file is there. `Err(Malformed)` means a file *was* there and
    /// carried no `Pss:` line, which is a different thing entirely and says so.
    fn read_pss_kib(&self, pid: u32) -> Result<Option<u64>, ProcPssError> {
        let rollup_path = self.pid_dir(pid).join("smaps_rollup");
        // Absent or unreadable: the rollup is optional — a container or a
        // hardened kernel withholds it — so any failure here falls through to
        // the per-mapping file rather than aborting.
        if let Ok(text) = fs::read_to_string(&rollup_path) {
            return sum_pss_kib(&text).map(Some).ok_or(ProcPssError::Malformed {
                path: rollup_path,
                reason: "no Pss: line".to_owned(),
            });
        }

        let smaps_path = self.pid_dir(pid).join("smaps");
        match fs::read_to_string(&smaps_path) {
            Ok(text) => sum_pss_kib(&text).map(Some).ok_or(ProcPssError::Malformed {
                path: smaps_path,
                reason: "no Pss: line".to_owned(),
            }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(ProcPssError::Refused {
                path: smaps_path,
                source,
            }),
        }
    }
}

impl Default for ProcPssProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryProbe for ProcPssProbe {
    fn sample(&self) -> Result<MemoryReading, MemoryProbeError> {
        Ok(self.read_tree()?.reading())
    }
}

impl ProcessEntry {
    fn parse(status: &str) -> Option<Self> {
        let mut command = None;
        let mut parent = None;
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("Name:") {
                command = Some(rest.trim().to_owned());
            } else if let Some(rest) = line.strip_prefix("PPid:") {
                parent = rest.trim().parse::<u32>().ok();
            }
        }
        Some(Self {
            command: command?,
            parent: parent?,
        })
    }
}

/// Sums every `Pss:` line's kibibyte figure, or `None` when there is no such
/// line — an empty parse is a parse failure, never a zero.
fn sum_pss_kib(text: &str) -> Option<u64> {
    let mut total = None;
    for line in text.lines() {
        let Some(rest) = line.strip_prefix("Pss:") else {
            continue;
        };
        if let Some(kib) = rest
            .split_whitespace()
            .next()
            .and_then(|figure| figure.parse::<u64>().ok())
        {
            *total.get_or_insert(0) += kib;
        }
    }
    total
}
