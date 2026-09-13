# 03 — Moving an account between places in the book

**Roadmap:** [10](../../roadmap/10-rearranging-accounts/README.md) · **Scope:** back-end · **Depends on:** 01

## Context

The application's window is split into one, two or four places, and each game
account either sits in one of them or waits out of sight. Today a simple rule
seats an account in the lowest free place, and the user cannot change that. A
later slice lets them drag an account onto another place. This slice builds what
that drag asks for, in the rules layer, with no screen involved.

It adds one operation: move this account to that place. It has exactly three
results. If the target place holds another account, the two trade places and
nothing else moves, so repeating the move undoes it. If the target is empty, the
account moves in and its old place is left empty. If the target is the place the
account already holds, is not a place the current arrangement has, or the account
is out of sight or unknown, nothing changes at all.

A real move carries three side effects, each needed for the result to feel
right. First, both accounts' new places are remembered, so switching to a
different arrangement and back puts them where the user put them rather than
where the old rule would. Second, the active account stays active. That is the
one the keyboard shortcuts and the zoom wheel act on, and the application tracks
it by place number, so the rules layer has to move that number along with the
account. Moving accounts around is not the same act as choosing which one to
look at. Third, the account list is reordered to read like the window: accounts
on screen first, in order of their place, then any accounts out of sight in the
order they already had. The sidebar shows that order, and the saved file is
written in it. When the workspace is restored, earlier accounts get first claim
on a remembered place, so the arrangement comes back as it was left.

A move never pauses, starts, reloads or resizes a game. The size a game is drawn
at is remembered per arrangement, and moving an account does not change the
arrangement.

## Technical details

- **Back-end** — `crates/idle-manager-core/src/layout.rs` gains `MoveOutcome`
  with three variants: `Swapped { with: SessionId }` when the target held another
  account, which now sits in the mover's old place; `Filled` when the target was
  empty; `Unchanged` otherwise. It is deliberately not the existing `Outcome` at
  `crates/idle-manager-core/src/layout.rs:119`. That type's `Swapped` promises the
  displaced account goes off-grid, and one variant must not mean two things (code
  standards rule 1). This departs from `FR.14.2`'s wording, which named the old
  variants; the behaviour is unchanged.
- **Back-end** — a pure `move_into_slot(current, layout, account, target) -> Move`
  beside `bring_into_focus` at `crates/idle-manager-core/src/layout.rs:176`, in
  the same shape: it reads every account's visibility and returns the full
  visibility map after the call plus the outcome. `Unchanged` when the target is
  the mover's own slot, is not a slot the layout has, or the mover is off-grid or
  unknown.
- **Back-end** — `SessionBook::move_to_slot(&mut self, account: &SessionId,
  target: SlotId) -> MoveOutcome` wraps it the way `focus_session` at
  `crates/idle-manager-core/src/session.rs:699` wraps `bring_into_focus`. On
  `Swapped` or `Filled` it writes both visibilities and records both accounts'
  places in `remembered`, so a later layout switch returns them there (`FR.3.2`).
  On `Unchanged` it changes nothing at all.
- **Back-end** — focus is a slot index, so keeping it on an account means: if
  `focused` was the mover's slot it becomes `target`; if `focused` was `target`
  and the result is `Swapped` it becomes the mover's old slot; otherwise it is
  unchanged (`FR.14.3`).
- **Back-end** — on a real move a private method rewrites the order of
  `sessions` with a stable sort: accounts with `Visibility::InSlot` first by slot
  index, then off-grid accounts in their existing relative order (`FR.14.7`). An
  unchanged move, a rename, a layout switch and a focus change leave the order
  alone.
- **Back-end** — no liveness, keep-awake flag or remembered zoom is read or
  written (`FR.14.8`). The store writes accounts in book order at
  `crates/idle-manager-store/src/session_file.rs:310`, so the new order persists
  with no format change. The doc comments on `SessionBook::sessions`,
  `SessionBook::workspace` and `Workspace::accounts` that say "the order they were
  added" become "the order they sit in".
- **Architecture** — core only: no change in `idle-manager-store` or the
  composition root (architecture rules 1, 7, 8, 9). `docs/code-standards.md`
  rules 1, 3, 17, 21 and 23 and `docs/naming.md` rules 6, 9, 11 and 12 bind.
- **Testing** — unit tests at the foot of `layout.rs` and `session.rs` (code
  standards rule 24). This task's `test-script.md` section is the `cargo test`
  run for the new tests and its result. If this is the item's first accepted
  slice, it also writes `## Setup` and `## Teardown`.

## Acceptance criteria

- [x] `(unit)` moving an account onto another account's slot returns
      `Swapped { with }` naming that account, trades exactly those two slots, and
      leaves every other account's visibility equal to before
- [x] `(unit)` moving an account onto an empty slot returns `Filled`, puts it in
      the target and leaves its old slot holding no account
- [x] `(unit)` moving onto the account's own slot, onto a slot the layout does
      not have, moving an off-grid account and moving an unknown id each return
      `Unchanged` and leave the book equal to before
- [x] `(unit)` with the focused slot being the mover's, a swap and a fill both
      set `focused` to the target
- [x] `(unit)` with the focused slot being the target, a swap sets `focused` to
      the mover's old slot, so the account that had focus keeps it
- [x] `(unit)` with the focused slot being neither the mover's nor the target,
      `focused` is unchanged after a swap
- [x] `(unit)` after a swap, switching to another layout and back puts both
      accounts in the slots the swap gave them
- [x] `(unit)` after a real move `sessions` lists in-slot accounts by slot index
      then off-grid accounts in their previous relative order, both with and
      without off-grid accounts present
- [x] `(unit)` moving a parked account keeps it parked, and no account's
      keep-awake flag or remembered zoom changes
- [x] `(unit)` restoring a new book from the workspace of a book after a move
      reproduces the same slots and the same order

## References

- [Roadmap item](../../roadmap/10-rearranging-accounts/README.md) — the full
  picture, including the "Dragging an account to another place" diagram's
  session-book step and the "How the list order follows a drop" flowchart this
  slice implements
- [`docs/requirements.md`](../../requirements.md) — `FR.3.2`, `FR.14.2`,
  `FR.14.3`, `FR.14.7`, `FR.14.8`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 7, 8, 9, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 3, 17, 21, 23,
  24
- [`docs/naming.md`](../../naming.md) — rules 6, 9, 11, 12

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
