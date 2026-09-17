# 05 — Selection mode and moving accounts

**Roadmap:** [11](../../roadmap/11-account-workspaces/README.md) · **Scope:** front-end · **Depends on:** 02, 03

## Context

This application runs several accounts of browser idle games in one window. A
sidebar lists them under workspace headings, where a workspace is a named set of
at most four accounts with its own arrangement of places, and a built-in
workspace called Ungrouped holds the rest. Earlier slices built that list,
switching between workspaces, and the rules for moving accounts. This slice is
where a person first puts accounts into a workspace.

Arranging happens in a selection mode, because in the normal list a click already
means "show me this account". A Select button in a new title row at the top of
the sidebar turns the mode on and reads Done while it is on. Every account row
then shows a tick box, and a bar appears near the bottom of the sidebar with a
count of ticked accounts and a Move to… button. In this mode, clicking a row ticks
or unticks it and never switches what is on screen. Ticks stay put when a heading
is folded away.

Move to… lists every workspace with room for all the ticked accounts, then
Ungrouped. A workspace without room is simply not offered. Choosing one moves the
accounts to the bottom of that workspace's list, ends the mode and clears every
tick. The workspace on screen does not change. If an account left it, its place
stays empty and its game keeps running out of sight. Done leaves the mode without
moving anything.

If every account leaves the workspace on screen, the grid shows a quiet "No games
in this workspace" message. That is different from the first-launch message,
which only appears when there are no accounts anywhere.

Making a brand-new workspace from the ticked accounts, and renaming or removing
workspaces, come in the next slice. That keeps this one to the mode itself and
moving between workspaces that already exist.

## User experience

- **Entry** — selection mode: a `Select` toggle in a new title row at the top of
  the sidebar, reading `Done` while the mode is on.
- **Flow** — press `Select`. Every account row gains a leading tick box, and a
  bar appears above the memory footer reading "0 ticked" beside a `Move to…`
  button, insensitive at zero. Press `Done` to leave without moving anything;
  ticks clear.
- **Flow** — click account rows, or their tick boxes, to tick them. A click never
  switches workspace or focus in this mode. The bar's count follows, and ticks
  survive collapsing a heading.
- **Flow** — open `Move to…`. The menu lists every named workspace with room for
  the whole ticked set, then `Ungrouped`; a workspace without room is not listed.
  Choose one: the ticked accounts move to the bottom of its list, selection mode
  ends and every tick clears. The shown workspace does not change, and an account
  that left it leaves its place empty.
- **States** — **`Move to…` with nothing ticked**: insensitive. **Ticked set
  larger than four**: no named workspace is listed, and `Ungrouped` is the only
  destination. **Shown workspace empty**: the grid area shows "No games in this
  workspace" centred and dim, with no button. **No accounts anywhere**: the
  existing first-run state, "No games yet — add one to get started." plus its
  button, unchanged.
- **New pattern** — a list-wide selection mode: a toggle that swaps what a row
  click means and reveals tick boxes plus an action bar. The shown-workspace-empty
  state reuses the window's centred empty box at
  `crates/idle-manager-shell/resources/ui/window.ui:73` with a second label.
  Nothing in `docs/design.md` covers a mode, so the design doc owes a rule once
  this ships.

## Technical details

- **Front-end** — `session-sidebar.ui` gains a title row above the scroller,
  holding an "Accounts" label and a `Select` `gtk::ToggleButton` whose label
  follows its state. It also gains a hidden `selection_bar` box above `footer`,
  holding the count label and a `Move to…` `gtk::MenuButton`.
- **Front-end** — `SessionSidebar` holds `is_selecting: Cell<bool>` and
  `ticked: RefCell<HashSet<SessionId>>`. Ticks are ids, not the list's positional
  selection, because collapsed rows leave the model (`FR.17.4`). `sync` prunes
  ids no longer in the book.
- **Front-end** — the superset row factory's account layout gains a leading
  `CheckButton`, shown only while selecting and bound from `ticked`.
  `connect_activate` at `session_sidebar/imp.rs:135` toggles the id in `ticked`
  instead of emitting activation while selecting. The check button's `toggled`
  does the same.
