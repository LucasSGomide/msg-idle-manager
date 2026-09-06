# 04 — Adding an account and placing it in a slot

**Roadmap:** [01](../../roadmap/01-isolated-accounts-and-layouts/README.md) · **Scope:** front-end · **Depends on:** 01, 02

## Context

This is the slice where the application stops being an empty frame. Pressing
"Add game" opens a small window over the main one asking two questions: what to
call this account, and the address the game starts at. Those are the only two
things the program cannot work out for itself. The confirm button stays greyed
out until both boxes have something in them, so there is no wrong way to fill
the form and no error message to design. Pressing Escape closes it and creates
nothing.

Confirming makes the account real. The dialog does not build anything itself —
it reports what the person did, the deciding part of the program works out which
of the visible places the account should take, and the window redraws from that
answer. That one-way flow is a project rule, and following it here is what keeps
the later slices from turning into a tangle of widgets changing each other.

The window's content area gains the grid that holds the accounts. It draws one,
two or four rectangles depending on the current arrangement, separated by a
hairline and carrying no other decoration at all — anything drawn on top of a
place is drawn on top of a game. Clicking inside one marks it as the focused
place, shown by thickening the hairline on its own edges. That mark matters
later: the focused place is where the next account lands when everything is
full.

An account that has just been created has no page in it yet, so its place shows
the account's name centred on the window's own background. That is all this
slice draws — the real page arrives in the last slice of the item. The centred
name is not a placeholder for its own sake: it is what a place looks like
between being given an account and that account's page painting, which is a real
state a person sees every time they add a game.

Once there is at least one account, the "add your first game" block is replaced
by the grid, and it comes back if there is ever no account again.

## User experience

- **Flow** — press "Add game"; a dialog asks for a display name and a starting
  address; confirming creates the account, places it in a slot and shows its
  name there. Focus starts in the name field. Enter in either field triggers
  "Add" once both are filled; Escape cancels.
- **Flow** — press "Add game" again for a second account. It takes the next free
  slot in the current arrangement.
- **Flow** — click inside a slot to focus it. The focused slot is where the next
  account added will land when no slot is free.
- **States** — **empty**: with no accounts the centred block and its "Add your
  first game" button replace the grid entirely; that button opens the same
  dialog as the header bar's.
- **States** — **loading**: a slot given an account that has not painted yet
  shows the account's name centred on the window's own background colour. A slot
  with no account at all shows nothing.
- **New pattern** — the add-game dialog, a two-field modal over the main window.
  `docs/design.md` owes a rule for modal input — where the suggested action
  sits, and the keyboard contract — once this ships.

## Technical details

- **Architecture** — rule 8: `add_game_dialog.rs` emits an intent carrying the
  name and address; the core mints the identifier and returns the placement; the
  grid renders that result and decides nothing.
- **Architecture** — rules 12 and 13: `add_game_dialog.rs` with
  `add_game_dialog/imp.rs` and `resources/ui/add-game-dialog.ui`;
  `session_grid.rs` with `session_grid/imp.rs` and
  `resources/ui/session-grid.ui`, both added to the GResource list.
- **Architecture** — the grid is a `gtk::Widget` subclass with a custom
  `LayoutManager` that allocates each child the rectangle of the slot the core
  assigned it. Off-grid handling is the next slice; this one places only
  sessions the core put in a slot.
- **Architecture** — rule 10: every widget call stays on the GTK main context.
- **Code standards** — the loading state is a label drawn by the grid behind the
  slot's content, not a widget of its own, because a later roadmap item
  introduces the real placeholder a parked account needs.
- **Code standards** — rule 5: the "Add" button's sensitivity is bound to both
  entries being non-empty, so no validation branch and no error state exists.
- **Naming** — rules 1, 2 and 4: `add-game-dialog.ui` beside
  `add_game_dialog.rs`; `session-grid.ui` beside `session_grid.rs`.

## Acceptance criteria

- [x] `(manual)` pressing "Add game" opens a modal with a name entry and an
      address entry, focus in the name entry, and an "Add" action insensitive
      until both hold text
- [x] `(manual)` Escape closes the dialog and creates nothing; Enter in either
      entry with both filled performs "Add"; reopening shows both entries empty
- [x] `(manual)` confirming replaces the empty-state block with the grid and
      draws the account's name centred in the first slot
- [x] `(manual)` adding a second account draws its name in the next free slot of
      the current arrangement and leaves the first account where it is
- [x] `(manual)` the empty state's "Add your first game" button opens the same
      dialog as the header bar's button
- [x] `(manual)` clicking a slot thickens its own edges and un-marks the
      previously focused slot, with exactly one slot marked at any time
- [x] `(manual)` a slot holding no account draws nothing inside it — no chrome,
      no placeholder

## References

- [Roadmap item](../../roadmap/01-isolated-accounts-and-layouts/README.md) — the full picture, including the "Adding a game account" diagram
- [Wireframes](../../roadmap/01-isolated-accounts-and-layouts/wireframes/) — `add-game-dialog.md` and `main-window.md`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13 and 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 18 and 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4 and 7
- [`docs/design.md`](../../design.md) — no numbered rules yet; the modal is one of the patterns it owes a rule for

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
