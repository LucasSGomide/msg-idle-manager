# 04 — The grip over each place

**Roadmap:** [10](../../roadmap/10-rearranging-accounts/README.md) · **Scope:** front-end · **Depends on:** —

## Context

The application's window is split into one, two or four places, and each place
can be filled by a running web game. A person clicks anywhere in a place to make
it the active one, which the keyboard shortcuts and the zoom wheel then act on.
This slice gives every occupied place a handle to pick its account up by. Where
the account can be dropped comes in a later slice.

Moving the pointer into a place reveals a small drag-handle icon in its top-right
corner. It appears whether the pointer is over the game's page or over the plain
panel a paused account shows, and it disappears when the pointer leaves. Pressing
the handle and moving past a small threshold starts a drag. A little rounded tag
with the account's name follows the pointer, and the handle hides while the drag
is under way. A press and release without moving does nothing at all. When the
window shows only one place there is nowhere to move an account, so no handle is
drawn. With no accounts on screen there are no places to hover.

Two things make this harder than it looks. The handle sits on top of a live web
page, so the handle itself must be the only thing there that takes clicks.
Anything around it that took them would stop every click in that corner reaching
the game. Also, a press on the handle must not make its place active. The grid
notices clicks before the game's page does, because the page would otherwise
swallow them, so it cannot simply let the handle go first. Instead, when a press
arrives it checks what is under the pointer and focuses nothing if that is a
handle.

The drag carries a private kind of data only this application understands, never
plain text. A web page is built to accept dropped text, and a drag carrying a
name could be pasted into a game's text box. Until the next slice adds a drop
target, letting go ends the drag and nothing moves. That is still enough to check
that no game receives anything.

## User experience

- **Entry** — a drag grip in the top-right corner of any occupied place, inset by
  a small margin, shown only while the pointer is inside that place, and never in
  the one-place arrangement.
- **Flow** — move the pointer into a place, over its game's page or its parked
  panel. The grip appears in its top-right corner and disappears when the pointer
  leaves. Only the icon takes clicks; the rest of that corner still reaches the
  game.
- **Flow** — press the grip. Focus does not move: unlike a click anywhere else in
  the place, a press on the grip does not make it active. A press and release
  with no movement does nothing at all.
- **Flow** — drag past the threshold. A small rounded chip carrying the account's
  name follows the pointer, and the grip hides once the drag begins. Letting go
  ends the drag with nothing moved; the drop itself is a later slice.
- **States** — **parked or queued place**: the grip sits over rule 4's plain
  panel the same way it sits over a page, and the panel is not restyled.
  **Drag over the web page of a game**: the page receives nothing.
  **One-place arrangement**: no grip anywhere. **Empty grid**: no places, so no
  grip.
- **New pattern** — a hover-revealed drag grip in a place's corner. Nothing in
  `docs/design.md` covers a control that appears over live content on hover:
  rule 10's readout is transient on a gesture and takes no input. The design doc
  owes a rule once this ships.

## Technical details

- **Front-end** — each `SlotEntry` gains a grip: a `gtk::Image` showing
  `list-drag-handle-symbolic`, added as another overlay child of the place's
  `gtk::Overlay`, aligned top-end with a small margin, carrying a `slot-grip` CSS
  class and styled from `session-grid.css`. It is the overlay child itself, not
  wrapped. If a wrapper is ever needed it must set `can-target` to false, or it
  takes every click in that corner away from the game (`FR.14.4`).
- **Front-end** — an `EventControllerMotion` on the overlay shows the grip on
  `enter` and hides it on `leave` (`FR.14.1`). `SessionGrid::sync` also hides it
  for an off-grid entry and in the `Single` layout. A hidden widget is never
  picked, so a hidden grip cannot take a press.
- **Front-end** — the grip carries two controllers grouped together, per
  `docs/research/gtk4-drag-and-accordion.md:35`. A `GestureClick` claims the
  sequence in `pressed`, so no ancestor gesture can also act on it. A
  `DragSource` with `DragAction::MOVE` provides the payload. `DragSource` never
  claims, so the grouping is what keeps the drag alive, and a drag starts only
  past the threshold.
