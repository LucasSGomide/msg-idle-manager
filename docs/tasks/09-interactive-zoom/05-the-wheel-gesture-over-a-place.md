# 05 — Zooming the account under the pointer

**Roadmap:** [09](../../roadmap/09-interactive-zoom/README.md) · **Scope:** front-end · **Depends on:** 04

## Context

The keyboard gesture acts on the focused place — the last one clicked. That is
the right answer for a keypress, which has no position, and it leaves the gesture
every browser has trained people to expect unserved: hold control, turn the mouse
wheel, and the thing under the pointer changes size.

So this slice adds the second entry point, with one thing the browser gesture
does not have. Holding control and turning the wheel over a game grows or shrinks
that game one step per notch, and the same percentage figure the keyboard gesture
shows appears over that place — but only over the focused place. A window here
holds up to four games running at once, unlike a browser tab, and the pointer
spends its time resting wherever it was last left rather than on the thing being
read. A wheel that resized whatever the pointer happened to be crossing changed
sizes nobody was looking at. The click is what arms the wheel: point at a game
you have chosen, and the gesture is the browser's; point at a neighbour, and the
wheel is a wheel. The game the pointer is on is always one that is on screen, so
there is no case where this reaches something invisible.

The other thing that has to be right is what a notch is worth. A step is a notch,
not a distance: the wheel controller has to hand over one step per physical notch
whatever the device, because a high-resolution wheel and a touchpad both report a
single notch as a run of small fractional deltas, and a step taken per delta
moves the size by an amount that varies with the hardware.

Two things have to be right or the gesture is worse than not having it. The page
underneath must not also scroll while control is held — otherwise every step
drags the game's own content around — so the event is consumed rather than passed
on. And without control the wheel has to behave exactly as it always has, because
these are scrollable pages and taking that away would be a bug, not a feature.

The awkward part is where the listener lives. Each place is a stack: the game's
page, the label that covers it while it loads, the panel that stands in when the
account is paused, and now the figure. The page itself is destroyed and rebuilt
every time an account is paused and started again, so a listener attached to it
would quietly stop working after the first pause. It goes on the stack instead,
which lives as long as the account does. Whether that stack actually sees a wheel
event on its way to a game page is genuinely unknown — the browser engine handles
scrolling inside its own separate process — and this slice is where that gets
measured. If the page wins, the listener has to move onto the page and be
re-attached every time one is built.

Both entry points hand the same request to the rules layer, which is what makes
it impossible for the two to disagree about the size or about which arrangement
it was recorded against.

## User experience

- **Entry** — no new control. The second of the item's two gestures: the control
  key with the mouse wheel over a game.
- **Flow** — click a game to focus its place, then hold control and turn the
  wheel over it: that game grows or shrinks one step per notch, the same step the
  keyboard takes, whatever the pointing device. The page underneath does not
  scroll while control is held.
- **Flow** — every gesture puts a percentage figure over the affected place. It
  fades after about a second and leaves nothing behind.
- **States** — **at the limit**: a further notch in the same direction leaves the
  size where it is and still shows the figure, so the gesture is visibly received
  rather than silently dropped. **Parked account under the pointer**: there is no
  page to resize, so the remembered size changes and takes effect the next time
  that account starts, and the figure still appears over the panel standing in
  for it. **An unfocused place under the pointer**: nothing resizes and no figure
  appears — the wheel behaves as it does without the gesture, and the page
  scrolls. Clicking that place first is what arms it. **Out of sight**: an
  account with no place on screen cannot be reached by this gesture at all.
- **Pattern** — the figure is task 04's readout, unchanged: the same layer over
  the same per-place stack that already carries the name cover and design rule
  4's parked panel
  (`crates/idle-manager-shell/src/session_grid/imp.rs:137`).

## Technical details

- **Front-end** — `session_grid/imp.rs` adds a vertical `EventControllerScroll`
  to each `SlotEntry`'s existing `gtk::Overlay` — the overlay, not the view,
  because the overlay lives for the account's whole life while the view is
  destroyed and rebuilt on every park and start, and the controller must survive
  that. Vertical is the only axis a zoom step reads, and the controller needs
  explicit flags to receive anything at all.
- **Front-end** — the flags carry `DISCRETE` beside `VERTICAL`, so GTK
  accumulates the device's smooth deltas and the handler is called once per
  physical notch with a ±1 delta. Without it a high-resolution wheel or a
  touchpad delivers several fractional-delta events per notch, each one a whole
  step, and a notch compounds into two or three — the step stops being the ±10%
  `FR.11.1` names and starts varying with the hardware. The flag is the
  constraint that forced it and is named in a comment (code standards rule 18).
