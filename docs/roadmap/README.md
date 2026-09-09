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

Items 01 to 04, 06, 07 and 09 are done. They built the base every other item assumes — a
session with its own storage on disk, a way to create one, the grid that holds
views out of sight, and the sidebar index of every account — plus the two
controls over what an account costs (parking, which hands memory back and takes
it again, and keep-awake, which keeps a game running while the window is down)
and the game catalogue that supplies an account's address, zoom and identity so
adding one is a name and a click. Item 07 then made the arrangement durable: the
workspace is written after every change and restored one account at a time on
launch, so the application can be left running for days and reopened without
thought.

Item 04 also settled what the grid's off-grid trick is worth: an account with no
place on screen is never marked hidden, so nothing throttles it. Keep-awake earns
its place only while the window is minimised, and both mechanisms were measured
doing so.

**Next up: 05 — Memory accounting and performance.** It turns the claim the
project rests on into a number on screen: what the application costs, and what
parking actually returns. Everything before it asserted that; nothing had shown
it — and the first measurement, taken on 2026-09-08 while the item was being
sliced, showed the claim failing. One live account with every other account
parked cost 884 MiB, most of it in a single rendering process, and the browser
this application replaces cost less for the same game. So the item grew a second
half and an estimate: it builds the instrument, then uses it, and its budget is
written as a comparison against that browser rather than as a figure chosen for
itself. The measurement also confirmed item 03's promise — there was exactly one
rendering process for the one live account, so parking really does hand the
memory back. It also gives item 04's open question a way to be answered: the
shim's frame interval is a guess with no measurement behind it, and this is the
first item that can price it. Item 08 then adds recovery from a crash nobody was
awake to see, building on the durable workspace 07 just landed.

Item 09 shipped the first file the application writes on an account's behalf — a
per-account `state.toml` holding the zoom chosen per arrangement — and with it
`Ctrl` +/-/0 and `Ctrl`+wheel resizing the focused game in place. It also set
the shape design rule 10 now names for any transient acknowledgement over live
content.

The `### Back-end` and `### Front-end` headings inside each item map onto this
repository's crates rather than onto a server and a browser: back-end means the
non-widget crates — the pure domain in `idle-manager-core`, the disk adapter in
`idle-manager-store`, the `/proc` adapter in `idle-manager-metrics` and the
composition root — and front-end means `idle-manager-shell`, the GTK 4 and
WebKitGTK layer. See [`../architecture.md`](../architecture.md).

Every front-end section in every item cites `docs/design.md` and finds few rules
to cite, because that file started nearly empty. Each item therefore records its
patterns as new and says what rule the design doc owes once the code exists. That
debt is being paid down: 02 added the sidebar row-status-marker rule, 03 the
inverting row action, the two meanings of dimming and the absent-game panel, 04
where a row's settings live and how the sidebar's width follows from what its
rows' trailing edge carries, and 06 how an escape-hatch option and a partial
failure are shown inside a chooser without a modal of their own.

## Ready

| # | Item | Est | Depends on | Status |
|---|---|---|---|---|
| [05](05-memory-accounting/README.md) | Memory accounting and performance | 8 | 02, 03 | not-started |
| [08](08-crash-recovery/README.md) | Surviving a crashed game | 5 | 03 | not-started |

## Blocked

_(none)_

## Parked

_(none)_

## Done

| # | Item | Est | Depends on | Status |
|---|---|---|---|---|
| [01](01-isolated-accounts-and-layouts/README.md) | Isolated game accounts in one splittable window | 13 | — | done |
| [02](02-session-sidebar/README.md) | The session sidebar | 5 | 01 | done |
| [03](03-parking-and-unparking/README.md) | Parking and unparking a session | 8 | 02 | done |
| [04](04-keep-awake/README.md) | Keep-awake for hidden games | 5 | 02 | done |
| [06](06-presets-and-adding-accounts/README.md) | Presets and adding an account | 5 | 01, 04 | done |
| [07](07-workspace-restore/README.md) | Restoring the workspace on launch | 8 | 03, 04, 06 | done |
| [09](09-interactive-zoom/README.md) | Interactive zoom, remembered per arrangement | 5 | 03, 06 | done |
