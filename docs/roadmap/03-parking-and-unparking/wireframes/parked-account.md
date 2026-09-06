# A parked account

## Purpose

How an account that is not running is shown, in both places it can appear: its
row in the list, and the place on screen it may still hold.

## Where it sits

Renders both interaction diagrams in `## User Experience` — "Parking a running
account" and "Starting a parked account" — which act on one screen from two
directions. Covers the running, parked, starting and parked-in-a-slot cases from
the `**States**` bullet.

## The screen

Two surfaces of the main window change.

**The sidebar row** gains an action on its trailing edge, after the place
marker: a single button whose label inverts with the state, reading "Park" for a
running account and "Start" for a parked one. It is the row's only action, so it
sits on the row itself rather than behind a menu — item 04's settings go behind
one precisely because they are not this. A parked account's name is drawn in the
dimmed style, and its place marker reads as parked alongside its slot number,
because liveness and place are independent and the row has to say both. While
starting, the button is insensitive and the marker reads as starting.

**The slot**, when a parked account holds one, is replaced by a placeholder
panel: the account's name, a line saying it is parked, and a "Start" button,
centred on the window's background colour with generous space around them. It is
deliberately plain — it stands where a game was and should not compete with the
games in the other slots. The same widget carries item 08's failure panel with
different text and a different action, so the text line and the button label are
properties, not markup.

```
+------------------+-----------------------------------+
| Main account 1 [Park]  |                             |
| Farm 1  2·parked[Start]|         Main account        |
| Farm 2   away   [Start]|                             |
|                        |=============================|
|                        |          Farm 1             |
|                        |         Parked              |
|                        |        [ Start ]            |
+------------------------+-----------------------------+
```

## Design rules

- `docs/design.md` has no numbered rules yet. Two debts start here: a row action
  whose label and meaning invert with state, and the slot placeholder — which
  item 08 reuses, so whatever this screen settles binds that item too.
- The dimmed style now means two different things (out of sight, from item 02,
  and parked). The design doc owes a rule reconciling them before item 04 adds a
  third marker to the same trailing edge.
- `docs/architecture.md` rule 8 — the button emits an intent; the domain decides
  what parking means and the shell then makes the engine match.
- `docs/architecture.md` rules 12 and 13, `docs/naming.md` rules 1 and 4 —
  `slot-placeholder.ui` beside `slot_placeholder.rs` and its `imp` module.