- **Front-end** — the count label reads from `ticked.len()` on every change, and
  `Move to…` is insensitive while `ticked` is empty.
- **Front-end** — the move menu is rebuilt when it opens, from a `Destinations`
  the window supplies through `set_move_destinations`. The window gets it from
  `WorkspaceBook::destinations(ticked.len())`, so the sidebar never decides room.
  Choosing an entry emits `connect_move_requested(Vec<SessionId>, MoveTarget)`
  with `MoveTarget::Existing(WorkspaceId)`.
- **Front-end** — a new window intent, `move_ticked(ids, to)`, calls
  `WorkspaceBook::move_accounts` (architecture rule 8). Only on `Ok` does it call
  the sidebar's `end_selection`, which clears `ticked` and leaves the mode, then
  `redraw` and `request_save`. No liveness changes and the shown workspace stays
  shown (`FR.17.8`).
- **Front-end** — `redraw` at `window/imp.rs:936` shows the first-run empty state
  only when no workspace has any account. When only the shown workspace is empty,
  it shows the new dim "No games in this workspace" label, with no button, in the
  centred box at `window.ui:73` (`FR.15.10`).
- **Design** — rules 1, 3 and 6 still bind account rows. The tick box fits the
  200-wide sidebar set by the tree slice. Testing is `(manual)` against the
  headless harness, seeded with a version 2 `sessions.toml`, in this task's
  section of `test-script.md` (architecture rule 14).

## Acceptance criteria

- [x] `(manual)` pressing `Select` relabels it `Done`, shows a tick box on every
      account row, and shows a bar above the memory footer reading "0 ticked"
      with `Move to…` insensitive
- [x] `(manual)` in selection mode, clicking an account row ticks it without
      focusing it or switching workspace, clicking its tick box does the same,
      and the count follows every toggle
- [x] `(manual)` ticking an account, collapsing its heading and expanding it
      again shows the account still ticked and the count unchanged
- [x] `(manual)` pressing `Done` hides the tick boxes and the bar and moves
      nothing, and pressing `Select` again shows "0 ticked"
- [x] `(manual)` with "Party" holding three accounts and "Duo" holding two, two
      ticked `Ungrouped` accounts open a `Move to…` menu listing "Duo" then
      `Ungrouped`, without "Party"
- [x] `(manual)` choosing "Duo" moves the ticked accounts to the bottom of Duo's
      list, ends selection mode with every tick cleared, leaves the shown
      workspace unchanged, and `sessions.toml` lists them under Duo
- [x] `(manual)` moving an account out of the shown workspace leaves its place
      empty, and its kept-awake game keeps ticking out of sight without a reload
- [x] `(manual)` with five accounts ticked, `Move to…` lists only `Ungrouped`
- [x] `(manual)` moving every account out of the shown workspace shows "No games
      in this workspace" centred and dim, with no button
- [x] `(manual)` with no accounts in any workspace, the first-run state "No
      games yet — add one to get started." and its button appear unchanged

## References

- [Roadmap item](../../roadmap/11-account-workspaces/README.md) — the full
  picture, including the "Grouping accounts in selection mode" and "Sidebar
  modes" diagrams; this slice builds their move-to-existing branch
- [Wireframes](../../roadmap/11-account-workspaces/wireframes/) — this slice
  draws `selection-mode.md` without its `New workspace…` entry, and
  `empty-workspace-grid.md`
- [`docs/research/gtk4-drag-and-accordion.md`](../../research/gtk4-drag-and-accordion.md)
  — why ticks are a set of ids over a tree model
- [`docs/requirements.md`](../../requirements.md) — `FR.15.10`, `FR.17.1`,
  `FR.17.2`, `FR.17.3`, `FR.17.4`, `FR.17.7`, `FR.17.8`
- [`docs/design.md`](../../design.md) — rules 1, 3, 6
- [`docs/architecture.md`](../../architecture.md) — rules 8, 12, 13, 14
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
