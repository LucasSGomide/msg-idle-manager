# 05 — The design rules, the Windows check and the runbook

**Roadmap:** [15](../../roadmap/15-window-shortcuts-and-focus/README.md) · **Scope:** full-stack · **Depends on:** 03, 04

## Context

Two of this item's slices left a debt behind them on purpose, and this one
pays it.

The first is a rule the item knowingly bends. `docs/design.md` rule 2 says a
row's park/start action gets **one** control whose label and effect invert
with the account's state, and gives two reasons: a single control that reads
the state can never be pressed in a direction that does not apply, and a pair
of buttons would always have one half greyed on a trailing edge the next rule
calls scarce. Task 02 shipped two chords instead of one. That is a real
departure and it must not sit in the code unrecorded — the whole point of the
rule docs is that the next person reads them rather than the diff. Neither of
rule 2's reasons survives a keyboard: the inapplicable direction is already a
silent no-op, so there is no wrong state to reach, and a key occupies no
trailing edge at all. What is genuinely lost — that a key carries no label
saying which direction it will take — is answered by the row's dot, which says
which of the two keys is live before either is pressed, and answered better
than a flip key would, since liveness moves on its own between deciding and
pressing. So rule 2 is *narrowed* to the control it was written about, and a
new rule states the keyboard case beside it.

The second is Windows. Task 03 added six key translations that this machine
can compile and unit test but cannot exercise. The VM at
`scripts/windows-vm/compose.yml` is where a physical keyboard presses the
chords, the way item 12's runbook did it. Where a chord genuinely cannot be
run there, it is named and left unticked rather than claimed.

And the runbook itself: `test-script.md` for this item, whose earlier sections
each slice appended, gets its last section and its `## Teardown`.

## User experience

- Nothing new on screen. This slice changes two documents and runs checks.

## Technical details

- **Design** — `docs/design.md` rule 2 gains one closing sentence narrowing
  its scope: its one-control reasoning governs a control carrying a visible
  label, and the keyboard case is rule 20. Nothing already in rule 2 is
  rewritten or removed.
- **Design** — `docs/design.md` gains rule 20: **give a two-state action one
  key per direction, each idempotent and inert where it does not apply, rather
  than one key that flips.** The reasoning is the roadmap item's Design
  section, written out: both of rule 2's justifications are about a control
  with a label on a scarce edge, neither applies to a chord; a flip key would
  read worse because nothing would say which direction the press is about to
  take and liveness moves on its own between deciding and pressing; and the
  row's dot already names which key is live. Cites `FR.26.3`, `FR.26.6` and
  rule 2.
- **Design** — rule 19's discoverability half is extended by one sentence:
  where a control is a menu item, the key is named in accelerator text rather
  than a tooltip, because a menu item has nowhere to hover (`FR.25.4`).
- **Architecture** — no code changes in this slice. `make verify` and
  `make windows-package` are run as a gate, not as a change.
- **Code standards** — `test-script.md` gains its `## 05` section and its
  `## Teardown`, and every earlier section's steps are confirmed run rather
  than assumed (rule 25's spirit: a ticked box means it was executed).

## Acceptance criteria

- [x] `(integration)` `make verify` and `make windows-package` pass
- [x] `(manual)` `docs/design.md` rule 2 carries the narrowing sentence, rule
      20 exists with the reasoning above, and rule 19 names accelerator text;
      the file's own numbering and cross-references stay consistent
- [ ] `(manual)` in the Windows VM with a physical keyboard and a game page
      holding focus, `Ctrl`+`B`, `Ctrl`+`1` / `2` / `4`, `Ctrl`+`P`,
      `Ctrl`+`S`, `Ctrl`+`Shift`+`P` and `Ctrl`+`Shift`+`S` each do what they
      do on Linux, and the page receives no `keydown` for any of them
- [ ] `(manual)` in the VM, `Ctrl`+`S` on a page with a focused text field
      opens no save dialog and types no `s` into the field
- [ ] `(manual)` in the VM, holding `Ctrl`+`Shift`+`P` for two seconds parks
      the workspace exactly once
- [ ] `(manual)` in the VM, focusing an account hands its page the keyboard
      with no click inside it (`FR.27.1` on the second engine)
- [ ] `(manual)` `test-script.md` holds a section per slice with every step
      run, a `## Setup` and a `## Teardown`, and any chord the VM could not
      exercise is named in this item's Blockers rather than ticked

## References

- [Roadmap item](../../roadmap/15-window-shortcuts-and-focus/README.md) —
  "Design — the two-chord departure from rule 2"; Front-end "Design and
  naming"; Blockers, the second and third
- [`docs/requirements.md`](../../requirements.md) — `FR.26.3`, `FR.26.6`,
  `FR.26.7`, `FR.25.4`, `FR.27.1`
- [`docs/design.md`](../../design.md) — rules 2, 6, 19; rule 20 is added here
- [`docs/architecture.md`](../../architecture.md) — rule 14
- [`docs/code-standards.md`](../../code-standards.md) — rule 25

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
