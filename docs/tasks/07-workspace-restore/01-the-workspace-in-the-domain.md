# 01 — The workspace in the domain, and coming back from one

**Roadmap:** [07](../../roadmap/07-workspace-restore/README.md) · **Scope:** back-end · **Depends on:** —

## Context

This program keeps its rules in a layer that touches nothing outside itself — no
screen, no disk, no network. That layer already knows what a game account is and
how the accounts are arranged: which of three screen arrangements is in use,
which account sits in which visible slot, and whether each account's game is
running or has been stopped to save memory.

Today that knowledge lasts exactly as long as the program is open. This slice
teaches the rules layer to describe its whole arrangement as one value that can
be handed out and handed back: the accounts in order, each with the name the
user gave it, the address its game starts at, how large to draw it, the identity
to present to the game, whether it was running or stopped, and where it sat —
plus which arrangement was in use. Nothing here writes anything anywhere.
Something outside does that later, which is why this slice also describes the
capability it will need — "save this arrangement" and "give me back the one you
saved" — without knowing that a file is involved at all.

Handing the value back is the interesting half. An account that was running when
the program closed must not come back as running, because nothing is running
yet: its game has not been loaded. It comes back in a new fourth state, queued,
meaning the user wants it up and its turn has not come. Accounts that were
stopped come back stopped and stay that way, costing nothing at start-up. The
rules layer also says in what order the queued accounts should be brought up,
because which accounts the user wants back is a decision about their wishes,
while waiting for each one to settle is a matter of timing and belongs to
whoever owns a clock.

Two details a restart makes visible. Every account carries an identity this
layer mints for it, so after coming back it has to keep counting from above the
highest identity it just read, or the next account added would collide with one
restored. And a saved arrangement can disagree with itself — an account recorded
in the fourth slot of an arrangement that shows only one — so a restored account
whose slot cannot be shown goes out of sight while remembering that slot. That
is exactly what already happens when a user switches to a smaller arrangement,
so it is the existing rule applied to a new caller, not a new rule.

## Technical details

- **Architecture** — a new `workspace.rs` in `idle-manager-core` for the value,
  and the `WorkspaceStore` trait in `ports.rs` beside `ProfileLocator`: declared
  core-side, implemented outside it (rules 1, 5, 6). No serde, no path building,
  no clock and no waiting anywhere in this slice (rules 1, 9) — the port's read
  and write are plain synchronous calls returning a `thiserror` enum the adapter
  defines.
- **Naming** — `WorkspaceStore` names the capability, not the technology behind
  it and never with "Manager" in it (rules 9, 10); task 02's adapter is the one
  that says TOML. `Workspace` and its entry type live in `workspace.rs` without
  repeating the module name (rule 6).
- **Back-end** — `Workspace` holds the accounts in the order they were added
  plus the active `Layout`. One entry carries the identifier, display name,
  start address, liveness, visibility, remembered slot, keep-awake flag, browser
  identity and zoom: every field `FR.8.1` names, plus the three item 06 copies
  onto an account at creation. The entry deliberately carries no reference to
  the game file it came from — `realise_account` already reads these off the
  book "never off a preset kept on the side", and a preset file deleted by hand
  must not be able to break a saved workspace.
- **Back-end** — `Liveness` gains `Queued`: the user wants this account running
  and nothing has started it yet. Distinct from `Starting`, which means a view
  exists and no page has painted. Every existing match over `Liveness` grows one
  arm — `park`, `unpark`, `mark_started` and the row derivations task 03 owns —
  rather than acquiring a catch-all, so a state can never be silently swallowed
  (code standards rule 1).
- **Back-end** — `SessionBook::restore(workspace)` rebuilds the book: accounts in
  order, an account saved as running becomes `Queued`, one saved as parked stays
  `Parked`, the saved layout becomes active, the remembered slots are seeded, and
  the minting counter resumes above the highest restored identifier.
- **Back-end** — restore normalises visibility through the same placement the
  layout switch uses, so an entry whose slot the restored layout cannot show
  lands off-grid with that slot remembered and returns to it when a layout with
  that slot is chosen (`FR.3.2`; the roadmap item's fourth blocker, answered
  here because the policy already lives here).
- **Back-end** — `SessionBook::workspace()` returns the value to save, named for
  its result rather than its mechanics (naming rule 11). An account that is
  `Starting` or `Queued` when it is asked is a running account as far as the
  value is concerned: those two states describe a moment, not a wish.
- **Back-end** — `SessionBook::start_order()` returns the queued identifiers in
  book order, and nothing else. Parked accounts are absent entirely, which is
  the whole point of parking surviving a restart (`FR.8.2`).
- **Testing** — unit tests in a `#[cfg(test)] mod tests` at the foot of each
  file, one subject per test, named for the behaviour they pin (architecture
  rule 14, code standards rules 21, 22, 23, 24).

## Acceptance criteria

- [x] `(unit)` restoring a workspace yields its accounts in the saved order, each
      carrying its name, start address, zoom, browser identity and keep-awake
      flag, with the saved layout active
- [x] `(unit)` an account saved as running comes back `Queued`, never `Live`
- [x] `(unit)` an account saved as parked comes back `Parked`
- [x] `(unit)` the start order lists exactly the queued accounts, in the order
      the workspace held them
- [x] `(unit)` a parked account is absent from the start order
- [x] `(unit)` an account whose saved slot the restored layout has no room for
      comes back off-grid, and takes that slot back when a layout that has it is
      chosen
- [x] `(unit)` an account added after a restore is minted an identifier distinct
      from every restored one
- [x] `(unit)` the workspace read off a restored book reproduces the one it was
      restored from, field for field, with a `Starting` or `Queued` account
      reported as running
- [x] `(unit)` restoring an empty workspace leaves an empty book, an empty start
      order and the default layout

## References

- [Roadmap item](../../roadmap/07-workspace-restore/README.md) — the full
  picture, including the "Restoring on launch" diagram that fixes what the core
  owns and what the shell's queue owns
- [`docs/requirements.md`](../../requirements.md) — `FR.8.1`, `FR.8.2`, `FR.3.2`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 5, 6, 8, 9, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 2, 3, 4, 8, 12,
  17, 21, 22, 23, 24
- [`docs/naming.md`](../../naming.md) — rules 6, 9, 10, 11, 12

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
