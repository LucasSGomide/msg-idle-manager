# 02 — Session and layout domain

**Roadmap:** [01](../../roadmap/01-isolated-accounts-and-layouts/README.md) · **Scope:** back-end · **Depends on:** —

## Context

This slice writes the rules of the application down in one place, as plain Rust
with no window, no web page and no files on disk. It is the part that decides
things; everything else obeys it.

There are two ideas. One is the account: a name someone typed, the address the
game starts at, an identifier the program mints for it, and where it currently
is — either occupying one of the visible places on screen, or out of sight. The
other is the arrangement: the window shows either one account filling it, two
side by side, or four in a two-by-two grid, and something has to decide which
account goes where when that choice changes or when an account is added.

That deciding is a single function, and it is the heart of the whole item. You
hand it the arrangement, the accounts in the order they were added, which place
is currently focused, and the place each account last held. It hands back where
every account goes now: a numbered place, or out of sight. All the behaviour
people notice lives in it. A new account takes the first free place. If there is
no free place it takes the focused one and pushes that account out of sight.
Choosing an arrangement with fewer places pushes the accounts that no longer fit
out of sight rather than closing them. Choosing one with more places brings them
back to where they were, or to the first free place if someone else has since
taken it.

Because the function is given everything it needs and reads no clock, no random
number and no file, the same inputs always produce the same answer, and every
one of those behaviours can be pinned by a test that runs in milliseconds on a
machine with no screen attached. That is precisely why this is its own slice: it
is the only part of the item that can be proven by an automated test at all.

## Technical details

- **Architecture** — rule 1: `crates/idle-manager-core` takes no dependency on
  GTK, WebKit, serde or the filesystem; `make arch-check` enforces it.
- **Architecture** — rule 9: the placement function is synchronous and pure —
  no clock, no randomness, no interior state; everything it needs is an
  argument.
- **Architecture** — the folder layout puts identity in `src/session.rs` and
  arrangement in `src/layout.rs`, both re-exported from `src/lib.rs`.
- **Code standards** — rule 1: `Visibility` is an enum of `InSlot(SlotId)` and
  `OffGrid`, never a boolean beside an optional slot. `Liveness` is deliberately
  absent — parking is a later roadmap item, and a second state field now would
  mean two fields describing three states.
- **Code standards** — rule 2: `SessionId` and `SlotId` are newtypes, so an
  identifier of one kind can never be passed where the other belongs.
- **Code standards** — rules 21 to 24: unit tests live in a `#[cfg(test)] mod
  tests` at the foot of each file, named after the behaviour they pin, one
  subject per test, arrange/act/assert separated by blank lines.
- **Naming** — rules 6, 8, 9 and 11: no `session::Session` stutter, Rust's own
  casing, types named after the thing rather than the pattern.

## Acceptance criteria

- [x] `(unit)` adding a session to the book returns an identifier distinct from
      every identifier already in it
- [x] `(unit)` a session added while a slot is free takes the lowest-numbered
      free slot of the current layout
- [x] `(unit)` a session added while every slot is occupied takes the focused
      slot, and that slot's previous occupant becomes off-grid
- [x] `(unit)` switching to a layout with fewer slots leaves every session whose
      slot no longer exists off-grid and every other session where it was
- [x] `(unit)` switching to a layout with more slots returns each off-grid
      session to the slot it last held
- [x] `(unit)` an off-grid session whose remembered slot is occupied when the
      layout grows takes the lowest-numbered free slot instead
- [x] `(unit)` placing the same layout, sessions and remembered slots twice
      returns the same placement
- [x] `(unit)` a placement pass leaves every session's display name and start
      address unchanged

## References

- [Roadmap item](../../roadmap/01-isolated-accounts-and-layouts/README.md) — the full picture, including the "Changing the arrangement" diagram this function serves
- [`docs/architecture.md`](../../architecture.md) — rules 1, 9 and 14, and the core crate's folder layout
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 2 and 21 to 24
- [`docs/naming.md`](../../naming.md) — rules 6, 8, 9 and 11

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
