# 03 — The memory report script and its soak mode

**Roadmap:** [05](../../roadmap/05-memory-accounting/README.md) · **Scope:** tooling · **Depends on:** —

## Context

Before anything in this item can be believed, something outside the application
has to be able to check it. This slice builds that: one script that finds the
running application, walks everything it started, and prints what each process
costs, with the same total the readout will later show on screen.

It comes first among the measuring work because everything else needs it. Task
02's parser needs captured kernel output to test against, and this is what
captures it. Tasks 06 and 07 each claim to make the application cheaper, and a
claim like that is worth nothing without a before and an after taken the same
way. Task 05's soak is this script left running. And when the readout and the
script disagree, the script is the one that is right, because it is the simpler
of the two and the one a person can read the source of in a minute.

Two modes, because there are two questions. Asked once, it answers "what is it
costing right now, and which process is holding it" — a line per process with
its identifier, its command name, its proportional figure and its resident
figure side by side, so the gap between them is visible rather than asserted.
Asked to soak, it answers "and is that figure going anywhere" — the same total
appended to a file on an interval, for as long as it is left running, in a shape
a spreadsheet or a plot can read without editing.

The resident figure is printed next to the proportional one on purpose. This
project has an opinion about which of the two to trust, that opinion is the
reason the readout exists at all, and a person who has only ever seen the number
their system monitor shows deserves to see both numbers together once rather
than be told the familiar one is wrong.

Finding the processes is the part that is easy to get wrong and this script must
get right, because it is the reference. The engine starts a rendering process
inside two nested sandboxes, so a script that listed direct children would print
the networking process and a couple of sandbox helpers and miss five sixths of
the memory. It reads the process table once, builds the parent map, and walks
down (`FR.19.1`).

## Technical details

- **Architecture** — every developer command is a Makefile target: `make
  memory-report` runs it, per CLAUDE.md, and the script itself lives in
  `scripts/` beside `arch-check.sh` and the two session-forensics scripts it can
  borrow its shape from.
- **Naming** — rule 1: `scripts/memory-report.sh`, kebab-case like every other
  file that is not a Rust module.
- **Back-end** — the walk reads `/proc/*/stat` (or `/proc/*/status`) once for
  every process, builds child lists by parent identifier, and walks from the
  application's own identifier. Never a match on direct children only, and never
  a match on command name to find the tree — a name match would collect another
  user's browser.
- **Back-end** — the per-process figure is `Pss` from `/proc/<pid>/smaps_rollup`,
  falling back to summing `Pss` across `/proc/<pid>/smaps` where the rollup is
  absent, which is the same pair of sources task 02's parser reads. The two
  implementations agreeing is one of this item's runbook checks.
- **Back-end** — a process that vanishes between the walk and the read is
  normal, not an error: it is skipped with a note and the total is still
  printed. A script that aborted because a page finished loading would be
  useless during exactly the transitions worth measuring.
- **Back-end** — the application is located by its executable name with an
  option to pass an identifier explicitly, because a developer with a debug and
  a release build running at once needs to say which.
- **Back-end** — soak mode takes an interval and an output path and appends one
  tab-separated row per sample: timestamp, the application's own figure, the
  descendants' figure, the total, the process count. Tab-separated because it is
  the shape both a spreadsheet and `awk` read without being told anything.
- **Code standards** — rule 5 applies to the script too: the default interval
  and the default unit are named at the top with their units in the name, not
  buried as literals in the loop.
- **Testing** — this slice has no automated test. It reads a live process tree,
  which `cargo test` must never require (code standards rule 25), so its
  evidence is `test-script.md` — and it is the slice that writes that file's
  `## Setup` and `## Teardown` sections, being the first here to reach
  acceptance.

## Acceptance criteria

- [ ] `(manual)` with the application running, `make memory-report` prints one
      line per process with identifier, command name, proportional and resident
      figures, and a total
- [ ] `(manual)` the printed tree includes the rendering process, which is a
      grandchild through two `bwrap` processes, and not only the direct children
- [ ] `(manual)` the total proportional figure is materially lower than the
      total resident figure, and both are printed so the gap can be seen
- [ ] `(manual)` parking every account but one drops the process count and the
      total, and the run before and the run after are both recorded in the
      runbook
- [ ] `(manual)` soak mode appends a tab-separated row per interval to the given
      file and keeps going across a page load, a park and an unpark
- [ ] `(manual)` a process that exits mid-walk is skipped with a note and the
      run still prints a total
- [ ] `(manual)` run with no application started, the script says so and exits
      without printing a total

## References

- [Roadmap item](../../roadmap/05-memory-accounting/README.md) — the measured
  tree on 2026-09-08 under Technical References
- [`docs/requirements.md`](../../requirements.md) — `FR.7.2`, `FR.19.1`,
  `FR.19.3`
- [`docs/architecture.md`](../../architecture.md) — rule 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 25
- [`docs/naming.md`](../../naming.md) — rule 1

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
