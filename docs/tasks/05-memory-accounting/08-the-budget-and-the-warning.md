# 08 — The budget, measured against the browser, and the warning

**Roadmap:** [05](../../roadmap/05-memory-accounting/README.md) · **Scope:** full-stack · **Depends on:** 04, 05, 06, 07

## Context

This is where the project stops asserting that it is cheaper than a browser and
writes down what it actually costs, next to what the browser actually costs, on
one machine, for the same three games.

The comparison is the point. An absolute figure — "the budget is six hundred
mebibytes" — is a number nobody can argue with because nobody can reproduce the
conditions that produced it, and a project whose entire claim is *cheaper than
the thing you were using* should hold itself to that claim rather than to a
threshold it picked for itself. So the budget doc records both columns
(`FR.19.2`), and the threshold comes out of the comparison. The first
measurement taken here, on 2026-09-08, had the browser winning; whether it still
does after tasks 06 and 07 is what this slice finds out and writes down either
way.

Three games, because one game's appetite is a fact about that game. And measured
last, after the reductions have landed, because a budget written against a
version about to change is a budget that was never true.

The warning is the small half. When a sample crosses the recorded budget, the
footer takes a warning appearance and says so. It does nothing else, and the
three things it does not do are each deliberate. It does not park or reload
anything, because the application cannot tell which account is responsible — the
engine offers no way to link a process to an account — and choosing what to give
up is the user's (`FR.20.2`). It does not move or resize a figure, so that an
eye returning to the total every few minutes never has to find it again. And it
does not persist: the moment a sample comes back under the budget, the warning
goes, with nothing to dismiss.

That last one is the opposite of how this application reports every other
problem, and the contrast is worth a design rule. The message strip of design
rule 9 never leaves until it is dismissed, because it reports an event that
happened once and would otherwise vanish unrecorded. This reports a condition
that is either true right now or is not, and a tint outliving its condition
would be a false statement where a faded strip would only be a lost one. The
rule the design doc owes has to say both halves, or the next item will reach for
the strip when it means this.

The verdict itself is not written here — task 01 built it, tested against
thresholds a test chose. What arrives here is the constant, and one CSS class
toggled from the answer.

## Technical details

- **Measurement** — three games, each measured in this application and in the
  browser being replaced, on the same machine, in the same session, with the
  same number of accounts. `docs/memory-budget.md` records machine, total system
  memory, engine version, browser version, games, build profile and both
  columns (`FR.7.3`, `FR.19.2`).
- **Measurement** — the browser column is measured the same way as ours, with
  task 03's script pointed at the browser's process tree, so the two columns are
  the same statistic. Comparing our proportional figure against a browser's
  resident figure would flatter us by exactly the amount this item exists to be
  honest about.
- **Code standards** — rule 5: the budget is a named constant carrying its unit,
  `MEMORY_BUDGET_MIB`, with a comment naming the section of
  `docs/memory-budget.md` it came from (rule 18).
- **Front-end** — the over-budget appearance is one CSS class toggled on the
  footer from the core's verdict, plus one line naming the budget. No second
  widget, no colour chosen in Rust, and the theme's warning tint rather than its
  error red — this is attention, not alarm.
- **Front-end** — the class is removed as soon as a sample comes back under
  (`FR.20.3`). Nothing latches, nothing is dismissed, and no timer is involved.
- **Architecture** — rule 8: the shell asks the core and paints the answer. The
  comparison stays where task 01 put it.
- **Design** — write both rules `docs/design.md` owes: how a figure is
  formatted, aligned and shown when unmeasured (task 04's pattern), and a
  readout that changes appearance on a measurement, clears itself, and offers no
  action — stating explicitly how it differs from rule 9's strip and why.
- **Architecture** — rule 14: the warning's evidence is `test-script.md`, driven
  by setting the budget low enough that a running account crosses it.

## Acceptance criteria

_Descoped by user decision (2026-09-12): the three-games-in-both app-and-browser
measurement, and the four warning-tint checks that depend on a real budget
being set, were not run. `MEMORY_BUDGET_MIB`
(`crates/idle-manager-shell/src/memory_footer.rs:40`) is left at `None` — the
warning stays permanently off (`is_over_budget` returns `false` whenever the
budget is `None`, per its own test) rather than shipping a guessed number._

- [ ] ~~`(manual)` `docs/memory-budget.md` records three games measured in both
      this application and the browser, on one machine, naming machine, memory,
      engine version, browser version and build profile~~ — descoped
- [ ] ~~`(manual)` both columns are the same statistic, taken with the same
      script, and the doc says so~~ — descoped
- [ ] ~~`(manual)` the budget threshold is stated with the sentence explaining
      which measurement produced it~~ — descoped
- [x] `(unit)` the budget is a named constant carrying its unit and the footer
      reads its verdict from the core rather than comparing figures itself
- [ ] ~~`(manual)` with the budget set below the current total, the footer
      takes the warning tint and shows one line naming the budget~~ —
      descoped: no real budget number exists to set
- [ ] ~~`(manual)` no figure moves, resizes or reorders when the warning
      appears, and no action button appears anywhere in the block~~ —
      descoped
- [ ] ~~`(manual)` parking an account back under the budget clears the tint
      and the line on the next sample, with nothing dismissed~~ — descoped
- [ ] ~~`(manual)` the warning never parks, reloads or otherwise changes an
      account by itself across a run that crosses the budget repeatedly~~ —
      descoped
- [x] `(unit)` `docs/design.md` carries both new rules, and the second states
      how it differs from rule 9's message strip

## References

- [Roadmap item](../../roadmap/05-memory-accounting/README.md) — `## User
  Experience` states and the "Reading the footer" diagram
- [Wireframe](../../roadmap/05-memory-accounting/wireframes/memory-footer.md) —
  the `## Over budget` variant
- [`docs/requirements.md`](../../requirements.md) — `FR.7.3`, `FR.7.4`,
  `FR.19.2`, `FR.20.1`, `FR.20.2`, `FR.20.3`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 18, 25
- [`docs/design.md`](../../design.md) — rule 9, whose opposite this is
- [Task 01](01-the-reading-and-the-budget-verdict.md) — the verdict this
  supplies the number to

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
