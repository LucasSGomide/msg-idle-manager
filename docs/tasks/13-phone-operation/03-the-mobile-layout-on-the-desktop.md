# 03 — The Mobile layout on the desktop

**Roadmap:** [13](../../roadmap/13-phone-operation/README.md) · **Scope:** full-stack · **Depends on:** 01

## Context

This application shows browser idle games in a window that can be split one,
two or four ways. A later slice streams one game to a phone, and for that
picture to make sense the game has to be drawn at a phone's shape, so the game
itself lays out for a phone. This slice adds that shape as a fourth
arrangement, called mobile mode, switched on and off from the desktop.

With mobile mode on, the window shows exactly one game in a slot the size of
a phone screen, 412 by 915 pixels by default, centred and outlined on the
window background. Every other account goes out of sight, running as before.
The slot is never scaled to fit: a window shorter than the slot simply clips
its bottom, because the page's size is the whole point. Zoom is locked at 100%
in this arrangement, since changing it would change the page's size out from
under the game.

Mobile mode is deliberately a layout and nothing more. It joins the `1` `2`
`4` toggles in the header bar as a fourth toggle labelled `Phone`. But it is
never saved: the program always starts in the arrangement it had before, and
turning mobile mode off puts that arrangement back exactly, including which
account sat where and which was focused, even after accounts were switched
around while the mode was on. Accounts deleted meanwhile are skipped and
accounts added meanwhile keep the place they got.

The slice ships before any phone exists so the desktop half can be tried on
its own: press `Phone`, see the frame, press `2`, see the old arrangement
return. The phone later drives the same switch through the same code.

## User experience

- **Entry** — A fourth toggle labelled `Phone` joins the linked `1` `2` `4`
  layout toggles in the header bar and switches mobile mode on and off.
- **Flow** — Turn mobile mode off from the desktop → the desktop restores the
  arrangement it had before mobile mode was entered.
- **States** — Desktop, `Mobile` layout: the grid shows one phone-shaped slot
  centred on the window background, with the slot's outline drawn around it;
  when the window is shorter than the slot, the slot is clipped at the bottom
  rather than scaled, so the page's viewport stays the phone's.
- **Pattern** — No zoom readout is flashed in `Mobile`; zoom is locked there
  (design rule 10 by contrast).
- **New pattern** — a fixed-size, phone-shaped slot centred in the grid, with
  its outline drawn and its overflow clipped rather than scaled. Nothing in
  `docs/design.md` covers a slot that does not fill its share of the window;
  the design doc owes a rule once this ships.

## Technical details

- **Architecture** — `Layout` gains `Mobile` with `slot_count() == 1`;
  `WorkspaceBook` gains `enter_mobile_mode(viewport)`, `leave_mobile_mode()`,
  `is_mobile_mode()`, `mobile_viewport()` and `set_mobile_viewport(viewport)`
  holding `MobileMode { viewport, snapshots: HashMap<WorkspaceId, Arrangement> }`
  where `Arrangement` is a workspace's layout, focused slot and every
  account's remembered slot; entering snapshots the active workspace and
  applies `set_layout(Mobile)`, a workspace shown for the first time while the
  mode is on is snapshotted then switched, leaving restores every snapshot
  (deleted accounts skipped, added accounts kept where they are) then clears
  the mode (rule 8; code standards rule 1).
- **Architecture** — `WorkspaceBook::saved()` substitutes each snapshot's
  layout, focus and slots for the live ones while the mode is on, so
  `sessions.toml` never records `Mobile` and liveness changes are still
  written; `Session::zoom_for(Layout::Mobile)` returns `ZoomLevel` 1.0 and
  `zoom_in`/`zoom_out`/`reset_zoom` return `None` in that layout, so
  `RememberedZoom` never gains a `mobile` key.
- **Architecture** — `idle-manager-store` `session_file.rs`: `LayoutRecord`
  gains no variant; `From<Layout> for LayoutRecord` maps `Mobile` to `Single`
  behind a comment stating `saved()` never emits it (rule 7; code standards
  rule 18).
