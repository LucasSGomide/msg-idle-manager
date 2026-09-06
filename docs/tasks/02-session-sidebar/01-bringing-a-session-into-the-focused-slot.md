# 01 — Bringing a session into the focused slot

**Roadmap:** [02](../../roadmap/02-session-sidebar/README.md) · **Scope:** back-end · **Depends on:** —

## Context

The application keeps a list of every game account it is running. Some of those
accounts have a place on screen — a numbered slot in the window's grid — and the
rest are running out of sight, fully alive but with nowhere to be drawn. A later
slice adds a list down the side of the window where a user can click any account
to bring it into view. This slice builds the rule that decides what "bring it
into view" means, and it lives entirely in the pure logic layer of the program,
the part that has no knowledge of windows or widgets.

The rule has to handle three situations. The account the user clicked is out of
sight and the slot they are currently focused on already holds another account:
the two trade places, so the clicked account takes the slot and the account that
was there goes out of sight. The clicked account is out of sight and the focused
slot is empty: the clicked account simply moves in and nothing is pushed out.
The clicked account is already in a slot: nothing moves at all, and instead the
focus shifts to the slot that account occupies, so a click on a visible row just
changes which slot the next action targets.

Two separate parts of the screen will draw from this rule's answer — the side
list and the grid — so the answer is deliberately complete rather than a
description of the change. It reports where every account stands after the click,
not just the one or two that moved. If it returned only the difference, the two
parts of the screen would each apply that difference to their own idea of the
current state and could slowly drift apart. It also reports, as a distinct
value, which of the three situations happened, so the caller does not have to
work that out from the placement.

This is its own slice because getting this rule right, with its three cases each
pinned by a test, is the foundation the side list is built on, and a bug here
would show up as two parts of the screen disagreeing about where an account is.

## Technical details

- **Architecture** — rules 1 and 9: pure domain code in `idle-manager-core`,
  synchronous, no GTK, no I/O, no clock, no randomness.
- **Architecture** — rule 8: the intent enters the layout model, the transition
  runs, the caller renders from the returned state; the model never calls back
  into a widget.
- **Architecture** — rule 6: no new port — the transition reads and writes only
  `SessionBook` state the core already owns.
- **Back-end** — extend `layout.rs` with the swap: the placement pass from item
  01 (the `arrange` function that already answers where every session goes) gains
  one more intent — bring a named session into the focused slot — returning a
  total placement: the `Visibility` of every session, not a delta, plus a value
  naming which of the three outcomes occurred (a swap, an empty-slot fill, or
  focus-only).
- **Back-end** — the returned type is named for its result — `Placement` — not
  for the calculation behind it (naming rule 11); the outcome is an enum, never
  a boolean beside an optional displaced identifier (code standards rule 1).
- **Back-end** — focus already lives in the core's layout model from item 01; add
  the intent that moves it to a given slot, so activating a row whose account
  already holds a slot is a focus change routed through the model, not a special
  case a widget handles.
- **Back-end** — a session already holding a slot: focus moves to that slot,
  `remembered` and every `Visibility` unchanged.
- **Testing** — unit tests in `#[cfg(test)] mod tests` at the foot of the file,
  one behaviour per test, named for the behaviour (code standards rules 21–24).

## Acceptance criteria

- [x] `(unit)` bringing an out-of-sight session into an occupied focused slot
      swaps the two: the named session takes the slot, the previous occupant
      becomes off-grid
- [x] `(unit)` in that swap, every session other than those two keeps the exact
      visibility it had
- [x] `(unit)` bringing an out-of-sight session into an empty focused slot places
      it there and moves no other session
- [x] `(unit)` activating a session that already holds a slot moves focus to that
      slot and returns a placement identical to the one before the call
- [x] `(unit)` the returned `Placement` carries the visibility of every session in
      the book and an outcome enum distinguishing the swap, the empty-slot fill
      and the focus-only case

## References

- [Roadmap item](../../roadmap/02-session-sidebar/README.md) — the full picture, including the "Bringing an out-of-sight account into view" diagram
- [`docs/architecture.md`](../../architecture.md) — rules 1, 6, 8, 9
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 21–24
- [`docs/naming.md`](../../naming.md) — rule 11

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
