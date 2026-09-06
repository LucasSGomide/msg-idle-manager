# 01 — Liveness in the session book

**Roadmap:** [03](../../roadmap/03-parking-and-unparking/README.md) · **Scope:** back-end · **Depends on:** —

## Context

The application holds several game accounts at once. Each one is a running
browser view with its own login, and each one costs memory whether or not
anyone is looking at it. Users need a way to shut an account down without losing
it: its login, its name and its place on screen all stay, but the part that
costs memory stops running. The word for that here is "parking", and starting it
again is "unparking".

This slice adds that idea to the pure logic layer of the program — the crate
that knows what an account is and nothing about windows, browsers or files.
Today an account records only where it sits: in one of the numbered places on
screen, or out of sight. This slice gives it a second, separate record of
whether it is running.

The two records are deliberately independent. All four combinations are legal
and each one means something a user might want: running and on screen, running
and out of sight (a game earning progress in the background), stopped and out of
sight (the memory saving this whole feature exists for), and stopped while still
holding its place on screen — keep the game where you put it, come back in an
hour, pay nothing for it meanwhile. A single combined state would make one of
those four impossible to express, and the missing case becomes a bug nobody
wrote.

There is a third value beside running and stopped: starting. Starting a stopped
account is not instant — a whole page has to load — and during that window the
control that started it must not be pressable again. Recording "starting" here,
in the logic layer, rather than as a flag on a button, is what makes the double
press impossible everywhere at once instead of in one handler.

Nothing in this slice draws anything or touches a browser. It adds the states,
the moves between them, and the tests that pin down what each move may not
disturb — above all, that starting and stopping an account never moves it on
screen.

## Technical details

- **Back-end** — add `Liveness` to `crates/idle-manager-core/src/session.rs`: an
  enum of `Live`, `Parked` and `Starting`, held as a field on `Session` beside
  `Visibility`, with a `liveness()` accessor beside `visibility()`. A newly
  added account is `Live`. Re-export it from the crate's `lib.rs` beside
  `Visibility`.
- **Code standards** — rule 1: two enums, never one combined state. `FR.5.1`
  says liveness and visibility are independent, so all four combinations are
  legal and a type that cannot express one of them is wrong. The rule's own
  example spells `enum Liveness { Live, Parked }`; `Starting` is the third value
  this item adds.
- **Naming** — rule 9: `Liveness`, named for the thing. Never
  `SessionLivenessState`, never `LivenessManager`.
- **Back-end** — three transitions on `SessionBook`, each taking a `&SessionId`
  and returning the session's new `Liveness`: park it, unpark it — which yields
  `Starting`, not `Live`, because no page has painted yet — and the one that
  ends the starting interval once the shell reports the first paint. None of the
  three touches `visibility` or the `remembered` slot map.
- **Back-end** — parking a parked session and unparking a live one are no-ops
  returning the state unchanged, not errors: a repeated click is not a failure.
  A `SessionId` that is not in the book leaves the book untouched.
- **Back-end** — back-end here is `idle-manager-core` only. Nothing in this item
  is written to disk and nothing is read from `/proc`, so
  `idle-manager-store` and `idle-manager-metrics` are untouched and
  `make arch-check` needs no new forbidden edge.
- **Architecture** — rule 9 keeps this synchronous and clock-free. There is no
  timing in parking; the retry timing that does exist belongs to item 08. Rule 6
  — no new port, because the domain needs nothing here it must not know how to
  do.
- **Testing** — architecture rule 14 and code standards rules 21–24: unit tests
  in the `#[cfg(test)] mod tests` at the foot of `session.rs`, named for the
  behaviour they pin, arrange/act/assert split by blank lines, one subject each.

## Acceptance criteria

- [x] `(unit)` a session added to the book starts `Live`
- [x] `(unit)` parking a live session returns `Parked`
- [x] `(unit)` parking a session leaves its visibility exactly as it was
- [x] `(unit)` unparking a parked session returns `Starting`, not `Live`
- [x] `(unit)` unparking a session leaves its visibility exactly as it was
- [x] `(unit)` ending the starting interval returns `Live`
- [x] `(unit)` parking an already-parked session returns `Parked` and changes
      nothing else about it
- [x] `(unit)` unparking a live session returns `Live` and changes nothing else
      about it

## References

- [Roadmap item](../../roadmap/03-parking-and-unparking/README.md) — the full
  picture, including the two interaction diagrams
- [`docs/requirements.md`](../../requirements.md) — `FR.5.1`, the independence
  this slice's types have to express
- [`docs/architecture.md`](../../architecture.md) — rules 6, 9, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 21, 22, 23, 24
- [`docs/naming.md`](../../naming.md) — rule 9

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
