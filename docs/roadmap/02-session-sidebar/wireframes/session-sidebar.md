# Session sidebar

## Purpose

The index of every account the application holds, and the only place an account
with no slot is visible at all.

## Where it sits

Renders the `**Entry**` and `**Flow**` bullets in `## User Experience` and both
interaction diagrams — "Bringing an out-of-sight account into view" and "Folding
the list away". Covers the empty, in-a-slot, out-of-sight and folded cases from
the `**States**` bullet.

## The screen

A column down the leading edge of the main window, full height, of a fixed
comfortable width rather than a proportion of the window — the games take the
variable space. It is a single vertical list with no header of its own; the
header bar above already names the application.

Each row is one line: the account's display name on the leading edge, and its
place on the trailing edge. The place reads as the slot number for an account
that has one, and as a short word for one that does not. The name of an
out-of-sight account is drawn in the dimmed style; a row whose account holds the
focused slot is drawn as the current row.

Under the list, a footer box, empty and not drawn in this item. Item 05 fills it.

With no accounts, the list area holds one centred line of text and nothing else;
the "Add game" button in the header bar is the only way forward and is already
there.

Folded, the column is not drawn at all and the grid occupies the full window
width. The header-bar toggle reads as off.

```
+------------------+-----------------------------------+
| Main account   1 |                                   |
| Alt account    2 |            the grid               |
| Farm 1      away |                                   |
| Farm 2      away |                                   |
|                  |                                   |
|                  |                                   |
+------------------+-----------------------------------+
| (footer, item 05)|                                   |
+------------------+-----------------------------------+
```

## Design rules

- `docs/design.md` has no numbered rules yet. This screen creates two debts the
  later items inherit: what a row's trailing state marker looks like when it
  carries more than one fact — items 03, 04, 07 and 08 each add one — and what
  the dimmed style means, given that item 03 reuses it for a different state.
- `docs/architecture.md` rule 8 — activating a row emits an intent; the row
  changes nothing itself.
- `docs/architecture.md` rules 12 and 13 — `imp` module beside the widget,
  layout in `session-sidebar.ui`.
- `docs/naming.md` rules 4 and 6 — `session-sidebar.ui` beside
  `session_sidebar.rs`; the row type is `sidebar::Row`, not `SidebarRow`.
