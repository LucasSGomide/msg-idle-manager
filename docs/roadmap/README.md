# Roadmap

Committed work. One doc per item, numbered on creation — **the number is a
permanent ID, never renumbered**. Ordering lives in this table only.

**Rules**

- Every table below is generated from the docs' metadata headers (`Depends on` /
  `Status` / `Estimate`). **Edit the doc, not the table.**
- Sections: **Ready** (every dependency `done`, not yet finished) first, then
  **Blocked**, each by estimate desc, ties by number asc. Then **Parked**.
  **Done** last, sorted by number asc.
- `Depends on` is roadmap numbers only. `—` means nothing blocks it.
- Status: `not-started` · `in-progress` · `parked` · `done`. Derived from the
  item's task checkboxes whenever a breakdown is open.
- `Landed:` / `Merged:` — added to the header when the branch ships (`but land`
  or a `git merge`). It is the only thing that retires the item's task breakdown;
  `done` alone does not.
- The prose above the table is hand-written and says **why** the next item is
  next. The table sorts by estimate; that sort is not a priority.

**Next up: 01 — Isolated game accounts in one splittable window.** Nothing else
can start. Item 01 is the only one with no dependency because it builds the three
things every other item assumes: a session with its own storage on disk, a way to
create one, and the grid that holds views out of sight without letting the engine
throttle them. Items 02 to 08 each add one switch or one surface to that base, in
roughly the order a user would miss them — an index of accounts, the memory
control the project exists for, the background-speed flag, the readout that turns
the memory claim into a measurement, the game catalogue, durability across a
restart, and finally recovery from a crash nobody was awake to see.

The `### Back-end` and `### Front-end` headings inside each item map onto this
repository's crates rather than onto a server and a browser: back-end means the
non-widget crates — the pure domain in `idle-manager-core`, the disk adapter in
`idle-manager-store`, the `/proc` adapter in `idle-manager-metrics` and the
composition root — and front-end means `idle-manager-shell`, the GTK 4 and
WebKitGTK layer. See [`../architecture.md`](../architecture.md).

Every front-end section in every item cites `docs/design.md` and finds no rule to
cite, because that file has none yet. Each item therefore records its patterns as
new and says what rule the design doc owes once the code exists. Item 01 is where
that debt starts being paid.

## Ready

| # | Item | Est | Depends on | Status |
|---|---|---|---|---|
| [01](01-isolated-accounts-and-layouts/README.md) | Isolated game accounts in one splittable window | 13 | — | in-progress |

## Blocked

| # | Item | Est | Depends on | Status |
|---|---|---|---|---|
| [03](03-parking-and-unparking/README.md) | Parking and unparking a session | 8 | 02 | not-started |
| [07](07-workspace-restore/README.md) | Restoring the workspace on launch | 8 | 03, 04, 06 | not-started |
| [02](02-session-sidebar/README.md) | The session sidebar | 5 | 01 | not-started |
| [04](04-keep-awake/README.md) | Keep-awake for hidden games | 5 | 02 | not-started |
| [05](05-memory-accounting/README.md) | Memory accounting | 5 | 02, 03 | not-started |
| [06](06-presets-and-adding-accounts/README.md) | Presets and adding an account | 5 | 01, 04 | not-started |
| [08](08-crash-recovery/README.md) | Surviving a crashed game | 5 | 03 | not-started |

## Parked

_(none)_

## Done

_(none)_
