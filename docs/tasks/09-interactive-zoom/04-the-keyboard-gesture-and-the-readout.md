# 04 — Zooming the focused account from the keyboard

**Roadmap:** [09](../../roadmap/09-interactive-zoom/README.md) · **Scope:** front-end · **Depends on:** 02

## Context

This is the slice where the feature becomes something a person can do. Until now
the rules layer knows what a step means and which size applies; nothing on screen
can ask for one.

The window divides itself into one, two or four places, and one of those places
is the focused one — the last one clicked. Holding the control key and pressing
plus makes the game in that place one step bigger, minus makes it one step
smaller, and zero puts it back to the size the game's own file asks for. The keys
work whenever the window is focused, whether or not the game itself is where
typing would go, because a game page that binds keys on its own canvas must not
be able to swallow them.

The game does not reload. Its page is resized where it stands, still running,
still logged in, still counting — which matters far more here than it would in a
browser, because these are games that lose progress when interrupted and the
entire point of the application is to leave them going.

Every gesture also puts a small percentage figure over the place it affected,
which fades on its own after about a second and leaves nothing behind. That is
not decoration. It is the only way three otherwise-invisible cases read as
working: a game already at the largest or smallest size the application allows
does not change, and without the figure the keypress would look lost; and a place
holding an account the user has paused has no page to resize at all, so the
figure over the standing-in panel is the entire visible result — the size takes
effect the next time that account starts.

With nothing on screen at all the keys do nothing and draw nothing, rather than
reaching for a game that is not there.

The figure is drawn as one more layer on the same stack each place already
carries for the name label and the paused-account panel, so nothing about how a
place is built or measured changes. Persisting any of this across a restart is a
later slice; here the sizes live only as long as the window does.

## User experience

- **Entry** — no new control anywhere. One gesture on the main window: the
  control key with plus, minus or zero. The header bar, the sidebar and the row
  menu are untouched.
- **Flow** — press control and plus (or control and equals) and the game in the
  active place grows one step, about ten percent. Control and minus shrinks it
  the same amount. Press control and zero and it goes back to the size its game
  file asks for, forgetting the size that was chosen for the current arrangement.
  The keys work whenever the window has focus, whether or not the game itself is
  where typing would go.
- **Flow** — every gesture puts a percentage figure over the affected place. It
  fades after about a second and leaves nothing behind.
- **States** — **nothing on screen**: the keys do nothing at all rather than
  reaching for a game that is not there. **At the limit**: a further step in the
  same direction leaves the size where it is and still shows the figure, so the
  gesture is visibly received rather than silently dropped. **Parked account in
  the active place**: there is no page to resize, so the remembered size changes
  and takes effect the next time that account starts; the figure still appears
  over the panel standing in for it, because a keypress that shows nothing reads
  as a keypress that was lost.
- **Pattern** — the figure is drawn as one more layer over the same per-place
  stack that already carries the name cover and the parked panel
  (`crates/idle-manager-shell/src/session_grid/imp.rs:137`), so nothing about how
  a place is built or allocated changes. The parked panel it may appear over is
  design rule 4's plain centred panel, unchanged: the figure sits above it and
  neither one is restyled.
- **New pattern** — a transient readout over a place, shown on a gesture and
  faded out on a timer. No numbered rule in `docs/design.md` covers a transient
  acknowledgement drawn over live content, and the doc owes one once this ships:
  where the figure sits, how long it lasts, and why it is never a permanent
  fixture.

## Technical details

- **Front-end** — `web_view.rs` gains `SessionView::set_zoom`, shaped on
  `set_keep_awake` at `crates/idle-manager-shell/src/web_view.rs:176`: it
  remembers the value on the holder whether or not a view exists, and applies it
  to the live view when there is one. It differs in the one way that matters —
  no reload, because `set_zoom_level` at
  `crates/idle-manager-shell/src/web_view.rs:267` takes effect on the running
  page, so the account never leaves `Live` (`FR.11.5`). Remembering it on the
  holder is what makes a parked account start again at the size chosen while it
  was parked.
- **Front-end** — the window's existing capture-phase key controller at
  `crates/idle-manager-shell/src/window/imp.rs:168` gains the branches rather
  than a second controller being added, keeping one place that decides what a
  keypress means. Capture phase because `FR.11.1` says the gesture is never gated
  on a web view holding keyboard focus.
