# 01 — The reading, the port and the budget verdict

**Roadmap:** [05](../../roadmap/05-memory-accounting/README.md) · **Scope:** back-end · **Depends on:** —

## Context

The application is about to start reading the kernel, and the rules layer must
not learn how. This slice writes down what the domain needs — someone who can be
asked what the application currently costs — and what an answer looks like, and
it settles the one judgement that follows from an answer: whether the figure is
over the line the project holds itself to.

An answer is three facts and nothing else: what the application's own process
costs, what everything it started costs between them, and how many processes
were counted to get there. The third is not decoration. A reading built from one
process on a machine that should have shown four is wrong in a way no figure
reveals, and the count is what lets a runbook step notice.

The figures are kibibytes, everywhere, in every direction. Converting to the
unit a person reads is the last thing that happens before a label is drawn and
it never happens on the way in, because a stored figure that has already been
rounded for display cannot be added to another one.

The count of running accounts belongs here too, and it does not come from the
kernel. The application already knows how many accounts are live; asking the
operating system how many rendering processes exist would answer a different
question and answer it worse, because a process that has died and not yet been
reaped is not an account anybody is running.

Last, the verdict. Whether a total is over budget is a rule, so it lives with
the rules and not with the widget that colours itself — the widget asks and
paints, exactly as it does for every other piece of state (architecture rule 8).
The *number* is not settled yet and cannot be until the games have been measured
in task 08; what this slice builds is the comparison, tested against a threshold
a test chooses. That is the whole point of separating them: the rule is
finished, checkable and merged long before anybody knows what to compare
against, and task 08 supplies a constant rather than reopening this file.

## Technical details

- **Architecture** — rule 5: the port `MemoryProbe` goes in the core's
  `ports.rs`, named for the capability rather than the technology (naming rule
  10), with `ProcPssProbe` in `idle-manager-metrics` fulfilling it in task 02.
  Rule 1 is why it exists at all: `scripts/arch-check.sh` forbids the core from
  reading `/proc`.
- **Architecture** — rule 6 is met twice over. The domain cannot read `/proc`,
  and the footer's formatting, its dash state and its verdict all need a test
  that does not depend on a machine having games running — so a fake probe in
  the core's tests is a requirement, not a nicety.
- **Code standards** — rule 5: the reading is a record whose field names carry
  their unit, `own_kib`, `descendants_kib`, `process_count`. A bare `own` invites
  the next caller to guess mebibytes.
- **Code standards** — rule 1: the reading a caller holds before the first
  sample is not a reading with zeroes in it. Absence is `Option`, so a zero in
  this type always means a measurement that came back zero.
- **Architecture** — rule 9: the verdict takes the budget as an argument rather
  than reading a constant, so a test picks its own threshold and the core stays
  free of a number nobody has measured yet.
- **Architecture** — rule 11, **code standards** rule 12: the port's error is a
  `thiserror` enum in the core beside the existing ones, distinguishing a
  measurement the system refused from one that came back unreadable — the shell
  shows the same unavailable state for both but logs a different reason
  (code standards rule 15).
- **Naming** — rules 8, 10, 11, 12 fix the spellings: `MemoryProbe` and its
  `sample`, a verdict named for its answer rather than its arithmetic, and any
  boolean shaped as a question.
- **Testing** — unit tests at the foot of the file (code standards rule 24),
  each named for the behaviour it pins (rule 21) and asserting one subject
  (rule 23).

## Acceptance criteria

- [ ] `(unit)` a reading exposes the application's own figure, its descendants'
      figure and the count of processes that contributed, all in kibibytes
- [ ] `(unit)` a total is the sum of the own and descendant figures and is
      derived from them rather than stored beside them
- [ ] `(unit)` a total equal to the budget is not over it, and one kibibyte
      above it is
- [ ] `(unit)` the verdict takes its budget as an argument, so two thresholds
      give two answers for the same reading
- [ ] `(unit)` a fake probe in the core's tests satisfies `MemoryProbe` without
      touching the filesystem
- [ ] `(unit)` a probe returning a refusal and a probe returning unreadable
      output are distinguishable by matching on the error
- [ ] `(unit)` the count of live accounts comes from the session book, and a
      parked account is not counted in it

## References

- [Roadmap item](../../roadmap/05-memory-accounting/README.md) — the "Sampling
  the application's memory" and "Reading the footer" diagrams
- [`docs/requirements.md`](../../requirements.md) — `FR.7.1`, `FR.7.4`,
  `FR.20.1`, `FR.20.2`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 5, 6, 8, 9, 11, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 5, 12, 15, 17,
  21, 23, 24
- [`docs/naming.md`](../../naming.md) — rules 2, 8, 10, 11, 12

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
