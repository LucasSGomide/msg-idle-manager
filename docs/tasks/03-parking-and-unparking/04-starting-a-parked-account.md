# 04 — Starting a parked account

**Roadmap:** [03](../../roadmap/03-parking-and-unparking/README.md) · **Scope:** front-end · **Depends on:** 03

## Context

Parking an account shut it down and gave its memory back. This slice is the
other half: pressing the same button starts it again, and the game comes back
already logged in.

The button's label inverts with the account's state — "Park" while it runs,
"Start" once it is parked — so one control on the row covers both directions and
there is never a second button to hunt for.

Starting is not instant. A whole page has to load, and until it does the account
is in a third state the program has to name: starting. During that window the
row's marker says so and the button cannot be pressed, so an impatient second
press cannot start two views for one account. The state lives in the pure logic
layer rather than as a flag on a widget, which is what makes the double press
impossible everywhere rather than in one handler.

What makes any of this work is the constraint the previous slice was built
around: the browser engine ties a view to its private storage area at the moment
the view is created, and never afterwards. So starting cannot revive the view
that was thrown away; it builds a brand new one against the storage area that
was kept the whole time. That storage area still holds the account's cookies,
which is why the game loads straight into the account rather than to a login
page. The absence of a login prompt is the observable proof that parking touched
none of the account's data, and it is the single most important thing to check
when accepting this slice.

An account's place on screen is untouched by any of it. Start a parked account
that was out of sight and it stays out of sight, now running. Start one holding
a place and the new view appears in that same place.

## User experience

- **Entry** — the same row button as task 03, reading "Start" once the account
  is parked. One control, two meanings, decided by the row's state.
- **Flow** — press "Start". A new view is built against the same storage area
  and the game loads, already logged in. The row updates to say it is running.
- **States** — **starting**: from the press until the page first paints, the
  marker reads `Starting` with its own dot and the button is insensitive, so it
  cannot be pressed twice.
- **States** — **running again**: once the page paints, the marker reads as
  running plus the account's place, exactly as it did before the account was
  parked, and the button reads "Park" again.
- **Flow** — starting an out-of-sight account leaves it out of sight; starting
  one that holds a place puts the new view in that same place, and no other
  place changes.

## Technical details

- **Front-end** — the row's button label and its sensitivity come from row
  properties derived from the session's `Liveness`, so the factory binds them
  and never branches. `status_key` gains `"starting"`; `status_label` and
  `sidebar.css` gain the matching word and dot class (design rule 1).
- **Front-end** — the sidebar's press intent carries the account's id only.
  `window/imp.rs` reads that session's current liveness from the book to decide
  whether the press means park or start; the widget decides nothing
  (architecture rule 8).
- **Front-end** — starting is, in order: ask the book to unpark, which returns
  `Starting`; redraw so the row shows it and the button goes insensitive; ask
  the session's holder from task 02 to start; hand the new view to
  `session_grid` for that account's existing `SlotEntry`.
- **Front-end** — the starting interval ends on a load signal from the new view,
  which calls the book's third transition and redraws. Which signal counts as
  "first paint" is the roadmap item's fourth blocker: `LoadEvent::Committed` is
  what `session_grid` already uses to drop its loading cover, and whichever is
  chosen has to be checked against a real game rather than picked off the API
  list.
- **Front-end** — a new view needs the same wiring `session_grid::add_session`
  gives the first one: the loading cover and its `connect_load_changed`. Reuse
  that path rather than growing a second one beside it.
- **Code standards** — rule 18: comment why starting builds a new view instead
  of reviving the old one. `network-session` is construct-only (`webkit6` 0.6.1,
  `web_view.rs:141`), so the binding cannot be remade (`FR.5.4`).
- **Testing** — architecture rule 14 and code standards rule 25: `(manual)` for
  everything a screen shows; the extended `status_key` keeps its `(unit)` test
  at the foot of `row.rs`.

## Acceptance criteria

- [ ] `(unit)` the row's status-key derivation returns `starting` for a starting
      account whatever its visibility, and the button label derivation returns
      "Park" for a live account and "Start" for a parked one
- [ ] `(manual)` a parked account's row button reads `Start` and a running
      account's reads `Park`
- [ ] `(manual)` pressing `Start` loads the game already logged in — no login
      prompt appears, proving the account's data directory survived the park
      (`FR.2.3`)
- [ ] `(manual)` from the press until the page paints, the marker reads
      `Starting` with its own dot and the button is insensitive, so a second
      press does nothing
- [ ] `(manual)` once the page paints, the marker reads as running plus the
      account's place and the button reads `Park` again
- [ ] `(manual)` `ps` shows a new `WebKitWebProcess` for the account after
      starting, with resident memory comparable to what it held before parking
- [ ] `(manual)` starting an out-of-sight account leaves it out of sight;
      starting one holding a slot puts the new view in that same slot and leaves
      every other slot untouched

## References

- [Roadmap item](../../roadmap/03-parking-and-unparking/README.md) — the full
  picture, including the "Starting a parked account" diagram
- [Wireframe](../../roadmap/03-parking-and-unparking/wireframes/parked-account.md)
  — the inverted button label and the starting case
- [`docs/requirements.md`](../../requirements.md) — `FR.2.3`, `FR.5.4`, `FR.5.5`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 18, 24, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7
- [`docs/design.md`](../../design.md) — rule 1, extended with the starting state;
  the inverting row action is still the rule it owes

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
