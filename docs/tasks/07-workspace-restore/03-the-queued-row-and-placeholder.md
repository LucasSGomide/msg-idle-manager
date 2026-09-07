# 03 — The queued row and the queued slot

**Roadmap:** [07](../../roadmap/07-workspace-restore/README.md) · **Scope:** front-end · **Depends on:** 01

## Context

When the program reopens and brings a user's games back one at a time, most of
those games are, for a while, in a state the window has never had to draw: the
user wants them running, and their turn has not come. This slice teaches the two
places that draw an account's state to say so, before anything actually puts an
account into that state.

There are two such places. One is the row in the list down the side of the
window, which carries a small coloured dot and nothing else — the dot's colour
is the whole vocabulary, and a tooltip and a screen reader read the word behind
it. That vocabulary had four values and now has five, which is the moment it
stops being a list of colours anyone can add to and needs a stated ordering: an
account that is not simply running says so first, whatever slot it holds, and
among the not-simply-running states, waiting for a turn is a weaker claim than
being on its way up. The other place is the panel that stands in an empty slot
when the account that owns it has no game loaded. That panel currently always
offers a button, because until now every reason for a slot to be empty was a
reason the user could fix by pressing something. Waiting in a queue is not:
there is nothing useful to press while a queue is draining, so the panel learns
to show its line of text with no button at all.

Nothing in the program produces this state yet — the slice that restores a saved
arrangement is what first will. That is exactly why this work is separate. Both
places are decided by small functions with no widget in them, they are checked
here by tests that need no screen, and the slices that follow inherit a
vocabulary that already covers what they are about to produce rather than
growing one under pressure. What a queued row and a queued slot actually look
like on screen is confirmed by hand in the slices that can produce one.

## User experience

- **Flow** — an account that is waiting its turn to come back reads as queued;
  the one being started reads as starting, in item 03's existing wording rather
  than a second word for the same thing.
- **States** — **queued**: the row's trailing edge carries one dot in the queued
  colour and nothing else, its name dimmed the way any out-of-sight or
  not-running account's name is, and its Park/Start item reads "Start" and is
  insensitive, because the queue is the only thing entitled to start it.
- **States** — **a slot whose account is queued**: the placeholder panel with
  the queued line of text and no button, rather than a button that does nothing
  useful.
- **New pattern** — the row's state vocabulary now has five values and needs an
  ordering rather than an accumulation, which is a debt design rule 1 records
  against this item.

## Technical details

- **Design** — extend rule 1 in `docs/design.md` with the queued value and state
  the ordering the five values follow, so the sixth value item 08 adds has a
  rule to obey rather than a precedent to copy. Add the queued dot colour to
  `resources/css/sidebar.css` as one more `.status-dot.status-queued` class,
  distinct from the starting blue and the parked grey at a glance.
- **Front-end** — `status_key` in `session_sidebar/row.rs` grows one arm for
  `Liveness::Queued` and `status_label` the word behind it, keeping that pair
  the only place a domain state becomes a marker and its accessible name.
- **Front-end** — `action_label` returns "Start" for a queued account and
  `action_sensitive` returns false for it, for the same reason it already does
  while starting: a second press must not be able to build a second view, and
  during a restore the queue owns the order (design rule 2).
- **Front-end** — the name dimming already applies to anything not running or
  out of sight; confirm a queued account falls under it rather than adding a
  second rule (design rule 3).
- **Front-end** — `SlotPlaceholder` gains a `button-visible` property beside its
  existing `button-label` and `button-sensitive`, defaulting to visible so every
  present caller is unchanged, and `session_grid/imp.rs` derives the panel's
  line and button from the account's liveness during `sync` rather than only
  from the imperative `release_view` and `attach_view` calls — which is what
  lets a slot drawn straight from a restored book read correctly before any
  view exists (design rule 4, architecture rule 8).
- **Code standards** — every match over `Liveness` in this crate grows an arm
  rather than a catch-all, so the state item 08 adds cannot be silently absorbed
  by a wildcard here (rule 1).
- **Testing** — the derivations are pure functions with unit tests at the foot
  of their own file, as `status_key`'s existing tests are; no display server is
  required to run them (architecture rule 14, code standards rules 24, 25). The
  on-screen confirmation belongs to tasks 04 and 05, which are the first that
  can put an account into this state.

## Acceptance criteria

- [x] `(unit)` a queued account's status key is `queued` whether it holds a slot
      or is off-grid, and whether or not its slot is the current one
- [x] `(unit)` the queued key's label is the word a tooltip and a screen reader
      can read, not an empty string
- [x] `(unit)` a queued account's Park/Start item reads "Start"
- [x] `(unit)` a queued account's Park/Start item is insensitive, so nothing but
      the queue can start it
- [x] `(unit)` a queued account's name is dimmed under the existing rule, with no
      second rule added for it
- [x] `(unit)` the placeholder panel derived for a queued account carries the
      queued line and no button, while the one derived for a parked account is
      unchanged — same line, same "Start" button, still pressable
- [x] `(unit)` `sidebar.css` carries a `status-queued` dot class distinct from
      `status-starting` and `status-parked`, and the compiled resource bundle
      still loads

## References

- [Roadmap item](../../roadmap/07-workspace-restore/README.md) — the full
  picture; the `**New pattern**` bullets in its User Experience section are the
  two design debts, of which the row vocabulary is this slice's
- [The launch window wireframe](../../roadmap/07-workspace-restore/wireframes/launch-window.md)
  — the queued row and the queued slot as they must read; the whole item's
  screen, of which this slice draws only these two states
- [`docs/design.md`](../../design.md) — rules 1, 2, 3, 4, 6
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 16, 17, 21, 23,
  24, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 12

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