- **Architecture** — `session_grid/imp.rs`: `grid_dimensions(Mobile)` is
  `(1, 1)`; `allocate_slots` gives slot 0 a rectangle of the book's viewport
  in logical pixels, centred horizontally and top-aligned, clipped by the
  existing `set_overflow(Hidden)`, and off-grid entries are allocated outside
  as today; `draw_slot_lines` draws that rectangle's
  outline in `Mobile`; the `Single` no-grip rule in `sync` and
  `sync_grip_strip` extends to `Mobile`; `SessionGrid::sync` reads the
  viewport from the book (rule 12; design rule 14 on Windows).
- **Architecture** — `window.ui` adds `layout_mobile`, a `gtk::ToggleButton`
  labelled `Phone` in the linked `layout_toggles` group after `4`;
  `window/imp.rs` `connect_layout_toggle` maps it to
  `enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT)` (a later slice passes the
  phone's own size), and activating `1`, `2` or `4` while the mode is on calls
  `leave_mobile_mode()` then `set_layout` only if the chosen layout differs
  from the restored one; `select_layout_toggle` maps `Mobile` to the new
  toggle under the existing re-entrancy guard; both paths call
  `snap_zoom_for_active()`, `redraw()` and `request_save()`.
- **Architecture** — `handle_shortcut_key`'s zoom branch and
  `apply_zoom_step` return early with no readout while the active layout is
  `Mobile`; `Ctrl`+wheel over the slot is likewise ignored.
- **Code standards** — the core changes are unit-tested with no display
  (rule 25), test names state behaviours (rule 21), one assertion subject
  each (rule 23).

## Acceptance criteria

- [x] `(unit)` `enter_mobile_mode` on a `Grid` workspace with four seated
      accounts leaves the focused account in slot 0 and the other three
      off-grid, and `leave_mobile_mode` returns all four to their previous
      slots with the previous focus
- [x] `(unit)` after entering, focusing another account (swapping slot 0),
      then leaving, the arrangement equals the pre-mobile one exactly
- [x] `(unit)` an account removed while the mode is on is skipped on leave,
      and an account added while it is on keeps its current placement
- [x] `(unit)` `saved()` while the mode is on reports the pre-mobile layout
      and slots, and a park performed meanwhile is reflected in the saved
      liveness
- [x] `(unit)` `zoom_for(Layout::Mobile)` is 1.0 for an account with a stored
      `Single` override, and `zoom_in` in `Mobile` returns `None` and writes
      no `mobile` key
- [x] `(unit)` the store maps `Layout::Mobile` to the `single` record spelling
      and the snapshot of a saved file written during mobile mode is
      unchanged from the pre-mobile one
- [x] `(integration)` `make verify` passes
- [x] `(manual)` pressing `Phone` shows one 412 × 915 outlined slot with the
      focused game inside and no grip, and with the window at its default
      800 px height the slot is clipped at the bottom, not scaled
- [x] `(manual)` pressing `2` restores the two-slot arrangement with the same
      accounts in the same slots; quitting in mobile mode and relaunching
      opens in the layout the workspace had before
- [x] `(manual)` `Ctrl`+`+` and `Ctrl`+wheel over the `Mobile` slot change
      nothing and show no readout

## References

- [Roadmap item](../../roadmap/13-phone-operation/README.md) — Back-end
  "Mobile mode in the domain"; Front-end "The `Mobile` layout in the grid"
  and "The header bar"; the "Entering and leaving mobile mode on the desktop"
  diagram
- [`openapi.json`](../../roadmap/13-phone-operation/openapi.json) —
  `Viewport`
- [Sequence diagrams](../../roadmap/13-phone-operation/sequence-diagrams.md)
  — the whole item
- [Wireframes](../../roadmap/13-phone-operation/wireframes/) —
  `header-bar-and-mobile-layout.md`
- [`docs/requirements.md`](../../requirements.md) — Remote Access `FR.3.1`,
  `FR.3.2`, `FR.3.4`, `FR.3.5`; Session Management `FR.3.2`, `FR.12.2`
- [`docs/architecture.md`](../../architecture.md) — rules 7, 8, 12
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 18, 21, 23, 25
- [`docs/design.md`](../../design.md) — rules 10, 14

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
