# 05 — Keeping the controls over a game visible on Windows

**Roadmap:** [12](../../roadmap/12-windows-support/README.md) · **Scope:** front-end · **Depends on:** 02

## Context

On Linux the application draws four small things on top of a game's place:
- a drag handle in the top-right corner, used to move the account to another
  place;
- a short-lived percentage after a zoom step;
- a cover with the account's name, shown until the page first paints;
- a highlight showing where a dragged account will land.

On Windows the game page is a separate native window laid over the GTK window.
It always draws above anything GTK draws in the same area, so all four would be
hidden.

This slice moves each of them somewhere the page does not cover, without
changing what the user can do:
- **The drag handle** sits in a thin strip just above each live game, at the
  strip's right end.
- **The zoom percentage** appears in a small popup of its own, low and centred
  over the game, with the same look and timing as on Linux. A popup is its own
  native window, so it draws above the page. It takes no focus and ignores
  clicks.
- **The name cover** shows while the game is kept at zero size, until the page
  first paints. The game is only shrunk, never hidden, so it starts running
  from its first frame.
- **During a drag**, every live game is shrunk to nothing, so the landing
  highlight and the lines between places show. The games reappear in their new
  places when the drag ends. They keep running the whole time.

A place with no live game, whether parked, waiting in the start queue or
stopped, shows the same plain panel as on Linux and needs no strip. Linux
itself does not change at all. This is a slice of its own because it touches
only the grid of places and the invisible widget that sizes each game. That lets
it run alongside the slices working on the engine and on memory figures.

## User experience

- **Flow** — Point at a place → the drag grip shows in a thin strip above that
  game, at its right end; dragging it rearranges accounts as on Linux (item 10).
- **Flow** — While dragging, every live game view is shrunk to nothing, so the
  drop highlight and slot lines show; on drop or cancel the views return. The
  games keep running.
- **Flow** — A zoom step flashes the percentage low and centred over that place,
  in a small popup that takes no focus and fades as on Linux.
- **States** — Loading: until a page first paints, its place shows the cover
  with the account's name (the view is at zero size, not hidden). Parked,
  queued, stopped and empty places are unchanged and have no strip.
- **Pattern** — The zoom acknowledgement keeps its shape, place, timing and
  single-figure behaviour (design rule 10); the absent-game panel is unchanged
  (design rule 4).
- **New pattern** — the per-place grip strip on Windows. Nothing in
  `docs/design.md` covers a control that cannot overlap live content, so the
  design doc owes a rule once this ships.

## Technical details

- **Design** — in `session_grid/imp.rs` `register_slot`, under `cfg(windows)`,
  the slot becomes a vertical `gtk::Box`: a `grip-strip` box on top, holding
  `build_grip`'s image at its end with the grip's margins moved to the strip's
  padding, then the existing overlay below it. The strip keeps the grip's
  current visibility rule, and is hidden when the place has no live view
  (design rule 4). Its CSS goes in `session-grid.css` under a `.grip-strip`
  class using the window background.
- **Design** — the readout: under `cfg(windows)`, `build_readout`'s label is the
  child of a `gtk::Popover` with `autohide(false)`, `has_arrow(false)`,
  `can_focus(false)` and `can_target(false)`, parented to the slot's overlay.
  It points at a 1×1 `gdk::Rectangle` at the overlay's bottom centre, sitting
  as low as the Linux label does. Showing uses `popup()`, the fade ends in
  `popdown()`, and `ZOOM_READOUT_FADE_MILLIS` with the rearm-not-queue
  behaviour is shared code (design rule 10).
- **Architecture** — `attach_view`, under `cfg(windows)`, downcasts
  `EngineView::widget()` to `EngineHost`, calls `collapse()`, and calls
  `restore()` from `connect_painted`, while the cover hides as today.
- **Architecture** — the item 10 drag source: on `drag-begin` the grid calls
  `collapse()` on every live `EngineHost`. On `drag-end` (dropped or
  cancelled) it calls `restore()` after the swap or fill transition has
  re-seated the slots, so each host restores from its new allocation. No view
  is set invisible (architecture rule 8: the grid redraws from the transition's
  result).
- **Architecture** — this slice edits only `session_grid/imp.rs`,
  `session_grid.rs`, `session-grid.css` and `web_engine/webview2/host.rs`
  (`restore()` re-reads the current allocation). It does not touch
  `web_engine/webview2.rs`, `ffi.rs` or `window/imp.rs`, so it can run in
  parallel with the scripts-and-zoom slice and the memory-probe slice.
- **Code standards** — Linux keeps its overlay code path. `cfg` branches are
  confined to small helpers (`fn mount_grip`, `fn mount_readout`), never
  scattered through `register_slot` (rule 6). Each helper's `///` names the
  airspace constraint (rule 18).

## Acceptance criteria

- [ ] `(integration)` `make windows-check` and `make verify` pass
- [ ] `(manual)` on Linux, the grip, the zoom readout, the loading cover and the
      drop highlight look and behave exactly as before
- [ ] `(manual)` in the Windows VM, hovering a live game shows the grip in a strip
      above it at the right end, and dragging it to another place swaps the two
      accounts
- [ ] `(manual)` in the Windows VM, during that drag every live game disappears and the
      drop highlight and slot lines show; the games reappear in their new
      places on drop and in their old places on `Esc`
- [ ] `(manual)` in the Windows VM, a game's in-page clock keeps advancing across a
      10-second drag
- [ ] `(manual)` in the Windows VM, `Ctrl`+`+` shows the percentage low and centred
      over that game, it stays above the page, keyboard focus stays in the
      game, and it fades after the same delay as on Linux
- [ ] `(manual)` in the Windows VM, starting a parked account shows its name cover
      until the page paints, then the game
- [ ] `(manual)` in the Windows VM, a parked, queued or stopped place shows the plain
      panel with no strip, and an empty place shows nothing new

## References

- [Roadmap item](../../roadmap/12-windows-support/README.md) — Front-end
  "Overlap on Windows" and the "Rearranging a place on Windows" diagram
- [Wireframes](../../roadmap/12-windows-support/wireframes/) —
  `windows-game-place.md`
- [`docs/requirements.md`](../../requirements.md) — Platform Support `FR.1.2`,
  `FR.1.8`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12
- [`docs/code-standards.md`](../../code-standards.md) — rules 6, 18
- [`docs/design.md`](../../design.md) — rules 4, 10

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The `(manual)` steps run in the
Windows VM ([`docs/windows-vm.md`](../../windows-vm.md)).
