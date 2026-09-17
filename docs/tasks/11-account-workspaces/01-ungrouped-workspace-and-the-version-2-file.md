# 01 — Ungrouped workspace and the version 2 file

**Roadmap:** [11](../../roadmap/11-account-workspaces/README.md) · **Scope:** back-end · **Depends on:** —

## Context

This application runs several accounts of browser idle games side by side in one
window. The window is split into one, two or four places, and every account
either sits in one of them or keeps running out of sight. All of that is saved to
a small settings file in the user's data folder, so the next launch opens the
same arrangement. Today the file holds exactly one arrangement for the whole
window.

The roadmap item this slice belongs to adds workspaces: named sets of up to four
accounts, each with its own arrangement. This slice builds what everything else
stands on, and a person using the app sees nothing new. After it, the app keeps a
list of workspaces instead of one arrangement. Every account sits in a built-in
workspace called Ungrouped, which behaves exactly like today's window. The
sidebar and the grid keep showing that one workspace.

Three things change underneath. First, each account's hidden number, which also
names its folder of website data on disk, is now handed out by one counter for
the whole app, and that counter is written to the file. Today the next number is
worked out from the accounts that exist. That would reuse a number after the
newest account is deleted, and a new account on a dead account's folder can
inherit its leftovers or crash the engine process every account shares. Second,
each workspace remembers its active place and whether its sidebar heading is
expanded, because both must survive a relaunch. Third, the file moves to a second
format version. An old file opens as one Ungrouped workspace with nothing lost,
and the first save keeps a copy of the old file beside it, so an older build of
the app can still read it.

This is one slice because the saved value's type crosses every layer. Changing
it breaks the rules layer, the file store and the window's save path at once, and
none of them compiles without the others.

## Technical details

- **Back-end** — add `WorkspaceId(String)` beside `SessionId` (code standards
  rule 2). Named workspaces are minted as `workspace-NNNN`. `Ungrouped` has the
  fixed id `ungrouped`, reached through a constructor, so nothing compares
  against a display name. Add `workspace_name(raw: &str) -> Option<String>`
  beside `account_name` at `crates/idle-manager-core/src/session.rs:46`: trimmed,
  `None` when empty.
- **Back-end** — `SessionBook` loses its `minted` counter
  (`crates/idle-manager-core/src/session.rs:260`). `add` and `add_from_preset`
  take the `SessionId` as their first argument. Add `session(&self, id) ->
  Option<&Session>` and use it in place of the `iter().find` repeated at
  `crates/idle-manager-shell/src/window/imp.rs:485`, `:624` and `:681`. Every
  existing placement, focus, zoom, move and rename method is left as it is.
- **Back-end** — `workspace.rs`: `Workspace` gains `id: WorkspaceId`,
  `name: String`, `focused: SlotId` and `is_expanded: bool` beside `accounts` and
  `layout` (`FR.15.2`). Focus is now saved, where item 07 always restored it to
  the first place (`session.rs:340`). Add `WorkspaceList { workspaces, active:
  WorkspaceId, next_account_number: u64, next_workspace_number: u64 }`, saved and
  restored whole. `SessionBook::restore` and `SessionBook::workspace` carry the
  new fields.
- **Back-end** — a new `workspace_book.rs` holds `WorkspaceBook` (naming rule 9).
  It keeps an ordered list of entries, each with a `WorkspaceId`, a name,
  `is_expanded` and a `SessionBook`, plus the active id and both counters.
  `Ungrouped` always exists and is always last. This slice adds:
  - `restore(WorkspaceList) -> Self`: an `active` naming no workspace falls back
    to `Ungrouped`, a missing `Ungrouped` is created empty, and a
    `next_account_number` at or below a restored account's number is raised
    above it.
  - `saved(&self) -> WorkspaceList`, `active(&self) -> &SessionBook` and
    `workspaces(&self)`.
- **Back-end** — `WorkspaceBook` also gains:
  - `add` and `add_from_preset(workspace, …) -> Result<SessionId,
    WorkspaceRefusal>`. They mint from `next_account_number` and never from the
    highest id. A named workspace that is already full is refused. Adding never
    switches (`FR.15.3`, `FR.17.9`).
  - `park`, `rename`, `set_keep_awake`, `unpark`, `mark_started` and the zoom
    steps, each passed to the account's own book. Zoom steps act only on the
    shown workspace's accounts (`FR.11.7`).
  - `start_order` puts the shown workspace's queued accounts first, then every
    other workspace's in list order (`FR.18.4`). `live_session_count` sums every
    book (`FR.18.2`).
  - `WorkspaceRefusal` is an enum with `EmptyName`, `NameTaken`, `NoRoom`,
    `Ungrouped`, `UnknownWorkspace` and `UnknownAccount` (code standards rules 1
    and 12). Named capacity is a constant derived from `Layout::Grid`'s slot
    count, not a bare 4 (rule 5).
