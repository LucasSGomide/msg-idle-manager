# 03 — Clicking a row to swap or focus

**Roadmap:** [02](../../roadmap/02-session-sidebar/README.md) · **Scope:** front-end · **Depends on:** 01, 02

## Context

The previous two slices built the pieces this one connects. One added a rule to
the pure logic layer that answers "what happens when the user picks an account
from the list" — trade it into the focused slot, or just move the focus if it is
already on screen. The other added the list itself down the side of the window,
showing every account and where it stands, but with rows that do nothing when
clicked. This slice makes a click do the obvious thing.

When the user activates a row, the widget does not decide what the click means.
It sends the account's identifier to the logic layer, gets back a complete
description of where every account now stands, and hands that same description to
both the grid and the list. The grid moves at most two web views — the account
coming into view and the one being displaced — and leaves every other view
exactly where it was, so no page reloads. The list redraws the two rows whose
state changed. Because both surfaces redraw from one answer rather than each
working out the change for itself, they cannot end up disagreeing about where an
account is.

Two rows change their trailing marker in a swap: the account that came into view
now shows a slot number, and the account that was pushed out now shows the
out-of-sight marker in the dimmed style. Clicking a row whose account is already
in a slot moves nothing — it just makes that slot the focused one, so the
current-row marker moves to it.

This is a separate slice from the list itself because it is the first place the
new two-surface arrangement is exercised end to end, and it is worth landing on
its own so that a failure here — a stale row, a reloaded page, a displaced
account with no marker — is isolated to this change.

## User experience

- **Flow** — click a row whose account is out of sight, and it swaps into the
  focused slot; the account that was there takes its turn out of sight. The
  clicked row updates to show its new slot, the displaced row updates to show it
  is out of sight.
- **Flow** — click a row whose account is already in a slot, and that slot
  becomes the focused one. Nothing moves.
- **States** — **In a slot**: after a swap the clicked row's trailing edge names
  its new slot and it becomes the current row. **Out of sight**: the displaced
  row's trailing edge switches to the out-of-sight marker and its name to the
  dimmed style.

## Technical details

- **Architecture** — rule 8: row activation emits an intent; the handler sends
  the account's identifier to the core, receives the `Placement`, and passes it
  to both the grid and the sidebar store — the widget decides nothing.
- **Front-end** — wire the `ListView`/`SingleSelection` activation signal in
  `session_sidebar/imp.rs` to a callback the window installs, carrying the
  activated account's identifier.
- **Front-end** — in `window/imp.rs`, route that identifier through the
  `SessionBook` intent from task 01, then feed the returned placement to the
  grid's existing re-place path and to the sidebar's store refresh.
- **Front-end** — the grid moves only the views whose visibility changed; a moved
  view is re-allocated between a slot rectangle and an out-of-bounds rectangle,
  never re-parented, so its page is not reloaded (item 01's constraint).
- **Front-end** — the sidebar refresh updates every row's state property from the
  new placement and the new focused slot, so the current-row marker and the
  dimmed style follow the swap.
- **Architecture** — rule 10: the whole exchange runs on the GTK main context; no
  widget is created or destroyed.
- **Testing** — architecture rule 14 and code standards rule 25: the swap is only
  observable with a display server, so it is covered by this item's
  `test-script.md`, not by `cargo test`.

## Acceptance criteria

- [x] `(manual)` clicking an out-of-sight account's row with the focused slot
      occupied swaps them: the clicked row shows the `Current` marker and its
      name goes bold, the displaced row shows the `Background` marker and the
      dimmed style
- [x] `(manual)` in that swap both web views keep their pages — neither reloads
      (`GTK_DEBUG=interactive` shows the same widget objects and unchanged page
      content)
- [x] `(manual)` every account other than the two involved keeps its row marker
      and its grid position
- [x] `(unit)` bringing an out-of-sight account into an empty focused slot moves
      it in and displaces nothing — backed by the core rule's unit test, since
      the shell cannot reach that state: an account is out of sight only when
      every slot is filled, so the focused slot is never empty while a row
      exists to click (the `Filled` outcome is live only for the pure function)
- [x] `(manual)` clicking a row whose account already holds a slot moves the
      current-row marker to that row and moves no view

## References

- [Roadmap item](../../roadmap/02-session-sidebar/README.md) — the full picture, including the "Bringing an out-of-sight account into view" diagram
- [Wireframes](../../roadmap/02-session-sidebar/wireframes/session-sidebar.md) — the row layout and its state markers
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 14
- [`docs/code-standards.md`](../../code-standards.md) — rule 25
- [`docs/naming.md`](../../naming.md) — rules 4, 6

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
