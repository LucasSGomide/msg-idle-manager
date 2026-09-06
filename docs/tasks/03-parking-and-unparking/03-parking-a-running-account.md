# 03 — Parking a running account

**Roadmap:** [03](../../roadmap/03-parking-and-unparking/README.md) · **Scope:** front-end · **Depends on:** 01, 02

## Context

The application lists every game account down the leading edge of the window,
one row each, showing the account's name and where it sits. This slice puts a
button on that row and makes it do the thing the whole feature exists for: shut
the account down and give its memory back to the system.

Press "Park" on a running account and three things happen in order. The pure
logic layer records that the account is now parked — and, importantly, does not
move it, because an account keeps its place on screen whether or not it is
running. Then the shell tells the browser engine to end that account's rendering
process and throws the view away, in that order, because doing it the other way
round leaves the engine to decide when the process dies. Then the row redraws:
its state marker reads "Parked" and its name is drawn in the dimmed style.

If the account was out of sight, nothing else on screen changes. If it was
holding a place on screen it keeps that place, and the place now shows the plain
cover of the account's name on the window's background that the program already
draws while a game is loading. A later slice replaces that with a proper panel
carrying its own "Start" button.

There is no way back yet. Starting a parked account again is the next slice;
this one stops at the stop, which is the half that has to be proved with a
measurement rather than a screenshot. Accepting it means watching the account's
rendering process disappear from the process list and the memory it held return
to the system. That measurement is also the answer to an open question the
roadmap item records: whether the engine's lowest cache setting really keeps no
cache of ended processes, or quietly holds the memory anyway. If the memory does
not come back, the mechanism is wrong and the item needs a different one.

## User experience

- **Entry** — a button on each sidebar row's trailing edge, after the state
  marker, reading "Park" for a running account. It is the row's only action, so
  it sits on the row itself rather than behind a menu — item 04's settings go
  behind one precisely because they are not this.
- **Flow** — press "Park". The account's rendering process ends, its view is
  thrown away, and its row updates to say it is parked.
- **Flow** — park an account that is out of sight and nothing on screen changes
  except its row. Park one holding a place and only that place changes; the
  other games keep running untouched.
- **Flow** — a parked account moves between places, and out of sight and back,
  exactly as a running one. Being parked does not move it.
- **States** — **parked**: the state marker reads `Parked` with its own dot, and
  the name is drawn in the dimmed style whether or not the account holds a
  place, alongside the place it holds. **Parked in a place**: that place shows
  the account's name centred on the window's background — the loading cover from
  item 01, standing in until task 05 replaces it.
- **New pattern** — a two-state action control on a list row, whose label and
  meaning invert with the row's state. `docs/design.md` owes a rule for it, and
  another for what "dimmed" means now that two states use it: out of sight, from
  item 02, and parked.

## Technical details

- **Front-end** — extend `session_sidebar/row.rs`: `status_key` takes the
  session's `Liveness` beside its `Visibility` and returns `"parked"` for a
  parked account whatever its place; `status_label` gains the word; `name_markup`
  dims a parked name. Design rule 1 — one key drives both the word and the dot's
  class, so this is one arm of one match, never a fourth branch in the factory.
- **Design** — add a `status-parked` dot class to `resources/css/sidebar.css`
  and the key to `STATUS_CLASSES` in `session_sidebar/imp.rs`. Design rule 1
  lists each state with its dot; append the parked state to it, and the rule the
  item's `## User Experience` says is owed for the two meanings of dimming.
- **Front-end** — the row factory grows a trailing `gtk::Button`. Its label
  comes from a row property, so task 04 inverts it to "Start" without reopening
  the factory. The list recycles row widgets, so the button's handler is
  re-bound on every bind, exactly as the dot's classes are cleared and
  re-applied.
- **Front-end** — `SessionSidebar` exposes the press as an intent carrying the
  account's id, through a `connect_*` handler beside `connect_row_activated`.
  The sidebar decides nothing itself (architecture rule 8).
- **Front-end** — `window/imp.rs` handles it in the order the item's first
  diagram fixes: ask the book to park the session, then tell that session's
  holder from task 02 to stop, then redraw from the new state. Domain first,
  engine second (architecture rule 8).
- **Front-end** — `session_grid` drops the parked session's view out of its
  overlay and leaves the name cover visible underneath. The `SlotEntry` keeps
  its `placement`, so the slot stays the account's.
- **Testing** — architecture rule 14 and code standards rule 25 keep GTK out of
  `cargo test`, so everything a screen shows is `(manual)`. `status_key` is a
  pure function over two enums and gets a `#[cfg(test)] mod tests` at the foot
  of `row.rs` (code standards rule 24).
- **Front-end** — the roadmap item's first blocker is settled on this slice's
  runbook: record the account's `WebKitWebProcess` resident memory with `ps`
  before and after the park. If the process survives, or the memory does not
  return to the system, `FR.5.3`'s mechanism is wrong and the item grows a
  slice.

## Acceptance criteria

- [x] `(unit)` the row's status-key derivation returns `parked` for a parked
      account whatever its visibility, and still returns `current`, `visible`
      and `background` unchanged for a live one
- [x] `(manual)` every running account's row shows a `Park` button on its
      trailing edge, after the state marker
- [x] `(manual)` pressing `Park` leaves the row reading `Parked` with its own
      dot and the name drawn in the dimmed style (design rule 1)
- [x] `(manual)` pressing `Park` ends the account's rendering process: its
      `WebKitWebProcess` is gone from `ps` and the resident memory it held,
      recorded before and after, is returned to the system
- [x] `(manual)` parking an account that holds a slot leaves every other slot's
      game running and untouched, and the parked slot shows the account's name
      centred on the window background
- [x] `(manual)` a parked account keeps its place: its row still names the slot
      beside `Parked`, and switching layouts moves it exactly as a running
      account
- [x] `(manual)` parking an out-of-sight account changes nothing on screen
      except its own row

## Note on the "names the slot beside `Parked`" wording

The sixth criterion says a parked row "still names the slot beside `Parked`".
That phrasing predates roadmap 02's `## As built`, which settled the row marker
as a *single* status key → one word + one dot, never a word plus a slot number
(the `docs/design.md` rule 1 the item wrote). Task 03's own Technical details
follow that: `status_key` returns `"parked"` whatever the place, and
`status_label` "gains the word". So a parked row reads `Parked`, not
`0 · Parked`. The criterion is met in substance — parking never touches
`Visibility`, the account keeps its slot, and switching layouts moves it
exactly as a running account (verified in `test-script.md`). If a parked row
should also surface its slot, that is a new design rule and a follow-up, not a
change to this slice.

## References

- [Roadmap item](../../roadmap/03-parking-and-unparking/README.md) — the full
  picture, including the "Parking a running account" diagram
- [Wireframe](../../roadmap/03-parking-and-unparking/wireframes/parked-account.md)
  — the row action, the parked marker and the slot it leaves behind
- [`docs/requirements.md`](../../requirements.md) — `FR.5.1`, `FR.5.2`, `FR.5.3`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 18, 21, 24, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7
- [`docs/design.md`](../../design.md) — rule 1, extended with the parked state;
  the inverting row action and the second meaning of dimming are the rules it
  still owes

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
