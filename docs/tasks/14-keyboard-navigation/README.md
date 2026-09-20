# 14 — Keyboard navigation and paged workspaces

This item is split at the model. The replacement of the place-per-account
seating with pages derived from one order comes first, with the version 3
file it forces, because every other slice reads the answers it gives. The
domain's four movements follow, then the keys that call them on GTK and on
Windows, the pager that shows them, the two whole-workspace menu items, and a
closing slice that proves the item on Windows and writes the two design rules
it owes. The shortcuts window is built beside the model change, since it
touches nothing the model touches.

**Waves.** 01 and 02 run in parallel: 01 edits the core, the store and the
three shell readers of the seat model; 02 edits only a new `.ui` resource,
the gresource manifest, `window.ui` and one template test. 03 runs alone after
01, in the same two core files. 04 runs alone after 03: it edits
`window/imp.rs`, the new `window/shortcut.rs` and the sidebar. 05 and 06 run
in parallel after 04: 05 edits only the three WebView2 files; 06 edits
`window.ui` (after 02) and `window/imp.rs` (after 04). 07 runs alone after 06,
because both edit `window/imp.rs`, and it returns to the two core files. 08
runs alone last.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-the-seat-model-and-the-version-3-file.md) | The seat model: pages derived from one order, and the version 3 file | full-stack | — | 10/10 | done |
| [02](02-the-shortcuts-window-and-the-main-menu.md) | The shortcuts window and the main menu | front-end | — | 2/5 | in-progress |
| [03](03-stepping-and-paging-in-the-domain.md) | Stepping and paging in the domain | back-end | 01 | 9/9 | done |
| [04](04-one-shortcut-table-and-the-keys-on-the-gtk-controller.md) | One shortcut table, and the keys on the GTK controller | front-end | 03 | 4/9 | in-progress |
| [05](05-the-keys-on-windows.md) | The keys on Windows | front-end | 04 | 2/6 | in-progress |
| [06](06-the-header-bar-pager.md) | The header-bar pager | front-end | 02, 04 | 2/8 | in-progress |
| [07](07-park-all-and-start-all.md) | Park all and Start all | full-stack | 06 | 0/10 | not-started |
| [08](08-windows-verification-and-the-design-rules.md) | Windows verification, the design rules and the runbook | full-stack | 05, 07 | 0/6 | not-started |
