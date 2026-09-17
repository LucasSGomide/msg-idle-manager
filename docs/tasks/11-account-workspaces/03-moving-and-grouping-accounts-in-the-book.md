# 03 — Moving and grouping accounts in the book

**Roadmap:** [11](../../roadmap/11-account-workspaces/README.md) · **Scope:** back-end · **Depends on:** 02

## Context

This application runs several accounts of browser idle games in one window, and
groups them into workspaces. A workspace is a named set of at most four accounts
with its own arrangement of places. A built-in workspace called Ungrouped holds
every account not put anywhere else, has no size limit, and can be neither
renamed nor removed. Earlier slices built the list of workspaces and switching
between them. This slice builds the rules for changing who belongs where, in the
rules layer only, with no screen involved.

It adds every operation that reorganises the list. Moving a set of accounts to a
workspace is all or nothing: if the destination cannot take the whole set, nothing
moves. An arriving account joins the bottom of its new workspace's list and takes
a free place if there is one, but never pushes out an account already seated. The
account it leaves behind leaves an empty place, and moving never changes which
workspace is on screen. The rules can also say which workspaces have room for a
given number of accounts. The move menu and the add-game window both ask that one
question, so they can never offer a destination the rules would refuse.

Creating a workspace from a set of accounts needs a name that is not empty and
not the same as any other workspace's, ignoring capital letters, Ungrouped
included. Renaming follows the same rule. Removing a workspace sends its accounts
to the bottom of Ungrouped in order, touches nothing on disk, and shows Ungrouped
if the removed workspace was on screen. Last, forgetting an account entirely,
which the delete feature calls only once its folder is gone, leaves its place
empty and never frees its number for a new account.

Each refusal comes back as a named reason rather than a yes or no. That lets the
screen grey out a button from the same answer the rules would give. It is its own
slice because every later screen slice asks these questions, and they can be
proved completely with fast tests.

## Technical details

- **Back-end** — `SessionBook::take(&mut self, id) -> Option<Session>` removes
  the session and its remembered slot, leaving its place empty and moving no
  other account (`FR.17.8`, `FR.21.5`). `adopt(&mut self, session: Session)`
  appends an arriving account to the end of `sessions`. It seats the account in
  the lowest free slot of this book's layout if one is free and off-grid
  otherwise, never displacing anyone (`FR.17.3`).
- **Back-end** — `WorkspaceBook::destinations(&self, count) -> Destinations`:
  named workspaces with room for `count` in list order, `can_create` (whether a
  new workspace can hold `count`, at most the named capacity), and `Ungrouped`.
  It is the single source for the move menu and the add-game field (`FR.17.2`,
  `FR.17.5`).
- **Back-end** — `move_accounts(&mut self, ids: &[SessionId], to) -> Result<(),
  WorkspaceRefusal>` is all or nothing on room. Accounts already in `to` are
  skipped. Each account is moved with `take` then `adopt`, and `active` never
  changes (`FR.17.8`). An unknown id gives `UnknownAccount`, and an unknown
  workspace gives `UnknownWorkspace`.
- **Back-end** — `create_workspace(&mut self, name, ids) -> Result<WorkspaceId,
  WorkspaceRefusal>`:
  - It runs `workspace_name` and refuses a name equal to another workspace's,
    ignoring case, `Ungrouped` included (`FR.15.8`).
  - It refuses more accounts than the named capacity.
  - It mints `workspace-NNNN` from `next_workspace_number`, inserts the
    workspace before `Ungrouped`, expanded, with a new book's default layout,
    then moves `ids` in.
- **Back-end** — `rename_workspace(&mut self, id, name) -> Result<(),
  WorkspaceRefusal>` refuses `Ungrouped` and a name taken by *another*
  workspace. The workspace's own current name, in any case, is accepted.
- **Back-end** — `remove_workspace(&mut self, id) -> Result<(),
  WorkspaceRefusal>` refuses `Ungrouped`. It adopts every account into
  `Ungrouped` in order and shows `Ungrouped` if the removed workspace was shown
  (`FR.15.4`, `FR.15.11`). It touches no disk.
- **Back-end** — `remove_account(&mut self, id) -> bool` calls `take` on the
  account's own book. It is the one call that forgets an account, made only after
  its folder is gone (`FR.21.5`), and it leaves `next_account_number` untouched
  (`FR.21.7`). Every method returns an owned result and never makes a partial
  change (code standards rules 1 and 12).
- **Testing** — unit tests go at the foot of `workspace_book.rs` and
  `session.rs` (code standards rules 21, 23, 24). This task's `test-script.md`
  section holds the `cargo test` runs for those modules and their pass counts. The
  screen slices that use these rules prove them end to end.

## Acceptance criteria

- [x] `(unit)` `SessionBook::take` removes an account and its remembered slot,
      leaving its place empty and every other account seated where it was
- [x] `(unit)` `SessionBook::adopt` appends the account to the end of the list.
      It seats the account in the lowest free place when one is free and off-grid
      when none is, and never moves a seated account
- [x] `(unit)` `destinations(2)` lists, in list order, only the named workspaces
      with room for two, then `Ungrouped`, and `destinations(5)` has
      `can_create` false and no named workspace
- [x] `(unit)` `move_accounts` to a workspace without room for the whole set
      returns `NoRoom` and leaves every workspace exactly as it was
- [x] `(unit)` moving an account out of the shown workspace leaves `active`
      unchanged and the account's former place empty, and ids already in the
      destination are skipped
- [x] `(unit)` `create_workspace` refuses an empty name (`EmptyName`), "party"
      when "Party" exists and "ungrouped" (`NameTaken`), and five accounts
      (`NoRoom`), changing nothing each time
- [x] `(unit)` `create_workspace` inserts the new workspace before `Ungrouped`,
      expanded, holding exactly the given accounts. A save and restore followed
      by another create never mints the same `workspace-NNNN`
- [x] `(unit)` `rename_workspace` refuses `Ungrouped` and another workspace's
      name ignoring case. It accepts the workspace's own name in a different
      case, and renames otherwise
- [x] `(unit)` `remove_workspace` refuses `Ungrouped`. Otherwise the removed
      workspace's accounts join the end of `Ungrouped` in their order, and
      removing the shown workspace makes `Ungrouped` active
- [x] `(unit)` `remove_account` leaves the account's place empty, and after a
      save and restore the next add never mints the removed account's number

## References

- [Roadmap item](../../roadmap/11-account-workspaces/README.md) — the full
  picture, including the "Grouping accounts in selection mode" diagram whose
  book calls this slice provides
- [`docs/requirements.md`](../../requirements.md) — `FR.15.3`, `FR.15.4`,
  `FR.15.8`, `FR.15.9`, `FR.15.11`, `FR.17.2`, `FR.17.3`, `FR.17.5`, `FR.17.8`,
  `FR.21.5`, `FR.21.7`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 2, 7
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 5, 12, 21, 23,
  24
- [`docs/naming.md`](../../naming.md) — rules 9, 10, 11

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
