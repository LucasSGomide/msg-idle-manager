# The memory footer

## Purpose

The application's only numeric readout: what it costs right now, refreshed
without being asked.

## Where it sits

Renders the `**Entry**`, `**Flow**` and `**States**` bullets in
`## User Experience` and the "Reading the footer" diagram. It fills the footer
box item 02 left empty at the foot of the sidebar. The "Sampling the
application's memory" diagram is mechanism, not a screen.

## The screen

A block pinned to the bottom of the sidebar column, separated from the list by a
single hairline, in the smaller type size. Four figures on two lines, each a
label and a value, with the values right-aligned to a common edge so they can be
compared and summed by eye:

```
+--------------------------+
| Main account          1  |
| Farm 1             away  |
|                          |
|--------------------------|
| App              84 MiB  |
| 3 running       412 MiB  |
| Total           496 MiB  |
+--------------------------+
```

The values use a fixed-width numeric style so a figure does not shift its
neighbours as it changes on each sample. Before the first sample every value is
a dash, never a zero. If the measurement cannot be taken at all, the block
collapses to one line saying so and stops refreshing.

The count of running accounts is deliberately printed as part of the second
row's label rather than as a fourth figure, so that dividing the aggregate by it
is the obvious thing to do — that division is the closest this item comes to a
per-account figure, and the tooltip says why there is no real one.

Hovering anywhere in the block shows that tooltip: the figures are proportional,
shared memory is divided among the processes sharing it, and per-account figures
are not available.

## Over budget

The same block, same figures, same positions — one warning tint across it and
one line added beneath naming what was exceeded:

```
+--------------------------+
| App              91 MiB  |
| 1 running       791 MiB  |
| Total           882 MiB  |
| over budget (600 MiB)    |
+--------------------------+
```

Three things it must not do. It must not move, resize or reorder a figure, so
that the eye reading the total every few minutes never has to re-find it. It
must not offer an action: no Park button appears here, because the application
does not know which account is responsible (`FR.7.4`) and choosing is the user's
(`FR.20.2`). And it must not persist — the moment a sample comes back under the
budget the tint and the line go, with nothing to dismiss (`FR.20.3`).

That last point is the deliberate opposite of design rule 9's message strip,
which never leaves until it is dismissed. The strip reports an event that
happened once and would otherwise leave no record; this reports a condition that
is either true right now or is not. A tint that outlived the condition would be
a false statement, where a strip that faded would be a lost one.

## Design rules

- `docs/design.md` has no rule for a figure. This is the first place in the
  application that shows a number at all, so it owes one: how figures are
  formatted and to which unit, how they are aligned, and that an unmeasured
  value is a dash rather than a zero.
- `docs/design.md` owes a second rule this screen is the first case of — a
  readout that changes appearance because a measurement crossed a line, clears
  itself when it crosses back, and offers no action. Design rule 9's strip is
  its opposite in all three respects and the new rule has to say so, or the next
  item will reach for the strip when it means this.
- `docs/architecture.md` rule 10 — the sample is taken off the GTK main context
  and only the finished reading crosses back to touch these labels.
- `docs/architecture.md` rules 12 and 13, `docs/naming.md` rules 1 and 4 —
  `memory-footer.ui` beside `memory_footer.rs` and its `imp` module.
- `docs/code-standards.md` rule 5 — the sampling interval is a named constant
  carrying its unit, not a literal in the timer call.
