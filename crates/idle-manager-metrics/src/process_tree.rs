//! `ProcessTreeProbe`: the [`MemoryProbe`] port fulfilled on Windows, where
//! there is no `/proc` to read.
//!
//! The walk itself is the same shape as [`crate::proc_pss::ProcPssProbe`]'s:
//! read the whole process table once, build a parent map, and descend from
//! the application's own process id (`tree::descend_from`, shared with the
//! Linux probe). What differs is the one Windows API `process_tree/ffi.rs`
//! calls per process — `K32GetProcessMemoryInfo` in place of a `smaps_rollup`
//! read — and the two figures it reports.
//!
//! [`Counters`], [`private_bytes`] and [`reading_from`] are plain data and
//! arithmetic, unit-tested on Linux (architecture rule 14) even though
//! nothing on Linux calls them for real — `cfg(any(windows, test))` compiles
//! them exactly there, rather than in every build, so a Linux release build
//! never carries dead Windows-only plumbing. Only [`ProcessTreeProbe`]
//! itself, which calls the Windows API through `ffi.rs`, is `cfg(windows)`
//! alone.

#[cfg(windows)]
mod ffi;

#[cfg(any(windows, test))]
use idle_manager_core::MemoryReading;

/// The two memory figures `K32GetProcessMemoryInfo` reports that matter here,
/// copied out of `PROCESS_MEMORY_COUNTERS_EX2` into a plain struct so
/// [`private_bytes`] can be unit-tested without the Windows API it came from.
#[cfg(any(windows, test))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Counters {
    /// `PROCESS_MEMORY_COUNTERS_EX2::PrivateWorkingSetSize`.
    pub(crate) private_working_set: u64,
    /// `PROCESS_MEMORY_COUNTERS_EX2::PrivateUsage`.
    pub(crate) private_usage: u64,
}

/// `PrivateWorkingSetSize`, in bytes, or `PrivateUsage` when the working-set
/// figure reads zero.
///
/// Some Windows 10 builds predate `PROCESS_MEMORY_COUNTERS_EX2`'s working-set
/// field and always report `0` there (`FR.2.2`); `PrivateUsage` is the closest
/// figure those builds do report, so it is never treated as "the process
/// really uses nothing."
#[cfg(any(windows, test))]
pub(crate) fn private_bytes(counters: &Counters) -> u64 {
    if counters.private_working_set != 0 {
        counters.private_working_set
    } else {
        counters.private_usage
    }
}

/// Builds the [`MemoryReading`] [`crate::MemoryProbe::sample`] returns from
/// the application's own figure and its descendants', both in bytes — the
/// same two facts `ProcPssProbe`'s `ProcessTreeReading::reading` fills from a
/// `/proc` walk, converted to kibibytes at this one boundary (`FR.7.1`). The
/// process count includes the application itself.
#[cfg(any(windows, test))]
pub(crate) fn reading_from(own_bytes: u64, descendant_bytes: &[u64]) -> MemoryReading {
    MemoryReading {
        own_kib: own_bytes / 1024,
        descendants_kib: descendant_bytes.iter().sum::<u64>() / 1024,
        process_count: 1 + descendant_bytes.len(),
    }
}

/// Reduces one walk's raw reads to the figures that actually contributed,
/// skipping a `None` — a process that exited between the snapshot and the
/// read — rather than failing the whole reading (code standards rule 14).
/// Pure so it is unit-testable on Linux, same reasoning as [`private_bytes`]
/// and [`reading_from`].
#[cfg(any(windows, test))]
pub(crate) fn collected(reads: impl IntoIterator<Item = Option<u64>>) -> Vec<u64> {
    reads.into_iter().flatten().collect()
}

#[cfg(windows)]
mod windows_probe {
    use std::collections::HashMap;

    use idle_manager_core::{MemoryProbe, MemoryProbeError, MemoryReading};

