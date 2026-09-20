# 07 — Park all and Start all

**Roadmap:** [14](../../roadmap/14-keyboard-navigation/README.md) · **Scope:** full-stack · **Depends on:** 06

## Context

This application keeps several browser idle games running at once on a
desktop that stays on all day. Each game is an account, and an account can be
"parked": its browser is torn down and its memory handed back, its place on
screen shows a plain stopped panel with a `Start` button, and starting it
again brings the game back. Accounts are grouped into workspaces, each with a
heading in the sidebar, and today parking or starting a whole workspace means
opening one row menu per account.

This slice adds `Park all` and `Start all` to the top of every workspace
heading's ⋯ menu, above the existing `Rename…` and `Remove workspace`, and
gives the built-in `Ungrouped` heading a ⋯ menu of its own holding exactly
those two. `Park all` parks every account in the workspace that is not
already parked, with no confirmation, because `Start all` undoes it. An
account whose game is running is parked at once. One waiting in the start
queue — the queue that brings accounts back one at a time so they never all
load together — simply leaves the queue, which already skips an account that
is no longer waiting. One whose page is mid-load is parked the moment that
page paints, so no second browser is ever built for it. `Start all` puts every
parked account of the workspace into that same queue, in the workspace's
order, and they come back one at a time. Each menu item is greyed when it
would touch nothing: `Park all` with no running, queued or loading account,
`Start all` with no parked one, both on an empty workspace.

