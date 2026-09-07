# 06 — Snapping every account on an arrangement switch

**Roadmap:** [09](../../roadmap/09-interactive-zoom/README.md) · **Scope:** front-end · **Depends on:** 05

## Context

The window can divide itself three ways: one game filling it, two side by side,
or four in a grid. A game that reads comfortably filling the whole window is half
or a quarter of that size in the other two, so a size chosen while four games are
showing is a size for four games and nothing else. The rules layer has recorded
sizes that way since the first slice of this item. Nothing on screen has acted on
it yet: switching arrangement today leaves every game at whatever size it
happened to be.

This slice closes that. Pressing one of the three arrangement buttons switches
the arrangement as it always has, and then every account is redrawn at whatever
size was last chosen for it in the arrangement being switched to — falling back
to the size its game file asks for wherever nothing has been chosen. Someone who
spends an evening getting four games comfortable, switches to one game
full-window, and switches back gets their four sizes handed back to them rather
than having to redo the work.

Deliberately, no percentage figure appears. The figure exists to acknowledge a
gesture, and nobody made one here — the arrangement did. Showing four figures
after a button press would read as four things having gone wrong.

The one non-obvious part is which accounts get snapped. It is every account
holding a running page, not only the ones with a place on screen. An account with
no place is invisible, so snapping it costs nothing visible, but it means that
when it is later brought into a place it is already the right size for the
arrangement in force — with no second piece of code to keep in step with this
one, and no flicker as a game is drawn wrong and then corrected.

Nothing is written to disk here. An arrangement switch consumes chosen sizes and
never records one; making them survive a restart is the next and last slice.

## User experience

- **Flow** — switch arrangement and every game already on screen snaps to the
  size last chosen for it in that arrangement, with no figure shown, because
  nobody asked for a change — the arrangement did.
- **States** — **nothing chosen for the new arrangement**: that account is drawn
  at the size its game file asks for, silently.
- **States** — **Out of sight**: an account with no place on screen takes its
  remembered size for the current arrangement when it is next brought into a
  place, with no figure and no visible correction.
- **States** — **Parked account**: there is no page to resize, so its remembered
  size for the new arrangement takes effect the next time it starts.
- **Pattern** — the readout built in task 04 is deliberately not called on this
  path; the wireframe's fourth variant is an arrangement switch showing no figure
  over any place.

## Technical details

- **Front-end** — `connect_layout_toggle` at
  `crates/idle-manager-shell/src/window/imp.rs:210` already tells the book to
  switch and then redraws; after the switch it now walks every account that has a
  holder with a live view and calls `SessionView::set_zoom` with that account's
  `zoom_for` the new layout.
- **Front-end** — every live account rather than only the visible ones, which is
  what satisfies `FR.11.7` for free: an account out of sight is already at the
  right size for the arrangement when it is next brought into a place, and there
  is no second code path to keep in step.
- **Front-end** — no readout is shown on this path (`FR.11.6`): the figure
  acknowledges a gesture and there was none.
- **Front-end** — `set_zoom` on a parked account's holder still records the size
  without a view, so a parked account carried through a switch starts at the new
  arrangement's size; that is task 04's holder behaviour with a second caller,
  not a new mechanism.
- **Front-end** — nothing is written: an arrangement switch consumes remembered
  sizes and never records one (`FR.12.4`). The existing `request_save` on this
  path still saves the workspace, which has always carried the arrangement, and
  is untouched.
- **Design** — the absence of the figure is the design, and it is part of the
  transient-readout rule `docs/design.md` owes once the item ships: an
  acknowledgement is for something the user did.
- **Testing** — `(manual)` against the headless harness (architecture rule 14,
  code standards rule 25), appended to `test-script.md` as this task's own
  section, reusing the `## Setup` steps task 04 wrote.

## Acceptance criteria

- [ ] `(manual)` set one size in the four-place arrangement and a different one
      in the single arrangement, then switch back and forth: each arrangement
      returns every game to its own size
- [ ] `(manual)` no percentage figure appears over any place on an arrangement
      switch
- [ ] `(manual)` an account with nothing chosen for the arrangement being
      switched to is drawn at the size its game file asks for
- [ ] `(manual)` an account off the grid during a switch is already at the right
      size the moment it is brought into a place, with no visible correction and
      no figure
- [ ] `(manual)` a parked account carried through a switch starts at the size
      chosen for the arrangement now in force
- [ ] `(manual)` no game reloads across a switch: a running counter and a login
      both survive, and no row passes through starting

## References

- [Roadmap item](../../roadmap/09-interactive-zoom/README.md) — the full picture,
  including the "Switching arrangement snaps every account to its remembered
  size" diagram this slice implements
- [The zoom readout wireframe](../../roadmap/09-interactive-zoom/wireframes/zoom-readout.md)
  — the whole item's screen; this slice is its fourth variant, the arrangement
  switch that shows no figure
- [`docs/requirements.md`](../../requirements.md) — `FR.11.6`, `FR.11.7`,
  `FR.12.2`, `FR.12.4`
- [`docs/design.md`](../../design.md) — rules 4 and 6, and the
  transient-readout rule the item's pattern owes
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 6, 7, 15, 17, 25
- [`docs/naming.md`](../../naming.md) — rules 2, 11

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