- **Front-end** — the payload is a small `DraggedAccount` wrapping the
  `SessionId`, registered as a `glib::Boxed` type and offered through
  `gdk::ContentProvider::for_value`, never a string (`FR.14.5`). The grid module
  exposes it so the drop slice can accept exactly this type.
- **Front-end** — in `drag-begin` the source sets a `gtk::DragIcon` child: a
  label holding the account's `display_name`, styled as a small chip in
  `session-grid.css`. Not a `WidgetPaintable` of the place: a quarter-window
  paintable would hide the places being dropped on.
- **Front-end** — the grid's click-to-focus gesture stays in the capture phase
  at `crates/idle-manager-shell/src/session_grid/imp.rs:120`, because a web page
  would otherwise swallow the click, so the grip's claim cannot stop it.
  Instead, `focus_slot_at` at
  `crates/idle-manager-shell/src/session_grid/imp.rs:434` first calls
  `pick(x, y, PickFlags::DEFAULT)` on the grid and returns without focusing when
  the picked widget has the `slot-grip` class (`FR.14.3`). Moving the gesture to
  the bubble phase, as `FR.14.3`'s last sentence says, would break click-to-focus
  over every page; the pick check was measured on X11 and Wayland.
- **Design** — rules 4, 6 and 10. The grip is an overlay, costs no layout and
  leaves the sidebar width untouched. The hover-revealed grip is a new pattern.
  `docs/design.md` owes a rule for it (where it sits, when it shows, that only
  the icon takes input), written once this slice ships.
- **Testing** — `(manual)`, against the headless harness, in this task's section
  of `test-script.md` (architecture rule 14), in a page with a click target and a
  text box in its top-right corner. If this is the item's first accepted slice, it
  also writes `## Setup` and `## Teardown`.

## Acceptance criteria

- [ ] `(manual)` in the four-place arrangement, moving the pointer over an
      occupied place's page shows the grip in its top-right corner, and moving it
      out of the place hides the grip
- [ ] `(manual)` hovering a parked account's place shows the grip over its panel,
      and the panel keeps its three elements unrestyled
- [ ] `(manual)` in the one-place arrangement no grip appears wherever the
      pointer goes, and with an empty grid none appears either
- [ ] `(manual)` a click in the place's top-right corner just beside the grip,
      not on it, reaches the game's page
- [ ] `(manual)` pressing the grip of an unfocused place leaves the focus outline
      on the place that had it, while a press on that place's page focuses it
- [ ] `(manual)` a press and release on the grip with no movement starts no
      drag, and focus and the arrangement are unchanged
- [ ] `(manual)` dragging the grip past the threshold shows a chip with the
      account's name following the pointer, and the grip hides while dragging
- [ ] `(manual)` releasing the drag over a text box inside another game's page
      leaves the text box empty and the arrangement unchanged
- [ ] `(unit)` a `DraggedAccount` stored in a `glib::Value` reads back carrying
      the same `SessionId`, and the value's type is not a string type

## References

- [Roadmap item](../../roadmap/10-rearranging-accounts/README.md) — the full
  picture, including the "Dragging an account to another place" diagram whose
  steps up to the name tag this slice implements
- [Grid drag wireframe](../../roadmap/10-rearranging-accounts/wireframes/grid-drag.md)
  — the whole item's grid; this slice draws the "At rest" and "Hover" states and
  the chip, and leaves the tint and "After the drop" to task 05
- [`docs/research/gtk4-drag-and-accordion.md`](../../research/gtk4-drag-and-accordion.md)
  — lines 14–68, the grip recipe
- [`docs/requirements.md`](../../requirements.md) — `FR.14.1`, `FR.14.3`,
  `FR.14.4`, `FR.14.5`
- [`docs/design.md`](../../design.md) — rules 4, 6, 10, and the grip rule this
  slice's pattern owes
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13, 14
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