- **Front-end** — the handler emits nothing unless the entry it belongs to is the
  focused one, which it asks the grid through a small `is_focused_session`
  helper resolving the focused `SlotId` the way `reload_focused` already does.
  Over an unfocused place it returns `Proceed` untouched, so that page scrolls
  and nothing is drawn. This is the one place the two gestures differ from the
  browser they borrow from, and the reason is a window holding four live games
  rather than one tab.
- **Front-end** — it runs in the capture phase for the same reason the grid's own
  click gesture does at
  `crates/idle-manager-shell/src/session_grid/imp.rs:92`: the web view would
  otherwise consume the event on the way down.
- **Front-end** — when the control modifier is held it emits the step intent
  carrying the entry's `SessionId` and returns `glib::Propagation::Stop`, so the
  page does not also scroll (`FR.11.3`); otherwise it returns `Proceed` untouched
  and the page scrolls as normal. The modifier state comes from the controller's
  current event rather than from a tracked key.
- **Front-end** — the intent reaches the window through a handler registered
  exactly like `connect_start_requested`, so both halves of the gesture end up in
  the same window method and the domain is the only thing that decides what a
  step means (architecture rule 8). That method already exists from task 04; this
  slice gives it a second caller, handing it the id of the account under the
  pointer — which the gate has already established is the focused one.
- **Front-end** — the readout call is task 04's, over the pointed-at place rather
  than the focused one, with the same cancel-and-rearm timer.
- **Front-end** — the item's second blocker is settled here by measurement: if
  the web view consumes the wheel event before the overlay's capture-phase
  controller sees it, the controller moves onto the view itself and
  `SessionView::start` re-attaches it on every rebuild. Whichever way it goes is
  recorded in a comment naming the constraint that forced it (code standards
  rule 18).
- **Testing** — `(manual)` against the headless harness (architecture rule 14,
  code standards rule 25), appended to `test-script.md` as this task's own
  section, reusing the `## Setup` steps task 04 wrote.

## Acceptance criteria

- [x] `(manual)` control and one wheel notch up over the focused game grows it
      one step, and one notch down shrinks it one step
- [x] `(manual)` the percentage figure appears over the place the pointer is on
- [x] `(manual)` the page underneath does not scroll while control is held, on a
      game page long enough to scroll
- [x] `(manual)` the wheel without control scrolls the page exactly as it did
      before this slice
- [x] `(manual)` one notch is exactly one step on every pointing device to hand —
      the figure reads the same percentage after one notch as after one keypress,
      and a high-resolution wheel or a touchpad does not take two or three steps
      for one notch
- [x] `(manual)` with the pointer over a place that is not the focused one,
      nothing changes size and no figure appears; clicking that place first and
      repeating the gesture then steps it
- [x] `(manual)` parking an account and starting it again leaves the wheel
      gesture still working over that place
- [x] `(manual)` at the largest and smallest size the application allows, a
      further notch leaves the page unchanged and still shows the figure
- [x] `(manual)` a notch over a focused parked account's panel changes its
      remembered size and shows the figure, and the account starts at that size
- [x] `(manual)` an account with no place on screen is reachable by no wheel
      gesture anywhere in the window, and comes into a place at the size it
      already had

## References

- [Roadmap item](../../roadmap/09-interactive-zoom/README.md) — the full picture,
  including the "Zooming the account in the active place" diagram whose wheel
  path this slice implements, and the second blocker this slice settles
- [The zoom readout wireframe](../../roadmap/09-interactive-zoom/wireframes/zoom-readout.md)
  — the whole item's screen; this slice reaches the same live-game,
  at-the-limit and parked-account variants through the pointer
- [`docs/requirements.md`](../../requirements.md) — `FR.11.3`, `FR.11.4`,
  `FR.11.5`, `FR.11.6`, `FR.11.7`, `FR.11.8` — the last narrows `FR.11.3` to the
  focused place
- [`docs/design.md`](../../design.md) — rules 4 and 6, and the transient-readout
  rule the item's pattern owes
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 7, 13, 17, 18,
  25
- [`docs/naming.md`](../../naming.md) — rules 2, 7, 11
- [`docs/stack.md`](../../stack.md) — `gtk4` and `webkit6` versions the
  controller's flags and propagation are pinned against

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
