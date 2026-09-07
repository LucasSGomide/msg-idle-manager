# 07 — Restoring the workspace on launch

Sliced along the same seam items 01 to 06 used: what an arrangement *is* lands
as pure core code with real unit tests, what a file *says* lands as a store
slice with integration tests against temporary directories, and everything a
screen does lands as slices whose evidence is this folder's `test-script.md`,
because a display server is kept out of `cargo test`.

Four decisions the item left open were settled before slicing. A restored-but-not
-yet-started account is a fourth `Liveness` value in the core rather than a set
of identifiers the shell carries on the side, because the row's marker already
derives from liveness alone and a second state vocabulary outside the book would
be a second place to keep in step. The file carries each account's address, zoom
and identity rather than a reference to the game file it came from, so deleting
a preset by hand cannot break a saved arrangement — the same independence
`realise_account` was written for. The format carries a version key from its
first write, because adding one later means guessing at files that predate it.
And an account whose saved slot the restored arrangement cannot show goes
off-grid with its slot remembered, which is item 01's existing placement rule
applied to a new caller rather than a new rule.

The item's first blocker needed no slice: `gtk::Application` is built with an
application id and default flags, so a second launch already activates the open
window instead of starting a second process. Task 04 pins that with a criterion
rather than building anything. The third blocker — what "settled" means for an
idle game, whose page keeps fetching long after it reports itself loaded — stays
open on purpose: task 05 ships the load-finished signal plus a named timeout the
item already specifies, and the measurement that justifies the number is a step
in this folder's runbook.

Reading a file is separate from writing one, and drawing an arrangement is
separate from filling it. Task 04 leaves a window that comes back perfectly
arranged with nothing loaded, which is exactly the state task 05's queue
consumes; splitting them is what keeps "one at a time" a behaviour with its own
checks rather than a loop nobody watches. Task 03 comes before both because a
vocabulary written under pressure, while the thing that produces the state is
being built, is how a fifth row state turns into a branch instead of an arm.

**Waves.** 01 runs alone — everything needs the workspace value and the port.
02 and 03 then run in parallel: 02 stays inside `idle-manager-store` and its
tests, 03 inside `idle-manager-shell`'s sidebar row, grid and stylesheet, and
neither opens a file the other touches. 04 needs the file from 02 and the queued
vocabulary from 03, and runs alone. 05 and 06 each rewrite `window/imp.rs`
throughout, so they run alone and in that order: 06 needs nothing from 05
logically, but the two cannot share a working tree at the same time, and 06 is
the one that can wait.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-the-workspace-in-the-domain.md) | The workspace in the domain, and coming back from one | back-end | — | 9/9 | done |
| [02](02-the-workspace-file-on-disk.md) | The workspace file on disk | back-end | 01 | 9/9 | done |
| [03](03-the-queued-row-and-placeholder.md) | The queued row and the queued slot | front-end | 01 | 7/7 | done |
| [04](04-restoring-the-arrangement-on-launch.md) | Restoring the arrangement on launch | full-stack | 02, 03 | 7/8 | in-progress |
| [05](05-the-start-queue.md) | The start queue | front-end | 04 | 0/7 | not-started |
| [06](06-saving-after-every-change.md) | Saving after every change | front-end | 04, 05 | 0/8 | not-started |
