# 05 — The soak, and whether it is a leak

**Roadmap:** [05](../../roadmap/05-memory-accounting/README.md) · **Scope:** measurement · **Depends on:** 03

## Context

One live account costs 751 MiB in its rendering process. Nobody knows whether
that is a number the game arrives at and stays at, or a number it was passing
through. Those are different defects with opposite fixes — a stable working set
is a budget problem answered by telling the engine it has a limit, and a curve
that never flattens is a leak answered by finding what holds the memory — and no
snapshot can tell them apart. This slice runs long enough to tell.

The twenty minutes already sampled show what makes the question hard. The
allocator heap did not move at all, staying at 138,616 KiB for the whole period,
while the process total swung between 715 and 787 MiB on a garbage collector's
sawtooth — a seventy-mebibyte swing — and its mean rose from 741 MiB over the
first third of the run to 749 MiB over the last. Eight mebibytes of drift under
a seventy-mebibyte sawtooth is a slope no honest reading can extract: it is a
quarter of a gigabyte a day if it is real and nothing at all if it is the
sawtooth's phase. Only a run long enough for the drift to outgrow the swing can
say which, and that is this slice.

So: at least four hours on one live account, sampled on an interval, with the
window left alone (`FR.19.3`). What comes out is a curve, and the curve is read
for one thing — does the floor of the sawtooth rise. Peaks rise and fall because
that is what collection does; the floor is memory the collector could not
reclaim, and a floor that keeps climbing after the first hour is the definition
of the leak this slice is looking for.

Two more runs make the answer usable rather than merely true. The same soak with
the account parked partway through says whether parking returns the memory in
full or leaves a residue — item 03's promise, checked over hours instead of
seconds. And a second game soaked the same way says whether what was found
belongs to the engine or to one game's page, which decides whether the rest of
this item can fix it at all.

The output is a written finding, in `docs/memory-budget.md`, and it is written
whichever way the answer comes out. A soak that found a flat curve is not a
wasted afternoon: it is the evidence that lets task 06 choose a limit
confidently and lets task 08 write a budget that is a budget rather than a hope.

This slice runs early and takes wall-clock time rather than working time, which
is why it starts as soon as task 03's script exists and does not wait for the
readout. Its answers are what tasks 06 and 08 are blocked on.

## Technical details

- **Measurement** — at least four hours, one live account, one game, sampled by
  task 03's soak mode to a file. The window is left alone for the duration; a
  soak someone kept clicking on measures the clicking.
- **Measurement** — the run is repeated for a second game. One game's curve
  cannot distinguish an engine that leaks from a page that does.
- **Measurement** — a third run parks the account at a known point and continues
  sampling, so the figure after parking can be compared against the figure
  before the account ever started. That is the residue check, and it is the only
  thing in this item that tests item 03's promise over hours.
- **Measurement** — the reading is the floor of the sawtooth over successive
  intervals, not the last sample. State the window used to compute it, because a
  floor read over five minutes and a floor read over an hour are different
  claims.
- **Measurement** — record the machine, the total system memory, the engine
  version (`libwebkitgtk-6.0` as installed), the game, the build profile and the
  wall-clock duration. A measurement whose conditions are not written down
  cannot be repeated, which is the same as not having been taken.
- **Naming** — rule 1: `docs/memory-budget.md`, created by this slice with its
  findings section. Task 08 appends the budget itself to the same file rather
  than starting a second one.
- **Testing** — there is nothing here `cargo test` can run (code standards rule
  25). The evidence is the runbook section and the sample files it names.

## Acceptance criteria

_Descoped by user decision (2026-09-12): the formal four-hour soak, the
sawtooth-floor reading, the second-game soak, and the park-partway residue
check below were not run. The leak this item exists to characterize was
instead found and eliminated directly (see `docs/memory-budget.md`, "Round
3") — a live account/no-account A/B comparison, not a multi-hour continuous
soak. Not verified by the method these three criteria describe; deliberately
not attempted._

- [ ] ~~`(manual)` a soak of at least four hours on one live account is
      recorded, naming machine, engine version, game, build profile and
      duration~~ — descoped
- [ ] ~~`(manual)` the curve is read for the floor of the sawtooth rather than
      the last sample, and the averaging window used is stated~~ — descoped
- [ ] ~~`(manual)` a second game is soaked the same way and its curve recorded
      beside the first~~ — descoped
- [ ] ~~`(manual)` a run that parks the account partway records whether the
      figure returns to its pre-start level or leaves a residue, and how
      large~~ — descoped
- [x] `(manual)` `docs/memory-budget.md` exists and states, in one sentence,
      whether a live account's memory grows without bound or settles
- [x] `(manual)` where it settles, the settled figure per game is recorded —
      N/A: confirmed it does not settle, so this branch is vacuous
- [x] `(manual)` where it does not settle, the rate of growth per hour is
      recorded and the finding says plainly that the remaining slices cap a
      leak rather than fix it — recorded (`docs/memory-budget.md`, ≈690-1030
      MB/hour across two live soaks); the finding says more than the
      criterion asked for, since the actual leak was found and fixed rather
      than only capped

## References

- [Roadmap item](../../roadmap/05-memory-accounting/README.md) — "Where the
  memory goes, and what can be done about it", and the last blocker
- [`docs/requirements.md`](../../requirements.md) — `FR.19.3`, `FR.7.3`
- [`docs/code-standards.md`](../../code-standards.md) — rule 25
- [`docs/naming.md`](../../naming.md) — rule 1
- [Task 03](03-the-memory-report-script.md) — the soak mode this slice runs

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
