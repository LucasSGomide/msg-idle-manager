# 01 — Isolated game accounts in one splittable window

Sliced so that the two halves the item can prove differently stay apart: the
placement rules land as pure core code with real unit tests, and everything the
shell draws lands as slices whose evidence is this folder's `test-script.md`,
because architecture rule 14 and code standards rule 25 keep a display server
out of `cargo test`. The build machinery the item's blockers call out goes
first, and the web view goes last, so every slice in between is something a
person can see working without a game loaded.

**Waves.** 01 and 02 depend on nothing and touch disjoint crates — the shell and
the binary against the core — so they are safe to run in parallel. 03 and 04
both need 02 and touch nothing in common, one working in the core, store and
binary and the other only in the shell, so they are the second parallel pair. 05
rewrites the grid 04 created and runs alone. 06 needs the directories from 03
and the grid from 05 and runs alone.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-application-shell.md) | Application shell and the resource pipeline | front-end | — | 5/5 | done |
| [02](02-session-and-layout-domain.md) | Session and layout domain | back-end | — | 8/8 | done |
| [03](03-profile-directories.md) | Profile directories port and its XDG adapter | back-end | 02 | 7/7 | done |
| [04](04-adding-an-account.md) | Adding an account and placing it in a slot | front-end | 01, 02 | 7/7 | done |
| [05](05-changing-the-arrangement.md) | Changing the arrangement | front-end | 04 | 7/7 | done |
| [06](06-isolated-web-view.md) | The isolated web view per account | front-end | 03, 05 | 11/11 | done |
