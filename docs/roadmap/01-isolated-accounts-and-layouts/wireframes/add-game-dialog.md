# Add-game dialog

## Purpose

Where an account is created: the two things the application cannot work out for
itself, asked once.

## Where it sits

Renders the first half of the "Adding a game account" interaction diagram in
`## User Experience`, up to the point where the view is built. Reached from the
"Add game" button in the header bar and from the empty state's button.

## The screen

A modal dialog over the main window, sized to its content rather than to the
window. Two labelled entries stacked vertically: a display name for the account,
and the address the game starts at. Beneath them, a cancel action and an "Add"
action, with "Add" as the suggested one.

"Add" is insensitive until both fields hold something, so there is no error
state to design and no invalid submission to report. The address is not
validated beyond being non-empty — an address that does not load is the engine's
message to deliver inside the slot, not this dialog's.

Focus starts in the name field. Enter in either field triggers "Add" once both
are filled; Escape cancels.

```
+-------------------------------+
|  Add a game                   |
|                               |
|  Name                         |
|  [ Main account            ]  |
|                               |
|  Address                      |
|  [ https://…               ]  |
|                               |
|            [ Cancel ] [ Add ] |
+-------------------------------+
```

## Design rules

- `docs/design.md` has no numbered rules yet. The design doc owes rules for
  modal input: where the suggested action sits, whether a submit action is
  disabled or shows an error, and what the keyboard contract is.
- `docs/architecture.md` rule 8 — the dialog emits an intent and does not create
  anything itself.
- `docs/architecture.md` rule 13 — described in `add-game-dialog.ui`.
- `docs/naming.md` rule 4 — `add-game-dialog.ui` beside `add_game_dialog.rs`.
