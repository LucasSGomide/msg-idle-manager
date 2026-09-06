# 01 — Keep-awake in the session book

**Roadmap:** [04](../../roadmap/04-keep-awake/README.md) · **Scope:** back-end · **Depends on:** —

## Context

The program runs several game accounts at once, each one a browser page of its
own. Browsers deliberately slow down a page nobody is looking at: timers are
stretched, animations stop, and the rapid callback that drives smooth motion is
never handed out. For most pages that is invisible and welcome. For an idle game
that counts time on the player's own machine it is the difference between a
night of progress and nothing, and the game says nothing about it — the player
finds out in the morning.

Not every game is at risk. Plenty keep their real state on their own servers and
work out what you earned from the clock when you come back. Forcing those to run
at full speed in the background would spend memory and processor time protecting
something that was never in danger. So the choice has to be made per account
rather than once for the whole program.

This slice adds that choice to the pure logic layer — the part of the program
that knows what an account is and nothing about windows, browsers or files. Each
account gains one yes-or-no record: must this one keep running at full speed even
when the program believes nobody is looking at it? A new account starts with it
off, and a later piece of work will supply a sensible starting value from a
catalogue of known games.

The move that sets it reports back whether it actually changed anything. That
matters because the program's reaction to a change is expensive — the account's
page has to be reloaded, which is the only way the countermeasure can be put in
place — and reloading a game for a setting that was already correct throws away
whatever the page was doing. Deciding that here, once, rather than inside a menu
handler, means every caller gets it right.

A change that lands on a running account also puts it into the same "on its way
up, no page has painted yet" state the program already uses when an account is
started from cold, because a reload is exactly that from the user's point of
view. An account that is currently stopped has no page to reload, so its record
changes and its state does not.

The rest of the slice is tests pinning what the record must survive: moving an
account between places on screen, pushing it out of sight and bringing it back,
stopping it and starting it again. Those are the whole reason it lives in the
logic layer instead of on a widget.

## Technical details

- **Back-end** — add `is_kept_awake: bool` to `Session` in
  `crates/idle-manager-core/src/session.rs`, `false` for every account
  `SessionBook::add` mints, with an `is_kept_awake()` accessor beside
  `liveness()`. Re-export nothing new: the flag is a field on an already-exported
  type.
- **Naming** — rule 12 gives a boolean the shape of a question, so the field and
  the accessor are both `is_kept_awake`. The menu's wording is the user's
  ("keep running when hidden"); this is the field's.
- **Code standards** — rule 1 is satisfied by a boolean here rather than
  violated: there are exactly two states and no third is ever possible, so an
  enum would add a name without adding a case. Contrast `Liveness`, which has
  three.
- **Back-end** — `SessionBook::set_keep_awake(&SessionId, bool) -> bool` sets the
  flag and returns whether it actually changed. The shell's response to a change
  is a page reload, and doing that for a set that set nothing is a bug the domain
  prevents rather than the widget (architecture rule 8). It is the only way in:
  item 06's game catalogue sets the flag at creation through this same
  transition rather than reaching into the field, so the field stays private and
  every path through it reports what it changed.
- **Back-end** — a change that lands on a `Liveness::Live` session also moves it
  to `Liveness::Starting`, because the shell reloads its page and `Starting`
  already means "no page has painted yet"; the existing `mark_started`
  transition ends the interval at the first paint. A `Parked` or `Starting`
  session keeps its liveness — a parked account has no page to reload. Widen
  `Liveness::Starting`'s doc comment from "unparked, but no page has painted yet"
  to the interval it now also covers.
- **Back-end** — `set_layout`, `focus_session`, `park` and `unpark` never touch
  the flag, and `set_keep_awake` never touches `visibility` or the `remembered`
  slot map. The flag belongs to the account, not to the slot and not to the
  process.
- **Architecture** — rules 1 and 9: `idle-manager-core` stays free of GTK,
  WebKit, serde and the filesystem, and stays synchronous and clock-free. Rule 6
  — no new port, because the domain needs nothing here it must not know how to
  do. Nothing is written to disk and nothing is read from `/proc`, so
  `idle-manager-store` and `idle-manager-metrics` are untouched and
  `make arch-check` needs no new forbidden edge.
- **Testing** — architecture rule 14 and code standards rules 21–24: unit tests
  in the `#[cfg(test)] mod tests` at the foot of `session.rs`, named for the
  behaviour they pin, arrange/act/assert split by blank lines, one subject each.

## Acceptance criteria

- [ ] `(unit)` an account added to the book starts with keep-awake off
- [ ] `(unit)` turning keep-awake on for an account that had it off returns
      `true` and leaves the flag on
- [ ] `(unit)` setting keep-awake to the value it already holds returns `false`
      and leaves the account's liveness exactly as it was
- [ ] `(unit)` turning keep-awake on for a live account leaves it `Starting`
- [ ] `(unit)` turning keep-awake on for a parked account leaves it `Parked`
- [ ] `(unit)` an account's keep-awake flag survives a layout change that moves
      it between slots
- [ ] `(unit)` an account's keep-awake flag survives being pushed off-grid and
      brought back into the focused slot
- [ ] `(unit)` an account's keep-awake flag survives being parked
- [ ] `(unit)` an account's keep-awake flag survives being unparked
- [ ] `(unit)` setting keep-awake on an id that is not in the book returns
      `false` and leaves the book untouched

## References

- [Roadmap item](../../roadmap/04-keep-awake/README.md) — the full picture,
  including the "Turning keep-awake on for an account" interaction diagram
- [`docs/requirements.md`](../../requirements.md) — `FR.6.1`, the per-session
  flag this slice's type has to carry
- [`docs/architecture.md`](../../architecture.md) — rules 1, 6, 8, 9, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 17, 21, 22, 23,
  24
- [`docs/naming.md`](../../naming.md) — rules 9, 12

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
