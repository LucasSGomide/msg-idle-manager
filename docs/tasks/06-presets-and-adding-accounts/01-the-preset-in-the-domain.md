# 01 — The preset in the domain and the catalogue port

**Roadmap:** [06](../../roadmap/06-presets-and-adding-accounts/README.md) · **Scope:** back-end · **Depends on:** —

## Context

This program keeps its rules in a layer that touches nothing outside itself — no
screen, no disk, no network. That layer knows what a game account is: an
identity the program mints for it, the name the user gave it, the address it
starts at, whether it currently holds a place on screen, and whether it is
running. Creating one today means handing it a name and an address, both typed
by the user.

This slice teaches that layer what a *game* is, as opposed to an account on one.
A game is a small record of five things: the name to show in a list, the address
it starts at, an optional identity for the program to introduce itself with,
how large to draw the page, and whether that game needs to be kept running at
full speed while nobody is looking at it. Two accounts on the same game want all
five to be the same, which is exactly why they belong to the game rather than
being asked for once per account.

It also adds the way this layer asks for the list of games without knowing where
the list comes from. That is a description of a capability — "tell me the games
you know about" — which something outside answers later by reading files. It
hands back three things rather than one: the games it understood, the ones it
could not read together with what was wrong with each, and a plain description
of where it looked. Three, because a screen showing a partial list plus a line
about the one bad file is better than a screen showing nothing, and because the
screen has to be able to name the folder when the list is empty without knowing
that the entries were ever files.

Finally it adds a second way to create an account: from a game rather than from
a typed address. The user still supplies a name for the account, so two accounts
on the same game can be told apart; everything else is copied off the game. The
copy is what makes the account independent afterwards — editing the game later
changes nothing for an account that already exists.

## Technical details

- **Architecture** — a new `preset.rs` module in `idle-manager-core`, with no
  serde, no filesystem and no widget anywhere near it (rule 1); the capability
  goes in `ports.rs` beside the profile locator, described on the core side and
  implemented outside it (rules 5, 6).
- **Code standards** — `PresetId` is a newtype so it can never be passed where a
  `SessionId` belongs (rule 2). `ZoomLevel` is a named type with a fallible
  constructor that rejects a non-positive or absurd multiplier, so an invalid
  zoom cannot be constructed anywhere (rule 1). `Preset` holds the display name,
  the start address, an `Option<String>` browser identity, a `ZoomLevel` and the
  keep-awake default.
- **Back-end** — `PresetCatalogue` in `ports.rs`, named for the capability the
  domain needs rather than the technology behind it (naming rule 10). It returns
  the readable presets, separately the entries that failed with a per-entry
  reason, and a human-readable description of where the entries came from — the
  last so the shell can name the folder in its empty state without learning that
  entries are files (architecture rule 3).
- **Back-end** — the browser identity is `Option`-shaped and absent means *the
  engine's own*, never a string this crate supplies (`FR.10.5`). Item 01
  measured a Chrome claim breaking a real sign-in, so there is deliberately no
  fallback constant to reach for.
- **Back-end** — `Session` gains the browser identity and the zoom, copied at
  creation, so the shell reads them off the account rather than holding onto a
  preset.
- **Back-end** — `SessionBook::add_from_preset` mints the id and places the
  session exactly as `add` does — lowest free slot, or displacing the focused
  slot when the grid is full — then copies the preset's values onto it.
- **Back-end** — the keep-awake default is applied by calling `set_keep_awake`,
  never by writing the field, so there is one path that sets it. That transition
  moves a live account to `Starting` because it implies a page reload; a
  brand-new account has no page to reload and its first view is built carrying
  the setting already, so `add_from_preset` follows it with `mark_started` and
  the new account is `Live` like any other. Both are existing transitions; no
  field is written behind their backs.
- **Testing** — unit tests in a `#[cfg(test)] mod tests` at the foot of each
  file, one subject per test, named for the behaviour they pin (architecture
  rule 14, code standards rules 21, 23, 24).

## Acceptance criteria

- [x] `(unit)` a zoom level cannot be constructed from a zero or negative
      multiplier
- [x] `(unit)` a zoom level constructed from an accepted multiplier reports that
      multiplier back unchanged
- [x] `(unit)` an account created from a preset carries the account name the user
      typed, not the game's display name
- [x] `(unit)` an account created from a preset carries the preset's start
      address, zoom and browser identity
- [x] `(unit)` an account created from a preset that names no browser identity
      carries none, rather than a string the core supplies
- [x] `(unit)` an account created from a preset whose keep-awake default is on
      starts with keep-awake on
- [x] `(unit)` an account created from a preset is `Live` whichever way its
      keep-awake default is set
- [x] `(unit)` an account created from a preset is placed exactly as one created
      from a typed address — the lowest free slot, or the focused slot with its
      occupant displaced when the grid is full
- [x] `(unit)` an account created from a typed address carries no browser
      identity and the default zoom

## References

- [Roadmap item](../../roadmap/06-presets-and-adding-accounts/README.md) — the
  full picture, including the "Where a game's values end up" diagram that fixes
  which of the five fields is consumed once and which becomes a default
- [`docs/requirements.md`](../../requirements.md) — `FR.10.1`, `FR.10.2`,
  `FR.10.5`, `FR.6.1`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 5, 6, 8, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 2, 3, 4, 8, 12,
  17, 21, 22, 23, 24
- [`docs/naming.md`](../../naming.md) — rules 2, 6, 9, 10, 12

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