- **Front-end** — it matches the control modifier with the plus, equals and
  keypad-add keys for a step in, minus and keypad-subtract for a step out, and
  zero and keypad-zero for a reset, because which of those a keyboard actually
  delivers depends on its layout. Whichever set the keyboard under test turns out
  to deliver is recorded in a comment naming the constraint (code standards
  rule 18).
- **Front-end** — each branch asks the book for the focused account, calls the
  matching transition, hands the returned size to that account's holder and asks
  the grid to show the figure over that place. An empty grid, or a focused place
  holding nothing, returns `None` from the book and stops there.
- **Front-end** — the readout is a third overlay child per place, built with the
  entry in `register_slot` beside the name cover and the parked panel, hidden by
  default. The grid gains a method that sets its text to the percentage, shows
  it, and arms a `glib::timeout_add_local_once` to hide it again, cancelling and
  rearming any timer already running for that place so a run of gestures shows
  one figure rather than a queue of them.
- **Front-end** — it is styled from a new `session-grid.css` registered in
  `resources/idle-manager.gresource.xml` and installed on the default display
  once, the same way `sidebar.css` is at
  `crates/idle-manager-shell/src/session_sidebar/imp.rs:172`; naming rule 4 puts
  it beside `session-grid.ui` under the same stem. Architecture rules 12 and 13
  keep the private state in `imp` and the markup in the bundle.
- **Design** — rule 4's panel keeps its exactly three stacked elements and gains
  no fourth; rule 6 — the figure is an overlay, costs no layout and leaves the
  sidebar's derived width untouched. The transient readout is the item's one new
  pattern, and `docs/design.md` owes a rule for it — where it sits, how long it
  lasts, and why it is never a permanent fixture — written once this slice ships.
  The fade duration is a named constant with its unit in the name (code standards
  rule 5).
- **Testing** — shell behaviour is `(manual)` against the headless harness,
  because `cargo test` never requires a display server (architecture rule 14,
  code standards rule 25). This is the item's first accepted slice, so it writes
  `test-script.md`'s `## Setup` and `## Teardown` sections as well as its own.

## Acceptance criteria

- [x] `(manual)` control and plus grows the focused place's game in place and a
      percentage figure appears over that place reading the new size
- [x] `(manual)` control and equals, and the keypad add key, produce the same
      step on the keyboard under test
- [x] `(manual)` control and minus shrinks the same game by the same step
- [x] `(manual)` control and zero returns the game to the size its game file asks
      for, and the figure reads that percentage
- [x] `(manual)` no gesture reloads the page: a running game keeps its counter
      and its login across a run of steps, and its sidebar row never reads
      starting
- [x] `(manual)` at the largest and the smallest size the application allows, a
      further step in the same direction leaves the page unchanged and still
      shows the figure
- [x] `(manual)` with a parked account in the focused place the figure appears
      over the parked panel, the panel keeps its three elements, and starting the
      account opens it at the size chosen while it was parked
- [x] `(manual)` with an empty grid the keys do nothing and no figure appears
      anywhere
- [x] `(manual)` a run of steps shows one figure that keeps updating rather than
      a queue of them, and it fades about a second after the last step
- [x] `(unit)` `session-grid.css` is readable from the compiled resource bundle,
      as the window and placeholder templates are

## References

- [Roadmap item](../../roadmap/09-interactive-zoom/README.md) — the full picture,
  including the "Zooming the account in the active place" diagram whose keyboard
  path this slice implements
- [The zoom readout wireframe](../../roadmap/09-interactive-zoom/wireframes/zoom-readout.md)
  — the whole item's screen; this slice draws the live-game, at-the-limit and
  parked-account variants and leaves the arrangement switch to task 06
- [`docs/requirements.md`](../../requirements.md) — `FR.11.1`, `FR.11.2`,
  `FR.11.4`, `FR.11.5`, `FR.11.6`
- [`docs/design.md`](../../design.md) — rules 4 and 6, and the transient-readout
  rule this slice's pattern owes
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 7, 13, 14, 15,
  17, 18, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
