# 08 — Deleting an account

**Roadmap:** [11](../../roadmap/11-account-workspaces/README.md) · **Scope:** front-end · **Depends on:** 04, 07

## Context

This application runs several accounts of browser idle games in one window. Each
account keeps a folder on disk with the game's logins and saved data. Today an
account that is no longer played can only be left in the sidebar forever, with
its folder still on disk. Earlier slices added the rule that forgets an account
and a safe way to remove one account's folder. This slice puts them behind a menu
item a person can use.

Each account's "⋯" menu gains "Delete account…" as its last entry, for any
account in any state or workspace. It opens a window asking for confirmation by
name and saying the logins and saved game data will be removed and cannot be
recovered. Cancel leaves everything as it was. Once the red Delete account
button is pressed, the window shows a spinner and cannot be closed. Data already
cleared cannot be put back, so stopping half way would only leave a mess.

Behind the spinner the app works in a fixed order. It pauses the account, asks
the browser engine to wipe all of that account's website data, stops its game and
throws away its view, waits a quarter of a second for the engine to close the
files it is willing to close, then removes the folder. Only when the folder is
gone does the account disappear from the sidebar and the grid. Its place stays
empty and no other account moves. Because the app never reuses an account's
number, a game added later gets a fresh folder.

If removing the folder fails, for example because it is read-only, the window
changes to say so. It shows the reason and the folder's path, which can be
selected and copied, and offers Retry and Close. Retry runs the whole sequence
again, which is safe because every step does nothing when it has already
happened. Close leaves the account in its workspace, paused and logged out, shown
as any paused account is, and ready to be deleted again later.

This is the last slice because it needs the removal port and edits the main
window after every other slice has.

## User experience

- **Entry** — an account row's ⋯ menu gains `Delete account…` as its last item,
  below `Rename…`. It is always sensitive, for an account in any workspace or
  state.
- **Flow** — row ⋯ → `Delete account…`. A modal window titled "Delete account?"
  reads "Delete <name>? Its logins and saved game data will be removed from this
  computer. This cannot be undone.", with `Cancel` and a red `Delete account`
  button.
- **Flow** — confirm. The buttons go, and a spinner reads "Deleting <name>…".
  The window cannot be closed or cancelled. On success it closes, and the account
  is gone from the sidebar and the grid; its place stays empty.
- **States** — **delete fails**: the spinner is replaced by "Couldn't finish
  deleting <name>.", the reason on one dim line, and the folder path on a second
  dim, selectable line, with `Retry` and `Close`. `Retry` runs the whole deletion
  again. `Close` leaves the account in its workspace, parked and logged out, with
  its row unchanged in shape — a grey parked dot.
- **Pattern** — `Delete account…` is one more item in the row's ⋯ menu, design
  rule 5. A parked account left behind by a failed delete uses the existing
  `parked` key and slot panel, design rules 1 and 4; no new state is added.
- **New pattern** — a destructive confirmation that becomes a blocking progress
  window and then, on failure, an error with retry. Rule 9's message strip is for
  problems the user did not cause, and this is the result of an action they are
  watching. The design doc owes a rule once this ships.

## Technical details

- **Front-end** — `bind_row_menu` in `session_sidebar/imp.rs` appends `Delete
  account…`, bound to a stateless `delete` action whose name is a constant. The
  action reports `connect_delete_requested(SessionId)` and is always sensitive
  (`FR.21.1`).
- **Front-end** — a new `DeleteAccountDialog` lives in
  `delete_account_dialog.rs`, `delete_account_dialog/imp.rs` and
  `resources/ui/delete-account-dialog.ui`, registered in the gresource
  (architecture rules 12 and 13; naming rules 2, 4 and 7). It is a modal,
  non-resizable `gtk::Window` holding a `gtk::Stack` of three pages. A stack is
  safe here: it holds labels and buttons, never a web view.
- **Front-end** — the dialog's three pages:
  - `confirm`: the sentence naming the account, `Cancel`, and `Delete account`
    carrying `destructive-action`.
  - `working`: a `gtk::Spinner` and "Deleting <name>…" with no buttons. On this
    page `deletable` is false, `close-request` returns `Propagation::Stop` and
    Escape does nothing (`FR.21.9`).
  - `failed`: the sentence, then the reason and the folder path in `dim-label`
    labels (the path `selectable`), with `Retry` and `Close`.

  The dialog emits `connect_confirmed` and `connect_retry` and exposes
  `show_working` and `show_failed(reason, folder)`. It decides nothing.
