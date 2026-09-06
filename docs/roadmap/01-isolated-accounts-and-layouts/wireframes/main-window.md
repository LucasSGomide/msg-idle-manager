# Main window

## Purpose

The application's only screen. It holds the games, the control that adds one and
the control that chooses how many are visible at once.

## Where it sits

Renders the `**Entry**` bullet ("the application window at launch") and both
interaction diagrams in `## User Experience` — "Adding a game account" ends here
and "Changing the arrangement" happens entirely here. Covers the empty, loading
and out-of-sight cases from the `**States**` bullet.

## The screen

A single window with a header bar and one content area. The header bar carries
the "Add game" button on the trailing edge and the three layout toggles beside
it, drawn as a linked group of three so it reads as one choice with three
answers rather than three independent switches. The content area below is the
grid, which is the whole rest of the window with no padding of its own — a game
is a web page and pads itself.

The grid draws one, two or four rectangles depending on the chosen layout, with
a hairline between them and nothing else; there is no chrome per slot, because
anything drawn on top of a slot is drawn on top of a game. The focused slot is
marked by that hairline thickening on its own edges, which is the least
intrusive marker available and still visible at a glance.

A slot that has been given an account but has not painted yet shows the
account's name centred on the window's own background colour. A slot with no
account at all shows nothing.

With no accounts at all, the grid is replaced entirely by a centred block: one
line of text and an "Add your first game" button, which is the same action as
the header bar's.

```
+--------------------------------------------------------+
| [ 1 | 2 | 4 ]                             [ + Add game ]|
+--------------------------------------------------------+
|                          |                             |
|      game A (focused)    |          game B             |
|                          |                             |
|==========================|=============================|
|                          |                             |
|      game C              |          (empty)            |
|                          |                             |
+--------------------------------------------------------+
```

Accounts with no slot are not drawn anywhere. They are laid out at coordinates
outside the grid's own bounds and clipped away — deliberately invisible rather
than absent, per the item's Front-end prose.

## Design rules

- `docs/design.md` has no numbered rules yet, so none can be cited. This screen
  is the first the application has, and the design doc owes rules for: what
  marks a focused slot, what a slot shows before its page paints, and where an
  empty state's action button sits relative to its text.
- `docs/architecture.md` rule 13 — the window and the grid are described in
  `.ui` composite templates compiled into a GResource, not built in Rust.
- `docs/architecture.md` rule 12 — each widget's private implementation lives in
  an `imp` module beside it.
- `docs/naming.md` rules 1 and 4 — the templates are `window.ui` and
  `session-grid.ui`, kebab-case, named after the widgets they define.
