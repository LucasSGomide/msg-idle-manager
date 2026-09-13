# 02 — Renaming an account from the sidebar

**Roadmap:** [10](../../roadmap/10-rearranging-accounts/README.md) · **Scope:** front-end · **Depends on:** 01, 04

## Context

The application shows a sidebar listing every game account, one row each. Every
row has a small "⋯" button that opens a menu with that account's settings: a Park
or Start action, which pauses or resumes the game, and a "Keep running when
hidden" switch. The previous rename slice gave the rules layer a rename operation
but nothing to reach it. This slice is where a person can actually rename an
account.

The row's menu gains one more item, "Rename…", placed last. Choosing it opens a
small window titled "Rename account". It has one text field holding the current
name, all selected so typing replaces it, and Cancel and Rename buttons. Rename
is greyed out while the field is empty or holds only spaces, and there is no
error message: the empty field says it. Enter confirms and Escape cancels.

On confirm the window closes and the new name appears at once in the sidebar row.
It also appears in the account's place in the main window: a place shows a label
with the name until the game's page has drawn, and a paused account's place shows
a plain panel carrying its name. Nothing reloads. A running game keeps running,
a paused one stays paused, and the change is saved like every other change.
Rename is offered in every state an account can be in, including while it is
still starting.

This slice also fixes a gap that would make a rename look broken. Today the
place's label and the paused panel's name are set once, when the place is
created, so a rename would never reach them. From now on they are refreshed from
the account list every time the grid redraws, the same way the paused panel
already is.

## User experience

- **Entry** — a new `Rename…` item in an account row's existing ⋯ menu in the
  sidebar, last, below the Park/Start action and "Keep running when hidden". It
  is never greyed out, even while a starting account's Park/Start item is. The
  row's trailing edge gains nothing.
- **Flow** — choose `Rename…`. A small modal window, transient for the main
  window and not resizable, with the add-game dialog's width and margins, opens
  titled "Rename account". It holds one text field with the current name, fully
  selected, and `Cancel` then `Rename` at the trailing edge, `Rename` the default
  button.
- **Flow** — type a new name. `Rename` is insensitive while the trimmed text is
  empty. Enter confirms, Escape cancels.
- **Flow** — confirm. The window closes, and the row, that account's place cover
  and its parked or queued panel show the new name immediately. The row keeps
  whatever bold or dimming it already had. Nothing reloads.
- **States** — **empty name**: `Rename` insensitive, no error text, no red field.
  **Same name**: accepted, and nothing visible changes. **Parked, starting or
  queued account**: identical to a live one; there is no state in which rename is
  unavailable.
- **Pattern** — `Rename…` joins the row's ⋯ menu per `docs/design.md` rule 5
  (action first, settings below). The rename window is the add-game dialog's
  details stage cut down to one field
  (`crates/idle-manager-shell/src/add_game_dialog/imp.rs:246`). A parked or queued
  place keeps rule 4's plain panel and shows the new name through its name
  property, with no restyle. Rule 8: an empty name greys the button rather than
  raising an error.

## Technical details

- **Front-end** — `bind_row_menu` at
  `crates/idle-manager-shell/src/session_sidebar/imp.rs:335` appends `Rename…`
  after the keep-awake item and adds a stateless `rename` action to the group it
  already builds, beside `PARKING_ACTION` and `KEEP_AWAKE_ACTION`, its name a
  constant. The handler carries only the account's id through a new
  `SessionSidebar::connect_rename_requested`, registered like
  `connect_parking_toggled`. The sidebar decides nothing (architecture rule 8).
- **Front-end** — a `RenameDialog` widget: `rename_dialog.rs` holds the public
  wrapper, `rename_dialog/imp.rs` the subclass, and
  `resources/ui/rename-dialog.ui` is registered in `idle-manager.gresource.xml`
  (architecture rules 12 and 13, naming rules 1, 2 and 4).
- **Front-end** — the dialog is a modal, non-resizable `gtk::Window` titled
  "Rename account" with one `gtk::Entry`, `Cancel` and `Rename`. `Rename` is the
  default widget so Enter confirms, and Escape closes the window. It is built
  with the current name and selects all of it on show. `Rename` is sensitive
  only while `account_name` returns `Some`. Confirming emits the entry's text
  through `connect_confirmed` and closes.
- **Front-end** — `window/imp.rs` registers `connect_rename_requested` and opens
  the dialog transient for itself, as `present_add_game_dialog` does at
  `crates/idle-manager-shell/src/window/imp.rs:466`. On confirm it calls
  `SessionBook::rename`. Only when that returns `true` does it call `redraw` and
  `request_save`. It never touches a `SessionView` holder, so nothing reloads.
- **Front-end** — names in the grid follow the book. Today the cover's label and
  the placeholder's name are set once in registration at
  `crates/idle-manager-shell/src/session_grid/imp.rs:173` and `:177`. `SlotEntry`
  keeps the cover's `gtk::Label`, and `SessionGrid::sync` sets it and the
  placeholder's name from each session's `display_name` on every pass, the way it
  already refreshes the placeholder panel (`FR.13.4`).
- **Design** — rules 3, 4, 5, 6 and 8. Rule 6: the row's trailing edge gains no
  widget, so the sidebar width needs no re-derivation.
- **Testing** — `(manual)`, against the headless harness, in this task's section
  of `test-script.md` (architecture rule 14). If this is the item's first
  accepted slice, it also writes `## Setup` and `## Teardown`.

## Acceptance criteria

- [ ] `(manual)` the ⋯ menu of a live, a parked, a queued and a starting account
      each lists `Rename…` last, below "Keep running when hidden", and it is
      sensitive in all four
- [ ] `(manual)` choosing `Rename…` opens a modal "Rename account" window over
      the main window whose field holds the current name fully selected
- [ ] `(manual)` clearing the field, or leaving only spaces in it, makes `Rename`
      insensitive with no error text shown; typing a letter makes it sensitive
      again
- [ ] `(manual)` Escape closes the window with the name unchanged in the row and
      `sessions.toml` not rewritten
- [ ] `(manual)` typing a new name on a live account and pressing Enter closes
      the window, and the row and that place's name cover show the new name at
      once while the game's page keeps running without a reload
- [ ] `(manual)` renaming a parked account shows the new name on its place's
      parked panel, and the account stays parked
- [ ] `(manual)` confirming the same name closes the window and nothing visible
      changes
- [ ] `(manual)` after a rename and a relaunch, the row shows the new name and
      the account's profile folder on disk has the same name as before
- [ ] `(unit)` `rename-dialog.ui` is readable from the compiled resource bundle,
      as the window and add-game dialog templates are

## References

- [Roadmap item](../../roadmap/10-rearranging-accounts/README.md) — the full
  picture, including the "Renaming an account" diagram this slice implements
  from the menu to the save
- [Row menu wireframe](../../roadmap/10-rearranging-accounts/wireframes/row-menu.md)
  — where `Rename…` sits; this slice draws all of it
- [Rename window wireframe](../../roadmap/10-rearranging-accounts/wireframes/rename-window.md)
  — the dialog, its empty-name and same-name states; this slice draws all of it
- [Grid drag wireframe](../../roadmap/10-rearranging-accounts/wireframes/grid-drag.md)
  — not drawn here; linked for the place cover and parked panel whose names this
  slice refreshes
- [`docs/requirements.md`](../../requirements.md) — `FR.13.1`, `FR.13.3`,
  `FR.13.4`, `FR.13.5`
- [`docs/design.md`](../../design.md) — rules 3, 4, 5, 6, 8
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13, 14
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
