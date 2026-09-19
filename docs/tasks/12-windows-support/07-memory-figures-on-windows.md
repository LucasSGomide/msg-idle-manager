# 07 — Memory figures on Windows

**Roadmap:** [12](../../roadmap/12-windows-support/README.md) · **Scope:** back-end · **Depends on:** 02

## Context

The bottom of the application's sidebar shows how much memory the application
is using. It is the number the whole project is judged by, since the point is
to run many games more cheaply than a browser would. On Linux that number comes
from a system folder that describes every running process and how much memory
each one uses. Windows has no such folder, so on Windows the figure cannot be
read at all yet.

This slice adds a Windows reader that produces the same kind of figure. It takes
one snapshot of every process on the machine and notes which process started
which. From that it finds the application and every process the application
started, directly or indirectly. That includes the web engine's browser,
graphics, helper and page processes, because the engine is started by the
application. For each of those processes it asks Windows how much memory
belongs to that process alone and adds the amounts up. Some older Windows 10
builds do not report that measure, and for them it uses the closest one Windows
does report. A process that exits during the count is skipped, as on Linux.

The logic that finds the application's descendants already exists for Linux.
It moves into a shared piece both readers use, so its existing tests keep
covering it. The adding-up logic is written against plain data, so it can be
tested on the Linux development machine. Only the few system calls are
Windows-only. The part of the program that chooses which reader to build picks
the Windows one on Windows. The sidebar's display of the figure does not change.

This slice is separate because it touches only the memory crate and one line
of the startup code. It can be built alongside the interface slices without
conflicting with them.

## Technical details

- **Architecture** — move the pure `descend_from` walk from
  `crates/idle-manager-metrics/src/proc_pss.rs:325` into a new target-neutral
  `tree.rs`, with its tests. `proc_pss.rs` calls it, and `proc_pss.rs` itself
  is compiled only on `cfg(target_os = "linux")`.
- **Architecture** — add `process_tree.rs` under `cfg(windows)` defining
  `ProcessTreeProbe`, which implements the core's existing `MemoryProbe` port
  and returns the same `MemoryReading` (architecture rule 5, naming rule 10).
  `sample()`:
  - takes one `CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS)`;
  - builds the parent table and calls `tree::descend_from(own_pid)`;
  - for each process id opens `PROCESS_QUERY_LIMITED_INFORMATION |
    PROCESS_VM_READ` and reads `K32GetProcessMemoryInfo` into
    `PROCESS_MEMORY_COUNTERS_EX2`;
  - sums `PrivateWorkingSetSize`, or `PrivateUsage` when that reads 0;
  - skips a process that cannot be opened because it has exited.
- **Architecture** — the summing and fallback are a pure
  `fn private_bytes(counters: &Counters) -> u64` over a plain struct, plus
  `fn reading_from(own: u64, descendants: &[u64]) -> MemoryReading`. Both are
  compiled on every target and unit-tested on Linux (architecture rule 14).
- **Code standards** — `process_tree/ffi.rs` is the metrics crate's one
  `#[allow(unsafe_code)]` module, with a `// SAFETY:` line on every `unsafe`
  block, handles closed on drop, and only safe functions exposed (rule 28 as
  amended). Errors map to the existing `MemoryProbeError::Refused` or
  `Unreadable` through a `thiserror` enum (rule 12).
- **Architecture** — `crates/idle-manager-metrics/Cargo.toml` takes `windows`
  under `[target.'cfg(windows)'.dependencies]` with the features
  `Win32_System_Diagnostics_ToolHelp`, `Win32_System_ProcessStatus`,
  `Win32_System_Threading` and `Win32_Foundation`.
  `crates/idle-manager/src/main.rs` builds `ProcessTreeProbe` on Windows and
  `ProcPssProbe` on Linux (architecture rule 3). Sampling stays on
  `gio::spawn_blocking`, since the port is `Send + Sync` (architecture rule 10).
- **Naming** — file names are snake_case module files (rule 2). The type is
  named for its mechanism as an adapter, `ProcessTreeProbe`, and repeats no
  module name (rules 6, 10).

## Acceptance criteria

- [x] `(unit)` `tree::descend_from` keeps every existing case from
      `proc_pss.rs` passing after the move
- [x] `(unit)` `private_bytes` returns `PrivateWorkingSetSize` when it is
      non-zero and `PrivateUsage` when it is zero
- [x] `(unit)` `reading_from` puts the application's own figure and the sum of
      its descendants into the same `MemoryReading` fields `ProcPssProbe` fills,
      with the process count including the application itself
- [x] `(unit)` a descendant list with one process missing (exited mid-walk)
      yields a reading from the rest, not an error
- [x] `(integration)` `make windows-check` and `make verify` pass, and the
      Linux metrics integration tests pass unchanged
- [x] `(integration)` `make arch-check` still passes with `windows` in the
      metrics crate only
- [ ] `(manual)` in the Windows VM, with four live accounts, the footer's figure is
      within 5% of the sum of Task Manager's "Memory (private working set)"
      over `idle-manager.exe` and its `msedgewebview2.exe` children

## References

- [Roadmap item](../../roadmap/12-windows-support/README.md) — Back-end,
  `ProcessTreeProbe` paragraph and Technical References
- [`docs/requirements.md`](../../requirements.md) — Platform Support `FR.2.2`
- [`docs/architecture.md`](../../architecture.md) — rules 2, 3, 5, 10, 11, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 12, 21–25, 28
- [`docs/naming.md`](../../naming.md) — rules 2, 6, 10

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The `(manual)` step runs in the
Windows VM ([`docs/windows-vm.md`](../../windows-vm.md)).
