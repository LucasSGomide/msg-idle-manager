# 02 — Walking the process tree and reading its proportional memory

**Roadmap:** [05](../../roadmap/05-memory-accounting/README.md) · **Scope:** back-end · **Depends on:** 01, 03

## Context

This is the slice that actually reads the kernel, and it is the one the item's
naming rule was written for: the domain asked for a `MemoryProbe` and this
supplies a `ProcPssProbe`, a name that says `/proc` and says proportional set
size and says nothing about sessions.

Two jobs, and the harder one is not the parsing. Finding the processes is what
this slice has to get right. The engine does not start a rendering process as a
child of the application — it starts it inside two nested sandbox processes, so
the process holding most of the application's memory is a grandchild. The
version of this plan written before anything was measured said to match direct
children against our own identifier, and that would have found the networking
process, missed the rendering process entirely, and reported an application
costing a sixth of what it costs. So the probe reads every entry in the process
table once, builds a map from parent to children, and walks down from itself
(`FR.19.1`). Everything found that way is counted, whether or not its name is
recognised.

Recognising names is the second, softer job. A breakdown by kind — rendering,
networking, sandbox helper, unrecognised — is what makes a figure diagnosable
rather than merely large, and it is deliberately not what makes it correct. An
engine that renames its processes in a future version should cost this crate its
labels, never its total, and the difference is why the walk decides membership
and the name decides only the label.

The parsing is the ordinary half. The kernel offers a rolled-up figure per
process in one short read, and where that file is withheld — a container, a
hardened kernel — the same number can be summed out of the much longer
per-mapping file. Both paths are written and both are tested against captured
output, because a fallback nobody has run is a fallback that does not work.

Every failure here has to keep its shape. "The system will not let us look" and
"the system showed us something we could not read" send the same unavailable
state to the footer but mean opposite things to whoever is debugging it, and a
crate that flattened them into one error would make the log useless at the one
moment it is needed.

## Technical details

- **Architecture** — rule 5 and **naming** rule 10: `ProcPssProbe` in
  `crates/idle-manager-metrics/src/proc_pss.rs`, implementing the core's
  `MemoryProbe` from task 01. Rule 2 binds: this crate depends on the core and
  on nothing else in the workspace.
- **Architecture** — rule 3: the binary constructs it and hands it to the shell.
  The shell must not name this crate and `scripts/arch-check.sh` already forbids
  that edge — no change to the script is needed, but the build failing if
  someone tries is the point.
- **Back-end** — the walk is one pass over `/proc` collecting identifier and
  parent identifier, then a breadth-first descent from our own. Reading the table
  once rather than once per level is what keeps the cost bounded on a machine
  with a thousand processes, which the sampling interval blocker cares about.
- **Back-end** — a process that exits between the walk and the read is skipped,
  not fatal. The count in the reading reports how many were actually read, so a
  caller can see that a sample was thin (task 01).
- **Back-end** — the figure comes from `smaps_rollup`'s `Pss` line, falling back
  to summing `Pss` over `smaps`. The fallback is selected by the rollup being
  absent or unreadable, never by parsing it and finding nothing — an empty parse
  is a parse failure and says so.
- **Architecture** — rule 11, **code standards** rule 12: a `thiserror` enum for
  this crate separating a refusal (the file is not there, or permission was
  denied) from a malformed read (it was there and did not parse), each carrying
  the path it was reading.
- **Code standards** — rules 13 and 14: no `unwrap` outside tests and no
  discarded `Result`; a skipped process is logged with `tracing` in fields
  naming the identifier and the reason (rule 15).
- **Architecture** — rule 14, **naming** rule 3: integration tests under
  `crates/idle-manager-metrics/tests/`, kebab-case after the behaviour, reading
  captured kernel output from `tests/fixtures/` — the folder
  `docs/architecture.md` already anticipates. The fixture is captured with task
  03's script from a real run with a game loaded, sandboxes and all, so the tree
  under test is the tree that exists.
- **Testing** — the tree walk is tested from a fixture process table, not from
  the live machine, so the grandchild case is pinned by a test rather than by
  somebody remembering to look (code standards rule 25).

## Acceptance criteria

- [x] `(integration)` a captured process table containing the two nested sandbox
      processes yields the rendering process among the descendants
- [x] `(integration)` a process table where the application has no descendants
      yields a reading with the application's own figure and a descendant figure
      of zero, not an error
- [x] `(integration)` an unrelated process tree belonging to another identifier
      is not counted
- [x] `(integration)` a captured `smaps_rollup` parses to the same figure as
      summing `Pss` across the matching captured `smaps`
- [x] `(integration)` a process whose `smaps_rollup` is absent falls back to
      `smaps` and produces a figure
- [x] `(integration)` a `smaps_rollup` with no `Pss` line at all is a malformed
      read carrying the path, and is distinguishable by matching from a file
      that was not there
- [x] `(integration)` a descendant whose command name matches no known kind is
      still counted in the total and labelled unrecognised
- [x] `(integration)` the reading's process count equals the number of processes
      actually read, not the number found in the walk

## References

- [Roadmap item](../../roadmap/05-memory-accounting/README.md) — the measured
  tree and the two kernel files under Technical References
- [`docs/requirements.md`](../../requirements.md) — `FR.7.1`, `FR.19.1`
- [`docs/architecture.md`](../../architecture.md) — rules 2, 3, 5, 11, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 12, 13, 14, 15,
  17, 21, 23, 25
- [`docs/naming.md`](../../naming.md) — rules 2, 3, 5, 10
- [`crates/idle-manager-metrics/src/lib.rs`](../../../crates/idle-manager-metrics/src/lib.rs)
  — the crate's existing note on why proportional rather than resident

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
