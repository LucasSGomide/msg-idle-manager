# 03 — The row menu and the engine's switches

**Roadmap:** [04](../../roadmap/04-keep-awake/README.md) · **Scope:** front-end · **Depends on:** 01, 02

## Context

The program lists every game account down the leading edge of the window, one row
each, showing the account's name, where it sits, and a button that stops or
starts it. This slice puts a second control on that row — a small three-dot menu
— and makes the one thing inside it work.

The menu holds a single checkable line: "Keep running when hidden". It lives in a
menu rather than on the row because it is a setting somebody decides once for an
account and then forgets, unlike the stop-and-start button, which is pressed all
the time. Putting it on the row would spend permanent space on something used
once.

Choosing the line does two things. The account's record of the setting flips, and
the browser engine is told to stop slowing that account's page down when nobody
is looking at it. The engine offers two internal switches for that — one that
stretches out a hidden page's timers, one that suspends its animations — and both
are turned off for that account alone. Every other account is untouched, because
these switches belong to a single page rather than to the whole program.

Then the page reloads. The reload is not decoration: the second half of this
feature, added in the next slice, is a piece of script that has to be in place
before the game's own code runs, and there is no way to hand a page that script
after it has already loaded. Reloading once is the cost of turning the setting on,
and the row says so — from the moment the line is chosen until the page has
painted again, the row reads the same way it reads while an account is starting
up from cold, with its stop-and-start button not pressable. That is deliberately
the same wording and the same interval, because it is the same situation from the
user's point of view.

Choosing the line a second time turns the setting off, turns both switches back
on, and reloads once more. Choosing a value the account already holds does
nothing at all — no reload, no interruption — because the logic layer reports
whether anything actually changed and the menu handler respects the answer.

The setting belongs to the account, not to the place it happens to occupy. Moving
it between slots, pushing it out of sight and bringing it back, or stopping and
starting it, all leave it exactly as it was, and a restarted account's brand-new
page is built with the same switches already off.

## User experience

- **Entry** — a switch on each row of the sidebar list, reached from the row's
  menu rather than sitting on the row itself, because it is set once per account
  and then forgotten.
- **Flow** — open the row menu for an account and turn "keep running when hidden"
  on. The account's page reloads once, and from then on it runs at full speed
  whether or not it has a place on screen.
- **Flow** — turn it off again and the page reloads once more, back to the
  engine's ordinary background behaviour.
- **Flow** — the flag stays with the account. Moving it between slots, pushing it
  out of sight and bringing it back, and parking it and starting it again never
  change it; the new view a started account gets is built with the same settings.
- **States** — **off**: the menu item reads as off and nothing is changed on the
  page. **On**: the menu item reads as on. **Reloading**: from toggling until the
  page paints again, the row's marker reads as reloading — the `Starting` word and
  blue dot item 03 already draws (design rule 1), with the row's action button
  insensitive. The always-visible indication that the setting is on is the next
  slice, not this one.
- **New pattern** — a per-row settings menu, which no earlier item needed because
  every earlier control was a single action. `docs/design.md` owes a rule for
  where a row's settings live and how they differ from a row's actions.

## Technical details

- **Front-end** — the row factory in `session_sidebar/imp.rs` grows a
  `gtk::MenuButton` after item 03's action button, drawn with the conventional
  three-dot icon and no label, opening a `gio::Menu` with one item bound to a
  stateful `gio::SimpleAction`. Its boolean state is set on every bind, like the
  dot's classes, because the list recycles row widgets (architecture rules 12
  and 13).
- **Front-end** — `Row` gains an `is-kept-awake` boolean property in
  `session_sidebar/row/imp.rs`, written by `Row::refresh` from the session's
  flag, so the factory sets the action's state without reaching into the book
  (naming rule 12).
- **Front-end** — `SessionSidebar` exposes the choice as an intent carrying the
  account's id and the value asked for, through a `connect_*` handler beside
  `connect_parking_toggled`. The menu decides nothing itself; the reload is the
  shell's response to what the domain returned (architecture rule 8).