Two things ride along. The start queue becomes one object that lives as long
as the window, with a way to append to it, instead of being replaced whole
each time it is needed — replacing it mid-drain would start two accounts at
once. And each workspace heading's name gains the hover text `Next workspace
(Ctrl+Tab)`, since this slice already rebuilds the heading and the pager slice
gave its key the same treatment.

The domain half — the transitions and the two "would it touch anything"
questions — is unit tested; the sidebar and window half is proven by hand. It
is one slice because the menu items are useless without the transitions and
the transitions have no other caller.

## User experience

- **Entry** — `Park all` and `Start all` at the top of every workspace
  heading's ⋯ menu, `Ungrouped`'s included.
- **Flow** — Park all: heading ⋯ → `Park all` → every running, queued or
  starting account in that workspace is parked, no confirmation; its slots
  show the plain stopped panel. `Start all` → every parked account in the
  workspace is queued and comes back one at a time, in order.
- **Flow** — Learn the keys: hover a workspace heading → `Next workspace
  (Ctrl+Tab)`.
- **States** — Heading menu: `Park all` is greyed when the workspace has no
  running, queued or starting account; `Start all` is greyed when it has no
  parked account. An empty workspace greys both. In sidebar selection mode
  the heading menu is hidden, as it is today, so both are unreachable.
- **Pattern** — Menu items whose sensitivity follows the workspace's state,
  exactly as a row's Park/Start item is insensitive when it does not apply
  (design rule 2); the deliberate actions lead the ⋯ menu, above the
  set-and-forget `Rename…` (design rule 5). A parked slot shows the plain centred panel of design rule
  4; `Park all` produces it in every slot at once.
- **Pattern** — A heading stays undecorated: no dot, no bold, no mark of the
  shown workspace (design rule 13). The tooltip is the only thing it gains.

## Technical details

- **Architecture** — in `crates/idle-manager-core/src/session.rs`,
  `SessionBook` gains `queue(id) -> Liveness`, the new `Parked → Queued`
  transition beside `park`, `unpark` and `mark_started`, guarded to act only
  on `Parked`. In `workspace_book.rs`, `WorkspaceBook` gains
  `park_all(&WorkspaceId) -> Vec<(SessionId, Liveness)>` (every account whose
  liveness was not `Parked`, with the liveness it had, after setting each to
  `Parked`; `FR.24.1`), `queue_parked(&WorkspaceId) -> Vec<SessionId>` (every
  `Parked` account set to `Queued` in workspace order; `FR.24.2`),
  `can_park_all(&WorkspaceId) -> bool` and `can_start_all(&WorkspaceId) ->
  bool` (`FR.24.3`).
- **Architecture** — `WorkspaceRow` gains `can_park_all: bool` and
  `can_start_all: bool` properties, filled in `SessionSidebar::sync` from the
  two predicates. `bind_heading_menu` in `session_sidebar/imp.rs` builds the
  menu as two sections: `Park all` and `Start all` bound to
  `HEADING_ACTION_GROUP` actions `park-all` and `start-all` whose `enabled`
  follows the two flags, then the existing `Rename…` and `Remove workspace`.
  `bind_heading` shows the ⋯ button for `Ungrouped` too, outside selection
  mode; for `Ungrouped` the menu holds the first section only (design rule 2).
- **Architecture** — two handlers, `on_park_all_requested` and
  `on_start_all_requested`, each `Option<Box<dyn Fn(WorkspaceId)>>`, with
  `connect_park_all_requested` / `connect_start_all_requested` on
  `SessionSidebar`, following `on_workspace_rename_requested`. The heading's
  name label gets `tooltip-text` `Next workspace (Ctrl+Tab)` in `bind_heading`
  (`FR.25.1`; design rule 13 — a tooltip is not a mark).
- **Architecture** — `window/imp.rs` `wire_sidebar_signals` connects them to
  `park_all(&WorkspaceId)` and `start_all(&WorkspaceId)`. `park_all` calls
  `book.park_all(workspace)` and, per `(id, before)`: `Live` → `stop_view(id)`
  (factored out of `park_session`: `holder.stop()`, clear `watched` if it was
  this id, `grid.release_view(id)`); `Queued` → nothing, since
  `StartQueue::advance` asks `is_queued` before every turn; `Starting` →
  insert into a new `park_on_paint: RefCell<HashSet<SessionId>>`.
  `finish_starting` checks that set after `mark_started`: an id found there is
  removed and goes through `park_session` (`FR.24.1`). One `redraw` and one
  `request_save` after the loop (rule 8).
- **Architecture** — `start_all` calls `book.queue_parked(workspace)` and hands
  the ids to the queue (`FR.24.2`, `FR.8.2`). `start_queue.rs` gains
  `pub(crate) fn enqueue(&self, ids: Vec<SessionId>)` appending to `pending`
  and calling `advance` only when nothing is in flight — a new `draining:
  Cell<bool>` set in `advance` when it starts an id and cleared when it finds
  none queued. The window keeps one `StartQueue` for its lifetime
  (`start_queue: StartQueue`, created in `constructed` with an empty list)
  instead of `RefCell<Option<StartQueue>>`, and `restore_workspace` enqueues
  the restore order. This settles the fourth Blocker.
- **Code standards** — the enqueue decision is a pure helper beside `next_up`
  (`fn enqueue_plan(draining: bool, pending: &[SessionId], ids: &[SessionId])
  -> (Vec<SessionId>, bool)` or equivalent) so it is unit tested without a
  window (rule 25); `queue`, `park_all`, `queue_parked` and the two predicates
  are tested in their files' `tests` modules (rules 21–24).
- **Design** — the stopped panel is the existing `slot-placeholder.ui` (rule
  4); rows re-key through the unchanged `status_key` (rule 1).

## Acceptance criteria

- [x] `(unit)` `park_all` on a workspace with a `Live`, a `Queued`, a
      `Starting` and a `Parked` account returns the first three with their
      prior liveness and leaves all four `Parked`
- [x] `(unit)` `queue_parked` sets every `Parked` account to `Queued` in
      workspace order and returns exactly those ids; `queue` on a `Live`
      account changes nothing
- [x] `(unit)` `can_park_all` is true only with a `Live`, `Queued` or
      `Starting` account, `can_start_all` only with a `Parked` one, and both
      are false on an empty workspace
- [x] `(unit)` the enqueue decision appends to a draining queue without
      starting a second account, and starts the first id on an idle queue
- [x] `(integration)` `make verify` passes
- [ ] `(manual)` heading ⋯ shows `Park all` and `Start all` above `Rename…`;
      `Ungrouped`'s ⋯ shows only the two; each is greyed per the workspace's
      state; in selection mode no heading shows ⋯
- [ ] `(manual)` `Park all` on a workspace with one live, one queued and one
      starting account: the live one's slot shows the stopped panel at once,
      the queued one never starts, the starting one parks the moment its
      page paints, and the debug log shows one view built for it
- [ ] `(manual)` `Start all` on three parked accounts brings them back one at
      a time in sidebar order: rows turn purple, then blue one at a time,
      then green
- [ ] `(manual)` `Start all` during a still-draining launch restore appends
      to the queue: at no point are two accounts `Starting` at once
- [ ] `(manual)` hovering a workspace heading reads `Next workspace
      (Ctrl+Tab)`; the heading gains no dot, bold or other mark

## References

- [Roadmap item](../../roadmap/14-keyboard-navigation/README.md) — Back-end
  "Workspaces" (the `park_all`, `queue_parked` and predicates sentences);
  Front-end "The sidebar" and "Park all and Start all"; the "Park all and
  Start all" diagram; Blockers, the fourth
- [Wireframes](../../roadmap/14-keyboard-navigation/wireframes/) —
  `workspace-heading-menu.md`
- [`docs/requirements.md`](../../requirements.md) — `FR.24.1`–`FR.24.3`,
  `FR.25.1`, `FR.8.2`, `FR.17.6`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 12, 13
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 21–25
- [`docs/design.md`](../../design.md) — rules 1, 2, 4, 5, 13
- [`docs/naming.md`](../../naming.md) — rules 7, 11, 12

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The `(manual)` steps run
under `make dev` with `RUST_LOG=idle_manager_shell=debug` to count the views
built.
