# 11 — Account workspaces, and deleting an account

Sliced along item 10's seam where the compiler allows it. What grouping and
switching *mean* is core code with unit tests, the saved file is proved by store
integration tests, and everything on screen is proved by this folder's
`test-script.md`. Task 01 is the exception. Changing the saved value from one
workspace to a list of them breaks the store, the saver and the window together,
so that change lands in one slice with no visible difference. Task 02 carries the
switching rules along with the tree that uses them, which keeps the item at eight
tasks.

Deleting is split at the port. Task 04 removes exactly one account folder behind
a port, with tests against a real temporary directory. Task 08 wires in the
confirmation window, the ordered engine steps and the failure page.

**Waves.** 01 runs alone. 02 and 04 then run in parallel. 02 edits
`workspace_book.rs` and the shell, and 04 only `ports.rs` and the store's
`paths.rs`. 03 edits `workspace_book.rs` after 02 and runs alone. 05, 06, 07 and
08 each edit `window/imp.rs`, and 05 and 06 both edit the sidebar, so they run
one at a time in that order. 08 also needs 04's removal port.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-ungrouped-workspace-and-the-version-2-file.md) | Ungrouped workspace and the version 2 file | back-end | — | 10/10 | done |
| [02](02-sidebar-tree-and-switching-workspaces.md) | Sidebar tree and switching workspaces | full-stack | 01 | 10/10 | done |
| [03](03-moving-and-grouping-accounts-in-the-book.md) | Moving and grouping accounts in the book | back-end | 02 | 10/10 | done |
| [04](04-removing-one-accounts-folder.md) | Removing one account's folder | back-end | 01 | 6/6 | done |
| [05](05-selection-mode-and-moving-accounts.md) | Selection mode and moving accounts | front-end | 02, 03 | 10/10 | done |
| [06](06-naming-workspaces-new-rename-and-remove.md) | Naming workspaces: New, Rename and Remove | front-end | 05 | 10/10 | done |
| [07](07-choosing-a-workspace-when-adding-a-game.md) | Choosing a workspace when adding a game | front-end | 06 | 7/7 | done |
| [08](08-deleting-an-account.md) | Deleting an account | front-end | 04, 07 | 10/10 | done |
