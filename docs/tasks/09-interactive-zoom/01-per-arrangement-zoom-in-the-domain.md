# 01 — Per-arrangement zoom in the domain

**Roadmap:** [09](../../roadmap/09-interactive-zoom/README.md) · **Scope:** back-end · **Depends on:** —

## Context

Every account in this application draws a game page at some size, and today that
size is decided once. When an account is created it copies a number out of the
game's own small text file — the multiplier the page is drawn at, where one means
the engine's normal size — and keeps it for as long as the account exists.
Nothing can change it afterwards, and one number has to serve whether the game is
filling the whole window or sharing it with three others.

This slice changes the shape of that number in the rules layer, and nothing else.
Two things come out of it. The first is arithmetic: a way to ask a size for the
size one step larger, or one step smaller, where a step is about ten percent. The
answer is always a usable size — asking for a step larger when a page is already
at the largest size the application allows gives back that largest size rather
than a failure, because a person pressing a key at the limit should have the
gesture received rather than dropped. That is different from a number arriving
from a file, which still has to be rejected outright if it makes no sense.

The second is the rule for where a size comes from. An account stops holding one
number and starts holding two things: the number the game's file asked for, which
never changes, and a small set of sizes the person has actually chosen — at most
one for each of the three ways the window can be divided. Asking an account what
size to draw it at now takes the arrangement as part of the question. If the
person has chosen a size for that arrangement, that is the answer; otherwise it
is the game file's number. This is written once, in one place, so no later code
can get the fallback wrong.

Nothing writes a chosen size yet — the gestures that do that are the next slice,
and the file that remembers them across a restart is the slice after. What this
one adds is the ability to install a whole set of chosen sizes onto an account at
once, which is what the code reading that file will eventually call, and which is
also how this slice's own tests put a chosen size in place to check the rule.

## Technical details

- **Back-end** — `preset.rs` gains `ZoomLevel::stepped_in` and `stepped_out` on
  the existing newtype, each returning a `ZoomLevel` one step away and clamped
  into the existing `MIN` and `MAX` at `crates/idle-manager-core/src/preset.rs:50`
  rather than failing. Clamping rather than erroring is what makes a gesture at
  the limit visibly received.
- **Code standards** — the step factor is a named constant with its unit stated,
  a ratio of 1.1, never a literal at a call site (rule 5). `ZoomLevel::new` keeps
  its fallible shape: a value arriving from a file still has to be rejected.
- **Back-end** — `layout.rs`'s `Layout` at
  `crates/idle-manager-core/src/layout.rs:10` gains `Hash` alongside its existing
  derives so it can key a map. That is the only change that file needs.
- **Back-end** — `session.rs`: `Session`'s single `zoom` field splits into a
  baseline copied from the preset at creation, read through a renamed
  `preset_zoom` accessor, and a map from `Layout` to `ZoomLevel` holding only the
  arrangements whose size has been changed. `zoom_for(Layout)` returns the map's
  entry when there is one and the baseline otherwise — the whole resolution rule,
  written once.
- **Back-end** — the remembered map is a small core-side named type wrapping
  `Layout` to `ZoomLevel`, not a bare `HashMap`, so the port added in task 03 can
  say what it carries (naming rules 6, 9). `SessionBook::restore_zoom` takes one
  and installs it on the named account, for the one caller that has just read one
  off disk; an unknown id changes nothing.
- **Architecture** — rule 1 keeps this in the core with no serde, no filesystem
  and no widget; rule 9 keeps it synchronous and pure. `Session`'s workspace
  round trip still carries the baseline, so a saved arrangement written before
  this slice reads back unchanged.
- **Front-end** — the two existing `Session::zoom` callers in
  `crates/idle-manager-shell/src/window/imp.rs:291` and `:419` move to
  `zoom_for` the book's current layout, so no caller can read a baseline where a
  resolved size belongs (code standards rule 1 applied to accessors). Behaviour
  is unchanged while nothing is remembered.
- **Testing** — unit tests in a `#[cfg(test)] mod tests` at the foot of
  `preset.rs` and `session.rs` (code standards rules 21, 23, 24), each named for
  the behaviour it pins.

## Acceptance criteria

- [x] `(unit)` `stepped_in` returns a level the step factor larger and
      `stepped_out` one the same factor smaller
- [x] `(unit)` a step in from `MAX` returns `MAX` and a step out from `MIN`
      returns `MIN`, neither of them erroring
- [x] `(unit)` `ZoomLevel::new` still rejects a multiplier outside
      `MIN..=MAX` and a non-finite one
- [x] `(unit)` `zoom_for` returns the baseline for a layout with no remembered
      size
- [x] `(unit)` `zoom_for` returns the remembered size for the one layout that has
      one and the baseline for the other two
- [x] `(unit)` `preset_zoom` still answers the game file's value after a
      remembered size has been installed over it
- [x] `(unit)` `restore_zoom` installs a whole remembered map onto the named
      account, and changes nothing for an id the book does not hold
- [x] `(unit)` the workspace an account produces still carries its baseline zoom,
      so the split changes nothing a saved arrangement holds

## References

- [Roadmap item](../../roadmap/09-interactive-zoom/README.md) — the full picture,
  including the "Where an account's size comes from" diagram this slice
  implements
- [Item 06's third blocker](../../roadmap/06-presets-and-adding-accounts/README.md)
  — it settled that a preset's zoom is a plain multiplier and deferred making
  zoom follow the slot; this slice answers that the other way round, so the
  baseline stays the game file's number and is never computed from a slot size
- [`docs/requirements.md`](../../requirements.md) — `FR.11.4`, `FR.12.1`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 9
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 2, 5, 17, 21,
  23, 24
- [`docs/naming.md`](../../naming.md) — rules 2, 6, 8, 9, 11

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
