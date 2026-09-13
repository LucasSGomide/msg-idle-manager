# 05 — Dropping an account on a place

**Roadmap:** [10](../../roadmap/10-rearranging-accounts/README.md) · **Scope:** front-end · **Depends on:** 02, 03, 04

## Context

By this point a person can pick a game account up by the small handle in its
place's corner and drag it, and the application's rules layer knows how to move
an account to another place. Nothing yet connects the two: letting go does
nothing. This slice is the drop, and it finishes the feature.

While dragging, whichever place is under the pointer is covered by a translucent
tint that follows the pointer from place to place. The source place and empty
places take the tint too. Dropping on a place holding another account makes the
two trade places. Dropping on an empty place moves the account there and leaves
its old place empty. A paused account's place is a valid target like any other,
and its plain panel moves with it. Dropping on the place the drag started from,
letting go outside the grid, or pressing Escape changes nothing and saves
nothing.

After a real move, the active account stays active wherever it now sits. That is
the one the keyboard shortcuts and the zoom wheel act on, and its outline
follows it. The sidebar list reorders to read like the window: accounts on
screen first, in the order of their places, then the accounts out of sight in
the order they had. The new places and the new order are saved, so the
arrangement comes back on the next launch. No game reloads, stops, starts or
changes size.

The difficult part is that each place is filled by a live web page, and a web
page has its own handler waiting for things to be dropped on it. The grid's drop
handler is attached so that it runs before any page's handler does. A
measurement showed that attached the other way, the page's handler silently wins
and the grid never hears about the drop. There is one drop handler for the whole
grid rather than one per place, because an empty place has nothing of its own to
attach a handler to.

## User experience

- **Flow** — drag. Whichever place is under the pointer is covered by a
  translucent tint, including the source place and empty places, and the tint
  follows the pointer from place to place. The game pages keep running
  underneath and show no drop cursor.
- **Flow** — drop on another occupied place: the two accounts trade places. Drop
  on an empty place: the account moves there and its old place is left empty.
  Either way the account that was active before the drag is still active, with
  the focus outline, in whatever place it now sits, and the sidebar list
  reorders to match the window.
- **Flow** — drop on the source place, outside the grid, or press Escape
  mid-drag: nothing changes and nothing is saved.
- **States** — **drag over a place showing a parked or queued panel**: a valid
  drop, highlighted like any other; the panel moves with its account,
  unrestyled. **Drag over the web page of a game**: the page shows no drop
  cursor and receives nothing.
- **Pattern** — the sidebar's current-row marker (`docs/design.md` rule 1)
  stays on the account that was current before the drag; grabbing or dropping
  never moves it. A parked or queued place keeps rule 4's plain panel.
- **New pattern** — a drop highlight over a place. No rule covers indicating a
  drop target. The design doc owes a rule once this ships, and it should say
  whether the highlight shares a colour with the focused place's outline.

## Technical details

- **Front-end** — the slot arithmetic in `focus_slot_at` at
  `crates/idle-manager-shell/src/session_grid/imp.rs:434` is split out into
  `slot_at(x, y) -> Option<SlotId>`, used by both click-to-focus and the drop.
- **Front-end** — one `gtk::DropTarget` for `DraggedAccount` with
  `DragAction::MOVE` goes on the grid itself, in the capture phase, so it runs
  before `WebKitWebViewBase`'s own drop target on the pages below (`FR.14.5`).
  Measured: in the bubble phase WebKit consumes the drop and the grid never
  receives it. The target is on the grid rather than per place so an empty place
  accepts a drop.
- **Front-end** — on `drop` the grid resolves the slot with `slot_at` and emits
  the account and the slot through a new `SessionGrid::connect_account_dropped`,
  registered like `connect_slot_focused`. It returns `true` when a slot was
  resolved, and `false` otherwise so GTK plays the cancel animation. The grid
  decides nothing (architecture rule 8).
- **Front-end** — a `gtk::DropControllerMotion` on the grid tracks the pointer.
  Its pointer test includes descendants, which `FR.14.6` needs, since the pointer
  is over a web page the whole time. On `motion` it stores the slot under the
  pointer and queues a redraw. On `leave` it clears the slot and queues a redraw.
  It accepts no drops, so it does not compete with the drop target.
- **Front-end** — the grid's `snapshot` draws a tinted rectangle over the stored
  slot after the children and before `draw_slot_lines` at
  `crates/idle-manager-shell/src/session_grid/imp.rs:457`, with its colour and
  alpha as named constants (code standards rule 5).
- **Front-end** — `window/imp.rs` registers `connect_account_dropped`, and the
  handler calls `SessionBook::move_to_slot`. On `Swapped` or `Filled` it calls
  `redraw`, which moves the places and rebuilds the sidebar in the new order,
  then `request_save` (`FR.14.7`). On `Unchanged` it does nothing. It never
  touches a `SessionView` holder or a zoom, so no page reloads or resizes
  (`FR.14.8`). Escape and a release outside the grid are cancelled by GTK and
  never reach the handler.
- **Design** — rules 1, 4 and 6. The drop highlight is a new pattern.
  `docs/design.md` owes a rule for it, including whether it shares a colour with
  the focus outline, written once this slice ships.
- **Testing** — `(manual)`, against the headless harness, in this task's section
  of `test-script.md` (architecture rule 14). No-save checks compare the
  modification time of `sessions.toml` before and after. If this is the item's
  first accepted slice, it also writes `## Setup` and `## Teardown`.

## Acceptance criteria

- [x] `(manual)` while dragging, the place under the pointer is tinted, including
      the source place and an empty place, the tint moves with the pointer, and it
      clears when the pointer leaves the grid
- [x] `(manual)` dropping on another occupied place swaps the two accounts, every
      other place is unchanged, and neither game's page reloads
- [x] `(manual)` dropping on an empty place moves the account there and leaves
      its old place empty
- [x] `(manual)` dropping a live account on a parked account's place swaps them,
      the parked panel now shows in the live account's old place, and that
      account is still parked
- [x] `(manual)` a drop on the source place, a release outside the grid and
      Escape mid-drag each leave every place unchanged and `sessions.toml` not
      rewritten
- [x] `(manual)` after swapping the focused account with another, the focus
      outline is on the focused account's new place and `Ctrl` + plus zooms that
      account
- [x] `(manual)` after a drop the sidebar rows list the on-screen accounts in
      place order, followed by the off-grid accounts in their previous order
- [x] `(manual)` a drop released over a text box inside a game's page moves the
      account and leaves the text box empty, and no drop cursor shows over the
      page during the drag
- [x] `(manual)` after a drop and a relaunch, every account is back in the place
      the drop left it and the sidebar order is the same

## References

- [Roadmap item](../../roadmap/10-rearranging-accounts/README.md) — the full
  picture, including the "Dragging an account to another place" diagram from the
  highlight loop to the save, which this slice implements
- [Grid drag wireframe](../../roadmap/10-rearranging-accounts/wireframes/grid-drag.md)
  — the whole item's grid; this slice draws the tint in "Dragging" and all of
  "After the drop"
- [`docs/research/gtk4-drag-and-accordion.md`](../../research/gtk4-drag-and-accordion.md)
  — lines 14–68
- [`docs/requirements.md`](../../requirements.md) — `FR.14.2`, `FR.14.3`,
  `FR.14.5`, `FR.14.6`, `FR.14.7`, `FR.14.8`
- [`docs/design.md`](../../design.md) — rules 1, 4, 6, and the drop-highlight
  rule this slice's pattern owes
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rule 5
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
