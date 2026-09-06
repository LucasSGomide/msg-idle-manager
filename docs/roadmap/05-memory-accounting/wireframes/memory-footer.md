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

## Design rules

- `docs/design.md` has no numbered rules yet. This is the first place in the
  application that shows a number at all, so it owes the rules: how figures are
  formatted and to which unit, how they are aligned, and that an unmeasured
  value is a dash rather than a zero.
- `docs/architecture.md` rule 10 — the sample is taken off the GTK main context
  and only the finished reading crosses back to touch these labels.
- `docs/architecture.md` rules 12 and 13, `docs/naming.md` rules 1 and 4 —
  `memory-footer.ui` beside `memory_footer.rs` and its `imp` module.
- `docs/code-standards.md` rule 5 — the sampling interval is a named constant
  carrying its unit, not a literal in the timer call.
