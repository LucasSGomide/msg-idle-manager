# 04 — The sidebar footer

**Roadmap:** [05](../../roadmap/05-memory-accounting/README.md) · **Scope:** front-end · **Depends on:** 01, 02

## Context

This is the item's visible half and the first number this application has ever
put on a screen. It fills the box item 02 left empty at the foot of the account
list: what the application's own window and controls cost, how many accounts are
running, what those accounts cost between them, and the total.

Four figures is a deliberate limit. The temptation with a measurement is to show
everything measured, and the process-by-process breakdown does exist — task 03
prints it — but a panel that listed six processes would be a diagnostic tool
stapled to a sidebar, and the person reading this is not debugging. They are
answering two questions: is this cheaper than the browser I replaced, and did
parking that account get me anything. Four figures answer both, and the second
one gets answered in the moment the account is parked, which is the whole reason
the readout is here rather than in a menu.

The count of running accounts is printed inside a label rather than as a figure
of its own, so that dividing the aggregate by it is the obvious next thought.
That division is as close to a per-account figure as this item goes, and the
tooltip says plainly why there is no real one: the engine offers no way to tell
which of its processes belongs to which account, so an attribution would be a
guess with a decimal point on it.

Before the first sample lands, every value is a dash. A zero would be a claim
that a measurement was taken and came back empty, and no measurement has been
taken. That distinction survives into the type — the reading is absent, not
zeroed — and the formatter is where absence becomes a dash.

The sample itself is file work and does not happen on the main context. Reading
the kernel for every process on the machine, several times a minute, on the
thread that draws the window, is how an interface starts stuttering under
exactly the conditions this application exists to survive.

The over-budget appearance is not in this slice. It needs a budget, the budget
needs three games measured, and the measuring needs this readout and the
reductions to have landed first — so it arrives in task 08, on the widget this
slice builds.

## Technical details

- **Architecture** — rules 12 and 13, **naming** rules 1, 2, 4, 7:
  `memory_footer.rs` with its `imp` module beside it and
  `resources/ui/memory-footer.ui` describing the labels, registered in
  `idle-manager.gresource.xml` like every other template.
- **Front-end** — a grid of label-and-value pairs with the values right-aligned
  to a common edge and set in a fixed-width numeric style, so a figure changing
  on a sample never shifts its neighbour. The wireframe fixes the arrangement.
- **Architecture** — rule 10: `glib::timeout_add_local` on the interval,
  `gio::spawn_blocking` to take the reading, `glib::spawn_future_local` to apply
  it. Nothing touches a widget off the main context.
- **Code standards** — rule 5: the interval is a named constant carrying its
  unit. The roadmap item lists choosing its value as an open blocker; this slice
  closes it by measuring what one sample costs with task 03's script running
  alongside, and records the figure that settled it in the runbook.
- **Architecture** — rule 8: the widget renders and does not decide. The live
  account count comes from the session book and the figures from the port; the
  footer joins two sources and computes nothing.
- **Front-end** — the formatter converts kibibytes to the displayed unit at the
  last moment and renders an absent reading as a dash (`FR.7.1`). It is a plain
  function, unit-tested in this crate, because it is the one part of this widget
  that can be tested without a screen.
- **Front-end** — a failed tick logs with `tracing` in fields and switches the
  readout to its unavailable line, which stops the timer rather than leaving
  stale figures on screen. Never `let _ =` (code standards rules 14, 15).
- **Front-end** — the tooltip covers the whole block and carries three
  sentences: the figures are proportional, shared memory is divided among the
  processes sharing it, and per-account figures are not available (`FR.7.4`).
- **Design** — `docs/design.md` owes a rule for how a figure is formatted,
  aligned and shown when unmeasured. This slice is its first case; write the
  rule when the code exists, not before, and cite it from task 08.
- **Architecture** — rule 14, **code standards** rule 25: everything but the
  formatter is proved in `test-script.md`.

## Acceptance criteria

- [x] `(unit)` the formatter renders an absent reading as a dash and a present
      one as a figure with its unit
- [x] `(unit)` `memory-footer.ui` is readable from the compiled resource bundle,
      as the window and placeholder templates are
- [ ] `(manual)` at launch, before the first sample, all four values read as
      dashes and never as zeroes
- [ ] `(manual)` once sampling starts the footer shows the application's own
      figure, the running count, the aggregate and the total, and they refresh
      without any interaction
- [ ] `(manual)` parking a running account drops the running count by one and
      drops both memory figures, and the drop matches what `make memory-report`
      prints at the same moment
- [ ] `(manual)` unparking that account raises all three back, and the count
      returns to what it was
- [ ] `(manual)` collapsing the sidebar hides the footer with the list and
      expanding it brings the figures back still refreshing
- [ ] `(manual)` hovering the block shows the tooltip stating the figures are
      proportional and that per-account figures are not available
- [ ] `(manual)` with the probe made to fail, the block collapses to its
      unavailable line, stops refreshing, and the log carries a reason
- [ ] `(manual)` the sampling interval's cost is measured with the script
      running alongside and the figure that settled the constant is recorded

## References

- [Roadmap item](../../roadmap/05-memory-accounting/README.md) — the "Reading
  the footer" diagram and the `## User Experience` states
- [Wireframe](../../roadmap/05-memory-accounting/wireframes/memory-footer.md)
- [`docs/requirements.md`](../../requirements.md) — `FR.7.1`, `FR.7.4`,
  `FR.18.2`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 14, 15, 17, 21,
  23, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7, 11
- [`docs/design.md`](../../design.md) — no rule yet; this slice is the first case
  of one it owes

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
