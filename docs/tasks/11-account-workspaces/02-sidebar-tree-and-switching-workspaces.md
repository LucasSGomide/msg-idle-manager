# 02 — Sidebar tree and switching workspaces

**Roadmap:** [11](../../roadmap/11-account-workspaces/README.md) · **Scope:** full-stack · **Depends on:** 01

## Context

This application runs several accounts of browser idle games in one window. The
window splits into one, two or four places, and a sidebar on the left lists every
account. The previous slice taught the app to keep several workspaces, which are
named sets of up to four accounts, each with its own arrangement. Every account
still starts in a built-in workspace called Ungrouped, and so far the screen only
ever shows that one. This slice makes workspaces visible and lets a person move
between them.

The sidebar becomes a two-level list. Each workspace gets a heading with its
accounts indented beneath, and Ungrouped always comes last. Clicking a heading
folds its accounts away or shows them again, and that choice is still there after
a relaunch. Headings show no status of their own. The coloured dots beside the
accounts already say what is running and what is on screen. The sidebar gets a
little wider so the indented names still read in full.

Clicking an account that sits in a workspace not on screen switches to that
workspace in one step. Its accounts take their places, the buttons that choose
one, two or four places change to show that workspace's choice, and the clicked
account moves into the active place. The accounts that were on screen go out of
sight but keep running. Nothing reloads and no game has to log in again.
Switching back puts the first workspace exactly as it was left. The layout
buttons now change only the workspace being shown.

Nothing in the app can create a named workspace yet; a later slice adds that. So
this slice is checked by hand against a settings file written with a named
workspace in it. It carries the switching rules with it because the tree is their
only user, and splitting them out would push the item past its task limit.

## User experience

- **Entry** — the sidebar is a two-level list: a heading per workspace, with its
  accounts nested beneath. `Ungrouped` is always last. The sidebar widens from
  150 to 200 under design rule 6's re-derivation clause, and the rule's record of
  width changes gains this step.
- **Flow** — switching: click an account under a workspace that is not shown.
  The window shows that workspace's arrangement, the header's 1/2/4 buttons flip
  to its layout, and the account takes that workspace's active place. Nothing
  reloads.
- **Flow** — expanding: click a heading or its expander arrow, and its accounts
  show or hide. The shown workspace does not change, and the choice survives a
  relaunch. The 1/2/4 buttons act on the shown workspace only.
- **States** — **named workspace with no accounts**: its heading stays, and
  expanding it shows one dim line, "No accounts".
- **States** — **account rows**: status dots, dimming and bold follow design
  rules 1 and 3. An account in a workspace that is not shown keys as
  `background` (or its liveness key), never `current` or `visible`. Headings carry
  no dot, no bold and no marker of which workspace is shown. The green dots
  under a heading say that.
- **New pattern** — a two-level accordion list with headings that carry no state.
  Rule 1 only covers account rows, so the design doc owes a rule for headings
  once this ships.

## Technical details

- **Back-end** — `WorkspaceBook` gains:
  - `placement(&self, id) -> Option<Visibility>`, which returns the account's
    own visibility in the shown workspace and `Visibility::OffGrid` for every
    account elsewhere (`FR.18.1`).
  - `focus_account(&mut self, id) -> Option<Switch>`. When the account's
    workspace is not the shown one, it switches to it and returns `Switch { from,
    to }`. Either way it then calls that book's `focus_session`, so the existing
    swap-into-focus rule is unchanged (`FR.16.3`).
  - `set_layout` on the shown workspace only (`FR.16.4`), and `set_expanded(&mut
    self, workspace, bool)` carried into `saved()` (`FR.16.2`).
- **Front-end** — `focus_session` at
  `crates/idle-manager-shell/src/window/imp.rs:668` calls `focus_account`. On a
  `Switch` it calls `select_layout_toggle` for the incoming layout and a new
  `snap_zoom_for_active`, which is `snap_all_zoom` at `:432` narrowed to the
  shown workspace's accounts, each resolved against its own workspace's layout.
  `connect_layout_toggle` at `:408` returns early when the toggled layout already
  equals the shown workspace's, so flipping the buttons never arranges or saves a
  second time. A new `set_expanded(id, bool)` intent calls the book and saves.
- **Front-end** — `SessionGrid::sync` at `session_grid/imp.rs:549` takes the
  `WorkspaceBook`. Layout and focus come from the shown book, and each entry's
  placement from `WorkspaceBook::placement`. An account not in the shown
  workspace is set to `OffGrid` explicitly; today an unfound entry keeps its
  stale placement (`:555`). Every view stays mapped, and none is built,
  destroyed, stopped or reloaded.