- **Front-end** — `SessionView::clear_data` awaits
  `NetworkSession::website_data_manager()` then
  `WebsiteDataManager::clear(WebsiteDataTypes::ALL, glib::TimeSpan(0), None,
  callback)`, both in `webkit6` 0.6.1. `SessionGrid::remove_session` unparents
  the account's overlay and drops its entry.
- **Front-end** — a new `account_deletion.rs` holds the async sequence, so the
  window stays one level of abstraction (code standards rule 6). It runs with
  `glib::spawn_future_local`, and every blocking call goes through
  `gio::spawn_blocking` (architecture rule 10). The order (`FR.21.10`):
  1. `WorkspaceBook::park`.
  2. `clear_data`.
  3. `SessionView::stop`, then drop the holder from `holders` and call
     `remove_session`.
  4. `glib::timeout_future` for `DELETE_SETTLE_MILLIS = 250`, twice the slowest
     storage close measured.
  5. `ProfileRemoval::remove`.

  A step whose holder is already gone is skipped, so `Retry` is safe.
- **Front-end** — on success: call `WorkspaceBook::remove_account`, cancel any
  pending zoom-save timer for the id (`window/imp.rs:88`), drop its
  `load_changed_handlers` entry (`:97`), prune it from the sidebar's ticked set,
  close the dialog, then redraw and save. On failure: call `show_failed` with the
  error's reason and `ProfileRemoval::folder(id)`. `Close` then redraws and saves
  with the account parked. A sign-in popup is left alone (`FR.21.12`). The start
  queue already skips an id no longer queued (`start_queue.rs:146`).
- **Front-end** — a window intent `present_delete_dialog(id)` opens the dialog
  transient for the window. `WindowPorts` gains the `ProfileRemoval` port, which
  `attach_ports` at `window/imp.rs:261` stores. `crates/idle-manager/src/main.rs`
  builds `XdgProfileRemoval` and passes it beside the existing five ports
  (architecture rule 3).
- **Testing** — `(manual)` against the headless harness in this task's section
  of `test-script.md` (architecture rule 14), plus a `(unit)` check that the new
  template is in the resource bundle. Design rules 1, 4 and 5 bind.

## Acceptance criteria

- [x] `(unit)` `delete-account-dialog.ui` is readable from the compiled resource
      bundle, as the window, add-game and rename templates are
- [x] `(manual)` the ⋯ menu of a live, a parked and a queued account, in the
      shown and in a hidden workspace, lists `Delete account…` last, below
      `Rename…`, and it is sensitive in every case
- [x] `(manual)` choosing it opens a modal "Delete account?" window with the
      exact sentence naming the account, `Cancel` and a red `Delete account`.
      `Cancel` closes it with the account and its profile folder unchanged
- [x] `(manual)` confirming shows a spinner reading "Deleting <name>…" with no
      buttons, and while it shows, the window's close button and Escape do
      nothing
- [x] `(manual)` on success the window closes, the row is gone from the sidebar,
      its place is empty, and every other account is unmoved. The profile folder
      no longer exists, and `sessions.toml` no longer lists the account
- [x] `(manual)` after a relaunch the deleted account does not reappear, its
      folder has not been recreated, and another account still loads logged in
- [x] `(manual)` with the account's profile folder made unremovable (its parent
      read-only), confirming shows "Couldn't finish deleting <name>.", a reason
      line, and a selectable folder path, with `Retry` and `Close`
- [x] `(manual)` restoring write permission and pressing `Retry` completes the
      deletion and closes the window
- [x] `(manual)` pressing `Close` after a failure leaves the account in its
      workspace with a grey parked dot and the parked panel in its place, and
      deleting it again later succeeds
- [x] `(manual)` deleting the newest account and then adding a game gives the
      new account a higher number and a different profile folder than the
      deleted one

## References

- [Roadmap item](../../roadmap/11-account-workspaces/README.md) — the full
  picture, including the "Deleting an account" diagram this slice implements and
  the Technical References measurement behind the 250 ms wait
- [Wireframes](../../roadmap/11-account-workspaces/wireframes/) — this slice
  draws `delete-account-window.md`, and the `Delete account…` item in the row
  menu of `sidebar-tree.md`
- [`docs/requirements.md`](../../requirements.md) — `FR.21.1`, `FR.21.2`,
  `FR.21.5`, `FR.21.7`–`FR.21.12`
- [`docs/design.md`](../../design.md) — rules 1, 4, 5, 9
- [`docs/architecture.md`](../../architecture.md) — rules 3, 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rule 6
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
