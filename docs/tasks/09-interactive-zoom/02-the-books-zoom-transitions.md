# 02 — The book's zoom transitions and the focused account

**Roadmap:** [09](../../roadmap/09-interactive-zoom/README.md) · **Scope:** back-end · **Depends on:** 01

## Context

The previous slice taught an account what size to draw at: a number the game's
file asked for, plus at most one chosen size per arrangement, with a single rule
deciding which applies. It left no way to choose one. This slice adds that, as
three operations on the object that holds every account and knows how the window
is currently divided.

Stepping in makes one account's size one step bigger; stepping out makes it one
step smaller; resetting throws away whatever was chosen and goes back to what the
game's file asked for. Each of them names an account and nothing else. None takes
the arrangement as an argument, and that is deliberate: the object already knows
which arrangement is showing, so recording a size against the wrong one is not a
mistake a caller can make. Every one of them hands back the size that now
applies, so whatever asked for the change can immediately draw the page at it
without asking a second question.

Naming an account that is not there is not a failure — it gives back nothing, and
the caller carries on. That matches how parking and the keep-awake switch already
behave, so the window never has to hold a special case for one of them.

What none of these do is touch whether an account is running. Resizing a page is
not restarting it, and the whole reason this application exists is that these
games lose progress when interrupted. An account that was live before a step is
live during it and live after it, and nothing about which slot it sits in moves
either.

The slice also adds one lookup: which account is in the place the window is
currently focused on. The keyboard gestures in a later slice need exactly that
account, and there is no way to ask for it today. Putting the question here keeps
"which account is active" as one answer in one place rather than a rule the
window re-derives, and it is the same question the sidebar's current-row marking
can later share.

## Technical details

- **Back-end** — `SessionBook` gains `zoom_in` and `zoom_out` in the style `park`
  and `set_keep_awake` already set: each steps the named account's size for the
  book's **current** layout, records it, and returns the new `ZoomLevel`.
- **Back-end** — `reset_zoom` removes the current layout's entry and returns the
  baseline the game file supplied.
- **Back-end** — none of the three takes a layout argument; the book already
  holds its own at `crates/idle-manager-core/src/session.rs:172`, which is what
  makes "records for the current arrangement only" impossible to get wrong at a
  call site.
- **Back-end** — all three return `Option<ZoomLevel>`, so an id not in the book
  is a `None` the caller skips on, matching the no-op-on-missing-id style of the
  existing transitions.
- **Back-end** — none of them touches liveness or visibility: a zoom is not a
  reload, so nothing moves to `Starting` (`FR.11.5`).
- **Back-end** — `focused_session` returns the account whose visibility is the
  focused slot, or `None`, so the shell has one place to ask which account a
  keyboard gesture acts on.
- **Architecture** — rule 8: the shell sends what the user did and the book
  decides what a step means, so the keyboard and the wheel can never disagree
  about the size or about which arrangement it was recorded for. Rules 1 and 9
  keep it pure and synchronous.
- **Testing** — unit tests at the foot of `session.rs`, one subject each (code
  standards rules 21, 23, 24).

## Acceptance criteria

- [ ] `(unit)` `zoom_in` returns the stepped size and records it against the
      book's current layout
- [ ] `(unit)` `zoom_out` returns the stepped-down size and records it the same
      way
- [ ] `(unit)` a step records nothing for the two layouts that are not current
- [ ] `(unit)` a step at the range's edge returns the clamped size and records
      that clamped size
- [ ] `(unit)` `reset_zoom` drops the current layout's entry, leaves the other
      layouts' entries in place, and returns the baseline
- [ ] `(unit)` all three return `None` for an id the book does not hold and
      change nothing about any account
- [ ] `(unit)` a step leaves the account's liveness and visibility exactly as
      they were
- [ ] `(unit)` switching the book's layout changes what `zoom_for` answers for an
      account without changing any size it has stored
- [ ] `(unit)` `focused_session` returns the account sitting in the focused slot,
      and `None` when that slot holds nothing

## References

- [Roadmap item](../../roadmap/09-interactive-zoom/README.md) — the full picture,
  including the "Zooming the account in the active place" diagram whose domain
  half this slice implements
- [`docs/requirements.md`](../../requirements.md) — `FR.11.1`, `FR.11.2`,
  `FR.11.4`, `FR.11.5`, `FR.12.1`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 8, 9
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 5, 7, 17, 21,
  23, 24
- [`docs/naming.md`](../../naming.md) — rules 6, 8, 11

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
