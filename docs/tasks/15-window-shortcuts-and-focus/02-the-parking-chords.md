# 02 — The parking chords

**Roadmap:** [15](../../roadmap/15-window-shortcuts-and-focus/README.md) · **Scope:** front-end · **Depends on:** 01

## Context

Parking an account stops its game and hands its memory back; starting it puts
it back on screen. Today both are one menu item on the account's own row —
one control whose label flips with the account's state — and, for a whole
workspace at once, a pair of items on its heading menu. All of them need the
mouse.

This slice adds the four chords that do the same from the keyboard:
`Ctrl`+`P` parks the focused account, `Ctrl`+`S` starts it, and with `Shift`
held the same two letters act on every account in the shown workspace.

Two decisions shape the work. The first: each chord means one direction and
only that direction. `Ctrl`+`P` always means parked, `Ctrl`+`S` always means
running, and a chord pressed where it does not apply — park on an
already-parked account, start on a live one — is swallowed and does nothing
at all. A single flip key would be fewer keys, but nothing would say which way
the next press is about to go, and an account's state moves on its own while
you are deciding: a queued account becomes starting, then live, with no press
from anyone. The row's coloured dot already says which of the two keys is live
before either is pressed. This departs from the shape design rule 2 asks for,
and that departure is recorded in the roadmap item and written into
`docs/design.md` by task 05 — not left implicit here.

The second: the chords go through the handlers the menu items already use.
Nothing in this slice decides whether an account can be parked, or which
accounts a workspace-wide park touches. The domain answers both, exactly as
it does for a click.

## User experience

- **Entry** — `Ctrl`+`P`, `Ctrl`+`S`, `Ctrl`+`Shift`+`P`, `Ctrl`+`Shift`+`S`,
  from anywhere in the window.
- **Flow** — Park the focused account: press `Ctrl`+`P` → the account in the
  outlined slot stops, its slot shows the plain stopped panel with its `Start`
  button, its sidebar dot goes grey. `Ctrl`+`S` starts it again.
- **Flow** — Park the shown workspace: press `Ctrl`+`Shift`+`P` → every
  running, queued or starting account in the shown workspace is parked, no
  confirmation, exactly as the heading menu's `Park all`. `Ctrl`+`Shift`+`S`
  queues every parked one and they come back one at a time, in order.
- **States** — A chord that would touch nothing is consumed and silent: no
  message strip, no sound, no flash (`FR.26.6`). That covers `Ctrl`+`P` on a
  parked account, `Ctrl`+`S` on a live or starting one, and either workspace
  chord on a workspace with nothing to act on — including an empty one.
- **States** — Sidebar in selection mode: all four consumed and inert.
- **States** — Held down: each runs exactly once, on the latch task 01
  generalised.
- **Pattern** — Two chords, each idempotent, rather than one flip key: the
  narrowing of design rule 2 the roadmap item argues and task 05 writes down.
- **Pattern** — The menu items name their key in accelerator text rather than
  a tooltip, because a menu item has nowhere to hover (`FR.25.4`, rule 19).

## Technical details

- **Architecture** — `window/shortcut.rs`: `Shortcut` gains `ParkFocused`,
  `StartFocused`, `ParkWorkspace` and `StartWorkspace`. `shortcut_for` gains
  `ctrl` with `p`/`P` → `ParkWorkspace` when `shift`, `ParkFocused` otherwise,
  and the same shape for `s`/`S`. Both letter cases are matched in both arms
  and the direction is taken from the `SHIFT_MASK` bit, never from the keyval's
  case: GDK delivers the shifted keyval `P` under `Shift` while Win32 reports
  the unshifted virtual key, so the modifier bit is the only discriminator
  that serves both engines (task 03).
- **Architecture** — `window/imp.rs` `run_shortcut` gains four arms, each
  returning while `self.sidebar.is_selecting()`. `ParkFocused` and
  `StartFocused` read the focused account's id and liveness from
  `book.active().focused_session()` and call the row menu's own
  `toggle_parking(&id)` only when the liveness matches the direction asked for
  — `Liveness::Live` for park, `Liveness::Parked` for start. `Starting` and
  `Queued` fall through to silence, matching the greyed row item (design rule
  2). `ParkWorkspace` and `StartWorkspace` call `self.park_all` /
  `self.start_all` for `book.active_id()`; both already return early on an
  empty result, so the silent no-op costs nothing (rule 8 — the shell names a
  transition, the domain decides it).