- **Back-end** — `WorkspaceStore::read` and `write` at
  `crates/idle-manager-core/src/ports.rs:154` carry `WorkspaceList`.
  `crates/idle-manager-store/src/session_file.rs` moves to `FORMAT_VERSION = 2`:
  - Keep the version 1 types as `SessionFileV1`. Read them as one expanded,
    active `Ungrouped` workspace: accounts in file order, the file's layout,
    focus on the first place, and `next_account` one above the highest id.
  - Version 2 is `version`, `active`, `next_account` and `next_workspace`, then
    `[[workspace]]` with `id`, `name`, `layout`, `focused` and `expanded`, and
    `[[workspace.account]]` entries exactly as today's `SessionEntry`.
  - The probe at `:211` dispatches on 1 or 2, and anything else is still
    `UnknownVersion`.
  - An account id appearing twice is `Malformed`. A named workspace holding more
    than four accounts keeps the first four and appends the rest to `Ungrouped`
    with a `tracing::warn`.
  - The first save over a version 1 file first copies it, once, to
    `sessions.v1.toml` beside it.
- **Front-end** — plumbing only; nothing on screen changes:
  - `window/imp.rs` replaces `book: RefCell<SessionBook>` at `:65` with
    `RefCell<WorkspaceBook>` and routes every intent to it.
  - `restore_workspace` at `:307` restores a `WorkspaceList`, builds a dormant
    holder for every account in every workspace, and starts the queue from
    `start_order`.
  - `request_save` at `:278` hands over `saved()`. `create_account` at `:590`
    adds into the shown workspace.
  - The memory footer counts with `live_session_count`.
  - `save_on_change.rs` changes `Workspace` to `WorkspaceList` in `latest` and
    `request`.
  - The grid and sidebar keep reading `active()`. The tree and hiding other
    workspaces' accounts come later.
- **Testing** — core unit tests go at the foot of `workspace_book.rs` (code
  standards rules 21, 23, 24). Store integration tests go under
  `crates/idle-manager-store/tests/`, with a new `insta` snapshot of the version
  2 file. The one `(manual)` check goes in `test-script.md` (architecture rule
  14). As the item's first slice to reach acceptance, this task writes
  `## Setup` and `## Teardown`.

## Acceptance criteria

- [x] `(unit)` a `WorkspaceList` holding a named workspace and `Ungrouped`, each
      with its own layout, focused place, expansion and accounts, comes back
      field for field from `WorkspaceBook::restore` followed by `saved`
- [x] `(unit)` restoring a list whose `active` names no workspace shows
      `Ungrouped`, and restoring a list with no `Ungrouped` adds an empty one as
      the last workspace
- [x] `(unit)` minting reads `next_account_number` rather than the highest id:
      a list whose counter is above every account mints the counter's number, and
      a counter left below a restored account's number is raised above it
- [x] `(unit)` adding into a named workspace that already holds four accounts
      returns `NoRoom` and changes nothing, and adding into a workspace that is
      not shown leaves the active workspace unchanged
- [x] `(unit)` `start_order` lists the shown workspace's queued accounts before
      every other workspace's, and `live_session_count` counts live accounts in
      every workspace
- [x] `(integration)` a version 1 `sessions.toml` reads as one active, expanded
      `Ungrouped` workspace with the accounts in file order, the file's layout,
      focus on the first place and `next_account` one above the highest id
- [x] `(integration)` a version 2 list writes the file pinned by the new `insta`
      snapshot and reads back equal to what was written
- [x] `(integration)` a version 2 file naming one account id twice reads as
      `Malformed`, and one whose named workspace holds five accounts reads with
      the first four kept and the fifth at the end of `Ungrouped`
- [x] `(integration)` the first write over a version 1 file leaves a byte-equal
      `sessions.v1.toml` beside it, and a second write does not replace that copy
- [x] `(manual)` an existing installation with a version 1 file opens with the
      same accounts, layout and running games as before, and after a change
      `sessions.toml` reads `version = 2` with `sessions.v1.toml` beside it

## References

- [Roadmap item](../../roadmap/11-account-workspaces/README.md) — the full
  picture; its Back-end section is the source of every bullet above
- [`docs/requirements.md`](../../requirements.md) — `FR.15.2`, `FR.15.3`,
  `FR.17.9`, `FR.18.2`, `FR.18.3`, `FR.18.4`, `FR.21.7`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 2, 3, 5, 6, 7, 8, 9,
  11, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 2, 5, 7, 12, 13,
  14, 15, 17, 18, 21, 23, 24
- [`docs/naming.md`](../../naming.md) — rules 6, 9, 10, 11, 12

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