    use super::{collected, ffi, private_bytes, reading_from};
    use crate::tree::{self, ProcessEntry};

    /// Reads private memory for the application's own process tree through
    /// one `CreateToolhelp32Snapshot` and a `K32GetProcessMemoryInfo` call per
    /// descendant.
    ///
    /// Built with [`ProcessTreeProbe::new`] against this process's own
    /// identifier.
    #[derive(Debug, Clone)]
    pub struct ProcessTreeProbe {
        own_pid: u32,
    }

    impl ProcessTreeProbe {
        /// Reads the live process table for this process's own tree.
        #[must_use]
        pub fn new() -> Self {
            Self {
                own_pid: std::process::id(),
            }
        }
    }

    impl Default for ProcessTreeProbe {
        fn default() -> Self {
            Self::new()
        }
    }

    impl MemoryProbe for ProcessTreeProbe {
        fn sample(&self) -> Result<MemoryReading, MemoryProbeError> {
            let snapshot = ffi::snapshot_processes()?;
            let table: HashMap<u32, ProcessEntry> = snapshot
                .into_iter()
                .map(|entry| {
                    (
                        entry.pid,
                        ProcessEntry {
                            command: entry.command,
                            parent: entry.parent,
                        },
                    )
                })
                .collect();
            let order = tree::descend_from(self.own_pid, &table);

            let mut reads = Vec::with_capacity(order.len());
            for pid in order {
                // Named for the same reason `ProcPssProbe`'s `ProcessKind`
                // is: a figure alone is not diagnosable, and which command
                // contributed it is.
                let command = table.get(&pid).map_or("", |entry| entry.command.as_str());
                let counters = ffi::process_memory(pid)?;
                match &counters {
                    Some(counters) => {
                        tracing::trace!(
                            pid,
                            command,
                            bytes = private_bytes(counters),
                            "process read"
                        );
                    }
                    // A process that exits between the snapshot and this read
                    // is normal, not fatal, same as the `/proc` walk (code
                    // standards rule 14).
                    None => {
                        tracing::debug!(
                            pid,
                            command,
                            reason = "exited during the walk",
                            "process skipped"
                        );
                    }
                }
                reads.push(counters.as_ref().map(private_bytes));
            }
            let read = collected(reads);

            let Some((&own, descendants)) = read.split_first() else {
                return Err(MemoryProbeError::Refused {
                    reason: format!("own process {} vanished during the walk", self.own_pid),
                });
            };
            Ok(reading_from(own, descendants))
        }
    }
}

#[cfg(windows)]
pub use windows_probe::ProcessTreeProbe;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_bytes_prefers_the_working_set_figure_when_it_is_non_zero() {
        let counters = Counters {
            private_working_set: 4096,
            private_usage: 8192,
        };

        assert_eq!(private_bytes(&counters), 4096);
    }

    #[test]
    fn private_bytes_falls_back_to_private_usage_when_the_working_set_figure_is_zero() {
        let counters = Counters {
            private_working_set: 0,
            private_usage: 8192,
        };

        assert_eq!(private_bytes(&counters), 8192);
    }

    #[test]
    fn reading_from_sums_descendants_and_counts_the_application_itself() {
        let reading = reading_from(2048 * 1024, &[1024 * 1024, 512 * 1024]);

        assert_eq!(
            (
                reading.own_kib,
                reading.descendants_kib,
                reading.process_count
            ),
            (2048, 1536, 3),
        );
    }

    #[test]
    fn reading_from_with_no_descendants_still_reports_the_application_itself() {
        let reading = reading_from(1024 * 1024, &[]);

        assert_eq!((reading.descendants_kib, reading.process_count), (0, 1));
    }

    #[test]
    fn collected_skips_a_process_missing_mid_walk_instead_of_failing_the_reading() {
        let reads = [Some(2048 * 1024), None, Some(512 * 1024)];

        assert_eq!(collected(reads), vec![2048 * 1024, 512 * 1024]);
    }
}
