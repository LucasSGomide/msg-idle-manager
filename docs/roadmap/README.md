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

Items 01 to 07 and 09 are done. They built the base every other item assumes — a
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

Item 05 — Memory accounting and performance — turned the claim the project rests
on into a number on screen: what the application costs, and what parking actually
returns. Its first measurement, taken on 2026-09-08, showed the claim failing —
one live account cost 884 MiB, more than the browser this application replaces —
so the item grew a second half that used the instrument it built, and its budget
is written as a comparison against that browser rather than a figure chosen for
itself.

Item 11 shipped named workspaces and the first action that removes an
account's data from disk — an account's folder is deleted through a confirm
window and a fixed sequence, since a measurement showed the web engine never
releases a deleted account's files, so deletion removes the folder without
waiting for them.

**Next up: 08.** Item 08 adds recovery from a crash nobody was awake to see,
building on the durable workspace 07 landed.

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
| [05](05-memory-accounting/README.md) | Memory accounting and performance | 8 | 02, 03 | done |
| [06](06-presets-and-adding-accounts/README.md) | Presets and adding an account | 5 | 01, 04 | done |
| [07](07-workspace-restore/README.md) | Restoring the workspace on launch | 8 | 03, 04, 06 | done |
| [09](09-interactive-zoom/README.md) | Interactive zoom, remembered per arrangement | 5 | 03, 06 | done |
| [10](10-rearranging-accounts/README.md) | Renaming and rearranging accounts | 8 | 03, 07 | done |
| [11](11-account-workspaces/README.md) | Account workspaces, and deleting an account | 13 | 06, 07, 10 | done |