- **Front-end** — `window/imp.rs` handles it in the order the item's first
  diagram fixes: ask the book to set the flag, stop if it reports that nothing
  changed, otherwise redraw so the row reads as reloading, then tell that
  account's holder to apply it, then connect the resulting load to the existing
  `finish_starting` so the interval ends at the first paint — the same
  `LoadEvent::Committed | Finished` branch `start_session` already uses.
- **Front-end** — `SessionView::set_keep_awake(bool)` in `web_view.rs` calls
  `Settings::set_feature_enabled(&feature, !on)` (`webkit6` 0.6.1,
  `src/auto/settings.rs:1157`) on the current view's settings object for each
  feature in the lazily-initialised list task 02 built — never re-searching
  `all_features()` per view — logs at `debug` which features it set for which
  account, then reloads the view. Settings are per view, not per context
  (`src/auto/web_view.rs:157`), which is what makes this a per-account setting
  rather than an application-wide one.
- **Front-end** — the holder remembers the value and `SessionView::start` applies
  it to the fresh view it builds, so parking an account with keep-awake on and
  starting it again produces a view carrying the same switches (`FR.6.1`,
  `FR.6.2`). A feature missing from this build is the `tracing` warning task 02
  wired, and the session runs without it (code standards rules 14, 15).
- **Design** — add the rule the item names as owed: a setting a row carries lives
  behind the row's menu, a one-click action lives on the row itself. Design rule
  2 fixed one inverting button as the row's action and is not extended to carry a
  setting; a menu would hide a one-click action behind two clicks, and a row
  control would spend permanent space on something chosen once.
- **Testing** — architecture rule 14 and code standards rule 25 keep GTK out of
  `cargo test`, so everything a screen shows is `(manual)` and this slice's
  evidence is the item's `test-script.md`.

## Acceptance criteria

- [ ] `(manual)` every row in the sidebar carries a three-dot menu button on its
      trailing edge, after the Park/Start button
- [ ] `(manual)` opening the menu shows one checkable item, "Keep running when
      hidden", unchecked for a newly added account
- [ ] `(manual)` choosing it closes the menu, reloads that account's page exactly
      once, and logs at `debug` that both hidden-page features were disabled for
      that account
- [ ] `(manual)` reopening the menu shows the item checked; choosing it again
      re-enables both features and reloads exactly once more
- [ ] `(manual)` choosing the value the account already holds reloads nothing and
      changes nothing on screen
- [ ] `(manual)` from the moment the item is chosen until the page paints, the
      row's marker reads `Starting` with its blue dot and the row's action button
      is insensitive, then returns to the marker it had before
- [ ] `(manual)` with keep-awake on and the account hidden, a page timer set to
      tick once a second keeps that rate, and with keep-awake off the same timer
      is stretched — both readings recorded in `test-script.md`, using the case
      task 02 found actually marks a page hidden
- [ ] `(manual)` turning keep-awake on for one account leaves every other
      account's page loaded, running and untouched
- [ ] `(manual)` an account with keep-awake on that is parked and started again
      comes back with the menu item still checked and both features disabled on
      its new view, and keeps the setting through a layout change and through
      going out of sight and back

## References

- [Roadmap item](../../roadmap/04-keep-awake/README.md) — the full picture,
  including the "Turning keep-awake on for an account" interaction diagram and
  the Technical References for `webkit6`
- [Wireframes](../../roadmap/04-keep-awake/wireframes/) — the row settings menu,
  where the menu button sits and what choosing the item does
- [`docs/requirements.md`](../../requirements.md) — `FR.6.1`, `FR.6.2`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 6, 14, 15, 18, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 12
- [`docs/design.md`](../../design.md) — rules 1 and 2, which this slice must stay
  true to; the row-settings rule is the one it adds

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
