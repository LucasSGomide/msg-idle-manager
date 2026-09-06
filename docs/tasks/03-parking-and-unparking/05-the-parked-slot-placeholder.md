# 05 — The parked slot placeholder

**Roadmap:** [03](../../roadmap/03-parking-and-unparking/README.md) · **Scope:** front-end · **Depends on:** 04

## Context

An account can be parked while still holding its place on screen. That is the
case someone wants when they mean "keep this game where I put it, I will come
back to it in an hour, and I do not want to pay for it meanwhile". Up to this
slice, that place shows the plain cover of the account's name that the program
already draws while a game is loading — honest, but it says nothing about why
the game is not there, and it offers no way to start the account from where it
sits.

This slice replaces it with a proper panel: the account's name, a line of text
saying it is parked, and a "Start" button. Pressing that button does exactly
what pressing "Start" on the account's row does. Someone looking at the window
rather than at the list can act on what they see.

The panel is deliberately plain — centred text and one button on the window's
own background, with generous space around them. It stands where a game was,
among other places that do hold games, and it should not compete with them for
attention.

Two things about it are settable rather than fixed text: the line of state text
and the button's label. A later item uses this same panel to report an account
that has stopped responding, with different words and a different action.
Making those two settable now means that item reuses the widget instead of
copying it, and the two panels cannot drift apart.

The panel follows the same shape as every other widget in the program: a
description of the layout in a template file compiled into the application, and
a small object beside it holding the private implementation.

## User experience

- **Entry** — the panel appears in the place a parked account holds, standing in
  for its game.
- **Entry** — its "Start" button is a second way to start an account, equivalent
  to the one on the account's row.
- **Flow** — park an account that holds a place and its game is replaced by the
  panel. Press the panel's "Start" and the account starts; the new view replaces
  the panel once the page paints.
- **States** — **parked in a place**: the account's name, a line reading
  `Parked`, and a `Start` button, centred with generous space on the window's
  background. **Starting**: the line reads `Starting` and the button is
  insensitive, matching the row.
- **Flow** — move a parked account between places and its panel moves with it;
  move it out of sight and the place it left is empty.
- **New pattern** — the placeholder that stands in for a game in a place.
  `docs/design.md` owes a rule for it, and whatever this screen settles binds
  item 08, which renders its failure panel from the same widget.

## Technical details

- **Front-end** — add `crates/idle-manager-shell/src/slot_placeholder.rs` with
  its `imp` module and `resources/ui/slot-placeholder.ui`, listed in
  `resources/idle-manager.gresource.xml` and declared in the shell's `lib.rs`
  (architecture rules 12 and 13; naming rules 1, 2, 4 — the module file is
  snake_case, the template kebab-case).
- **Front-end** — a centred `gtk::Box` carrying the `background` style class, as
  the grid's loading cover already does: a name label, a state label and one
  `gtk::Button`. Naming rule 9 — the type is `SlotPlaceholder`, named for what
  it is.
- **Front-end** — the name, the state text and the button label are `glib`
  properties, not markup, because item 08 renders its failure panel from this
  same widget with different words and a different action. The press is exposed
  as a handler the way `SessionSidebar` exposes row activation (architecture
  rule 8).
- **Front-end** — `session_grid` puts the panel into the parked session's
  `SlotEntry` overlay in place of the view, and swaps it back out when the new
  view arrives. This replaces task 03's stand-in, which simply left the loading
  cover showing.
- **Front-end** — the panel's press reaches `window/imp.rs` as the same start
  intent the row button sends, so both paths run one start and not two.
- **Design** — the placeholder is the second pattern this item owes
  `docs/design.md` a rule for. Write it in design rule 1's shape — one
  imperative and one line of why — appended, never renumbered.
- **Testing** — architecture rule 14 and code standards rule 25: `(manual)` for
  everything a screen shows. The template's presence in the compiled bundle
  needs no display server and is checked the way `lib.rs` already checks
  `window.ui`.

## Acceptance criteria

- [x] `(unit)` `ui/slot-placeholder.ui` resolves from the registered `GResource`
      bundle, the way `window.ui` already does
- [x] `(manual)` parking an account that holds a slot replaces its game with a
      panel showing the account's name, a line reading `Parked` and a `Start`
      button, centred on the window's background
- [x] `(manual)` pressing the panel's `Start` starts the account, and the new
      view replaces the panel once the page paints
- [x] `(manual)` while the account is starting, the panel's line reads
      `Starting` and its button is insensitive, matching the row
- [x] `(manual)` the panel's state line and button label are properties: setting
      them to item 08's wording changes the panel with no markup change
      (`GTK_DEBUG=interactive` shows both properties on the widget)
- [x] `(manual)` moving a parked account between slots carries its panel with
      it, and moving it out of sight leaves the slot it held empty

## References

- [Roadmap item](../../roadmap/03-parking-and-unparking/README.md) — the full
  picture, including both interaction diagrams
- [Wireframe](../../roadmap/03-parking-and-unparking/wireframes/parked-account.md)
  — the panel's contents, its plainness, and the item 08 reuse it has to allow
- [`docs/requirements.md`](../../requirements.md) — `FR.5.1`, `FR.5.5`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rule 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7, 9
- [`docs/design.md`](../../design.md) — rule 1 for the shape to write in; the
  slot placeholder is the rule this slice adds

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