- **Code standards** — the liveness read and the borrow of `book` are bound to
  a `let` and dropped before `toggle_parking` borrows the book again, the same
  care `create_account` and `move_ticked` already take (rule 12).
- **Design** — `session_sidebar/imp.rs`: `bind_heading_menu` sets the `accel`
  attribute on the `Park all` and `Start all` `gio::MenuItem`s
  (`<Control><Shift>p`, `<Control><Shift>s`), and `bind_row_menu` sets it on
  the single Park/Start item, choosing `<Control>p` or `<Control>s` from the
  direction `data.action_label()` already computes, so the menu names the key
  that actually applies rather than both (`FR.25.4`, rule 19). Both menus are
  rebuilt on every bind already, so the accelerator follows the state with no
  extra wiring.
- **Design** — `resources/ui/help-overlay.ui`'s `Window` group gains
  `<Control>p` "Park the focused game", `<Control>s` "Start the focused game",
  `<Control><Shift>p` "Park every game in the workspace" and
  `<Control><Shift>s` "Start every parked game in the workspace" (`FR.25.3`).

## Acceptance criteria

- [x] `(unit)` `shortcut_for` maps `Ctrl`+`p` and `Ctrl`+`P` to `ParkFocused`,
      `Ctrl`+`s` and `Ctrl`+`S` to `StartFocused`
- [x] `(unit)` `shortcut_for` maps `Ctrl`+`Shift` with `p`, `P`, `s` and `S`
      to `ParkWorkspace` and `StartWorkspace`, never to the unshifted pair
- [x] `(unit)` `Ctrl`+`p` with `LOCK_MASK` and `MOD2_MASK` added still maps to
      `ParkFocused`, and with `SHIFT_MASK` added maps to `ParkWorkspace`
- [x] `(unit)` `repeats_while_held` is false for all four new variants
- [x] `(integration)` `make verify` passes
- [x] `(manual)` `Ctrl`+`P` with a live focused account parks that account and
      only that account; a second `Ctrl`+`P` leaves it parked and starts
      nothing, with no message strip
- [x] `(manual)` `Ctrl`+`S` on a focused account that is starting or queued
      changes nothing and builds no second view
- [x] `(manual)` `Ctrl`+`Shift`+`P` parks every non-parked account of the
      shown workspace and leaves another workspace's alone;
      `Ctrl`+`Shift`+`S` brings the parked ones back one at a time, in order
- [x] `(manual)` `Ctrl`+`Shift`+`P` on a workspace with nothing running, and
      `Ctrl`+`Shift`+`S` on one with nothing parked, do nothing and show
      nothing
- [x] `(manual)` all four are inert while the sidebar is selecting, and a held
      `Ctrl`+`Shift`+`P` parks the workspace exactly once
- [ ] `(manual)` with a game page holding the keyboard, `Ctrl`+`S` saves no
      page and opens no save dialog, and `Ctrl`+`P` opens no print dialog —
      not run: the probe page used for the runbook binds neither key, and a
      page that does is what this step is about
- [x] `(manual)` a row's ⋯ menu shows `Ctrl+P` beside `Park` on a live account
      and `Ctrl+S` beside `Start` on a parked one; a heading's shows
      `Ctrl+Shift+P` / `Ctrl+Shift+S`
- [x] `(manual)` `Ctrl`+`?` lists all eight of this item's chords under the
      `Window` group

## References

- [Roadmap item](../../roadmap/15-window-shortcuts-and-focus/README.md) —
  "Design — the two-chord departure from rule 2"; Front-end "The table",
  "The acts", "Discoverability"
- [`docs/requirements.md`](../../requirements.md) — `FR.26.3`, `FR.26.4`,
  `FR.26.5`, `FR.26.6`, `FR.25.3`, `FR.25.4`; `FR.24.1`, `FR.24.2` for what
  the two workspace chords reuse
- [`docs/architecture.md`](../../architecture.md) — rule 8
- [`docs/code-standards.md`](../../code-standards.md) — rules 12, 25
- [`docs/design.md`](../../design.md) — rules 2, 19; rule 20 is written by
  task 05
- [`docs/naming.md`](../../naming.md) — rule 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
