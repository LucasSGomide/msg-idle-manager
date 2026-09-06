# 05 — The row's keep-awake indication

**Roadmap:** [04](../../roadmap/04-keep-awake/README.md) · **Scope:** front-end · **Depends on:** 04

## Context

By the time this slice starts, the setting works: an account can be told to keep
running at full speed while nobody is looking at it, and the browser engine and
the injected script both do what they are told. But there is only one place the
setting is visible, and that is inside the menu it was chosen from. To find out
whether an account is protected you have to open its menu, one account at a time.

That is the wrong shape for this particular setting. It is chosen once and then
forgotten for months, which is exactly why it needs to be readable at a glance:
the moment somebody wonders "is this game actually earning overnight?", the answer
should already be on the screen. If the only way to check is to open five menus,
the setting quietly becomes something nobody trusts.

So this slice puts a small permanent mark on the row of any account that has the
setting on. It sits at the trailing end of the row, next to the coloured dot and
the word that already say whether the account is on screen, running in the
background, or stopped. It is deliberately small and quiet: it is not a state that
changes, it is a fact about how the account is configured, and it must not compete
for attention with the dot beside it, which does change.

Putting it there means that end of the row is now carrying three separate things
at once — which place the account holds, what state it is in, and this. The
sidebar has a fixed, fairly narrow width, so three things sharing one edge is the
point at which the arrangement needs to become a written rule rather than a
happy accident. The account's name is the part that gives way: it shortens with an
ellipsis so nothing at the trailing end is ever pushed off.

The work itself is small. The row already computes what it should show from the
account's state, in one place, deliberately built so each new fact is one more arm
rather than a new branch scattered through the list. This adds one more arm, one
more style, and the two design rules the feature owes.

## User experience

- **States** — **on**: the row's trailing edge carries a small persistent
  indication that the setting is on, readable without opening the menu. **Off**:
  nothing is drawn there. **Reloading**: unchanged — the `Starting` word and blue
  dot from item 03, with the action button insensitive.
- **Flow** — turn the setting on and the indication appears once the page has
  painted; turn it off and it is gone on the next redraw.
- **New pattern** — an always-visible indication that a background setting is on.
  `docs/design.md` owes a rule for it, since the row's trailing edge now carries
  three things at once at the sidebar's fixed width.

## Technical details

- **Front-end** — extend the row's state derivation in
  `session_sidebar/row.rs`, added in item 02 and extended in item 03: a small
  pure function over the account's keep-awake flag returns the trailing
  indication, written into a `Row` property by `Row::refresh` beside the ones
  already there.
- **Front-end** — leave `status_key` alone. Liveness still wins over every other
  fact and the state marker keeps its five keys; keep-awake is a separate,
  independent mark, not a sixth state (design rule 1). Two facts that can both be
  true at once must not share one key.
- **Front-end** — the row factory in `session_sidebar/imp.rs` binds the
  indication into a widget between the status dot and the action button, cleared
  and re-applied on every bind like the dot's classes are, because the list
  recycles row widgets (code standards rule 18).
- **Design** — style the indication in `resources/css/sidebar.css` beside the
  status-dot rules, quieter than any of them: it marks a configuration that does
  not change, and the dot beside it marks a state that does.
- **Design** — add the rule the item names as owed: how three separate facts —
  the place, the state and a background setting — share one row's trailing edge
  without the row becoming unreadable at the sidebar's fixed width. The name is
  the only hexpanding child and already ellipsises with
  `pango::EllipsizeMode::End`, so it is what gives way; nothing at the trailing
  end is ever pushed off.
- **Testing** — the derivation is a pure function and gets its tests in the
  `#[cfg(test)] mod tests` at the foot of `row.rs` (code standards rule 24).
  Everything a screen shows stays `(manual)` under architecture rule 14 and code
  standards rule 25.

## Acceptance criteria

- [ ] `(unit)` the keep-awake indication derivation returns the indication only
      when the account's flag is on
- [ ] `(unit)` `status_key` returns the same key for an account whether or not
      its keep-awake flag is on — liveness and place still decide the state
      marker alone
- [ ] `(manual)` an account with keep-awake on shows the indication on its row's
      trailing edge without the menu being opened, and an account with it off
      shows nothing there
- [ ] `(manual)` turning the setting on shows the indication once the page has
      painted, and turning it off removes it on the next redraw
- [ ] `(manual)` the indication renders correctly beside every state the row
      already draws — `Current`, `Visible`, `Background`, `Parked` and
      `Starting` — and reads distinctly from the status dot next to it
- [ ] `(manual)` at the sidebar's fixed width a long account name is ellipsised
      rather than pushing the state marker, the indication, the action button or
      the menu button off the row

## References

- [Roadmap item](../../roadmap/04-keep-awake/README.md) — the full picture,
  including the `**New pattern**` bullets naming the two design rules this item
  owes
- [Wireframes](../../roadmap/04-keep-awake/wireframes/) — the row settings menu
  and the trailing edge this slice adds to
- [`docs/requirements.md`](../../requirements.md) — `FR.6.1`
- [`docs/architecture.md`](../../architecture.md) — rules 10, 12, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 18, 21, 22, 23, 24,
  25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 12
- [`docs/design.md`](../../design.md) — rules 1 and 3, which this slice must stay
  true to; the trailing-edge rule is the one it adds

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
