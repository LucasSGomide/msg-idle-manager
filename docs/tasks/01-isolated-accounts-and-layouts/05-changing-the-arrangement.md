# 05 — Changing the arrangement

**Roadmap:** [01](../../roadmap/01-isolated-accounts-and-layouts/README.md) · **Scope:** front-end · **Depends on:** 04

## Context

By now the window can hold accounts and draw them in places. This slice makes
the three buttons in the title bar work: one account filling the window, two
side by side, or four in a two-by-two grid. Changing between them never restarts
a game and never logs anything out.

The interesting part is what happens to accounts that no longer fit. Going from
four places to two leaves two accounts homeless. They are not closed. They keep
running, keep ticking, keep earning whatever an idle game earns while nobody is
watching — they are simply moved somewhere the window is not showing. Their old
place is remembered, and choosing a bigger arrangement puts them back in it.
That distinction, out of sight but still running, is the single most important
idea in the whole application.

It is also the one that needs the most care, and the reason this is its own
slice. The obvious way to show one thing at a time is a standard container that
swaps between children, and it is exactly wrong here: it tears down the children
it is not showing, and the engine that runs the games treats a torn-down page as
hidden and slows it to a crawl. For an idle game, slowed to a crawl means lost
progress. So instead the homeless accounts are given real positions outside the
window's visible area, inside a container that clips anything outside its own
bounds. They are still fully alive and still being laid out; they are just
drawn where nobody can see them. The code that does this deserves a comment
saying why the obvious approach was rejected, because the next person to read it
will otherwise simplify it back into the bug.

None of the deciding happens here. The window asks the rules half of the
program what the new arrangement means, gets back a place or "out of sight" for
every account, and obeys it.

## User experience

- **Flow** — pick one of three arrangements from the header bar: one slot, two
  side by side, or four in a two-by-two grid. The toggles behave as one linked
  choice with three answers.
- **Flow** — press "Add game" when every slot is full: the new account takes the
  focused slot and pushes that slot's previous occupant out of sight.
- **States** — **out of sight**: an account with no slot has no representation
  anywhere in this item — nothing on screen says it exists. That gap is
  deliberate and is what the sidebar in a later roadmap item fills.
- **New pattern** — the slot grid with its out-of-sight holding area.
  `docs/design.md` owes a rule about slots, focus and the out-of-sight state
  once this ships.

## Technical details

- **Architecture** — rule 8: the layout toggles send the chosen layout to the
  core, which returns the new visibility of every session; `session_grid.rs`
  re-places children from that result and decides nothing.
- **Architecture** — the grid's custom `LayoutManager` allocates an off-grid
  child a rectangle whose origin is outside the widget's own bounds, and the
  grid sets `set_overflow(gtk::Overflow::Hidden)` so that child is clipped
  rather than unrealised.
- **Code standards** — rule 18: comment the clipping trick with the constraint
  that forced it — a `GtkStack` unrealises the children it is not showing, and
  the engine treats an unrealised view as hidden, which throttles the game.
- **Architecture** — rule 10: re-placement runs on the GTK main context; no
  child widget is created or destroyed by a layout change.
- **Architecture** — rule 14 and code standards rule 25: this behaviour is
  covered by the item's `test-script.md`, because no test in this repository may
  require a display server.

## Acceptance criteria

- [x] `(manual)` each of the three header-bar toggles redraws the grid as one,
      two or four rectangles with a hairline between neighbours, and choosing
      one releases the other two
- [x] `(manual)` with four accounts in the two-by-two arrangement, choosing "two
      side by side" leaves the first two where they are and draws the other two
      nowhere in the window
- [x] `(manual)` choosing the two-by-two arrangement again returns each of those
      two accounts to the slot it held before
- [x] `(manual)` an out-of-sight account is still realised and still allocated —
      `GTK_DEBUG=interactive` shows the widget present with an allocation
      outside the grid's bounds — and it is the same widget object as before the
      change, not a rebuilt one
- [x] `(manual)` with every slot occupied, adding another account puts it in the
      focused slot and pushes that slot's previous occupant out of sight
- [x] `(manual)` an out-of-sight account whose remembered slot was taken while
      it was away lands in the lowest-numbered free slot when the arrangement
      grows
- [x] `(manual)` choosing an arrangement with more slots than there are accounts
      leaves the extra slots empty and draws nothing in them

## References

- [Roadmap item](../../roadmap/01-isolated-accounts-and-layouts/README.md) — the full picture, including the "Changing the arrangement" diagram
- [Wireframes](../../roadmap/01-isolated-accounts-and-layouts/wireframes/) — `main-window.md` describes the slot rectangles, the focus marker and the clipping
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13 and 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 18 and 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4 and 7
- [`docs/design.md`](../../design.md) — no numbered rules yet; the slot grid is one of the patterns it owes a rule for

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