- **Front-end** — `session_sidebar/imp.rs` replaces the flat `ListStore` of `Row`
  at `:124`:
  - Its root is a `gio::ListStore` of a new `WorkspaceRow`
    (`session_sidebar/workspace_row.rs` plus `imp`, holding id, name and
    is-empty).
  - The root is wrapped in `gtk::TreeListModel::new(root, false, false,
    create_model_func)`. The function returns a `ListStore` of `Row` for a
    `WorkspaceRow` and `None` for a `Row`. Passthrough must be false so
    `TreeExpander` gets its list row.
  - The model stays behind `gtk::NoSelection`, for the reason given at `:125`.
- **Front-end** — `sync` takes the `WorkspaceBook`, rebuilds the root and child
  stores, then walks the flat model calling `set_expanded` from each workspace's
  `is_expanded`. A guard flag stops that re-application from emitting expansion
  intents. `Row::refresh` at `row.rs:38` takes visibility from `placement`, and
  `current` only for the shown workspace, so `status_key` at `row.rs:67` and
  `name_markup` need no new arm (design rules 1 and 3).
- **Front-end** — `row_factory` at `:212` builds a `gtk::TreeExpander` whose
  child box is a superset of both layouts, which later slices extend with a tick
  box and a heading menu:
  - Heading: the name label and the dim "No accounts" label.
  - Account: name, dot, keep-awake mark and ⋯ `MenuButton`.
  - `bind` reads the item type, sets the expander's list row and shows one
    layout. `hide-expander` is set on account rows and `indent-for-depth` on
    both (GTK 4.10+).
  - Heading rows set `ListItem:activatable` false. The expander's
    `notify::expanded` reports `connect_expansion_toggled(WorkspaceId, bool)`.
  - `ListItem:focusable` is not used because it needs GTK 4.12. Each row is
    accepted as two focus stops.
- **Design** — `session-sidebar.ui` `width-request` at `:6` becomes 200
  (`FR.16.5`). Add this step to `docs/design.md` rule 6's record of width
  changes. Rules 1, 3 and 6 bind.
- **Testing** — core unit tests go at the foot of `workspace_book.rs`. The
  `(manual)` checks run against the headless harness, seeded with a
  hand-written version 2 `sessions.toml`, in this task's section of
  `test-script.md` (architecture rule 14).

## Acceptance criteria

- [x] `(unit)` `placement` reports each shown account's own visibility and
      `OffGrid` for every account in another workspace
- [x] `(unit)` `focus_account` on an account in another workspace returns a
      `Switch` naming both workspaces and puts the account in the new
      workspace's focused place. The outgoing workspace's seating and focused
      place stay exactly as they were
- [x] `(unit)` `focus_account` on an account in the shown workspace returns
      `None` and focuses exactly as `SessionBook::focus_session` does
- [x] `(unit)` `set_layout` changes only the shown workspace's layout, and
      `set_expanded(false)` on a workspace shows as `is_expanded: false` in
      `saved()`
- [x] `(manual)` seeded with a named workspace "Party" of two accounts and two
      accounts in `Ungrouped`, the 200-wide sidebar shows the Party heading, then
      `Ungrouped` last. Each account appears once, nested, with its name in full,
      and neither heading has a dot or bold
- [x] `(manual)` clicking an account under the hidden workspace switches to it.
      Its accounts fill the places, the 1/2/4 buttons show its layout, the clicked
      account holds the active place, and the outgoing accounts' rows show the
      background key. A kept-awake game in the outgoing workspace keeps ticking
      with no reload
- [x] `(manual)` switching back restores the first workspace's layout and
      focused place exactly, and each switch rewrites `sessions.toml` once with
      the new `active`
- [x] `(manual)` pressing a 1/2/4 button changes only the shown workspace.
      Switching to the other workspace shows its own layout, unchanged
- [x] `(manual)` collapsing a heading hides its accounts without changing the
      shown workspace, and after a relaunch that heading is still collapsed
- [x] `(manual)` a named workspace with no accounts in the file keeps its
      heading, and expanding it shows one dim "No accounts" line

## References

- [Roadmap item](../../roadmap/11-account-workspaces/README.md) — the full
  picture, including the "Switching workspace from the sidebar" diagram this
  slice implements end to end
- [Wireframes](../../roadmap/11-account-workspaces/wireframes/) — this slice
  draws the tree in `sidebar-tree.md`, not its tick boxes or heading menus
- [`docs/research/gtk4-drag-and-accordion.md`](../../research/gtk4-drag-and-accordion.md)
  — lines 90–133, the tree-list findings applied here
- [`docs/requirements.md`](../../requirements.md) — `FR.15.7`, `FR.15.9`,
  `FR.16.1`–`FR.16.5`, `FR.18.1`
- [`docs/design.md`](../../design.md) — rules 1, 3, 6
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 21, 23, 24
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7, 9

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
