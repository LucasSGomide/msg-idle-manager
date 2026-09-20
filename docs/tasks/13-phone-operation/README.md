# 13 — Operating an account from a phone

This item is split at its two seams. The vocabulary the desktop and the phone
share, and the ports between them, come first, beside the engine seam's three
new calls, because everything else is written against those. Then the two
halves that can be built apart: the fourth layout on the desktop, and the
server in its own crate. The phone page and the desktop wiring follow, the
enrolment dialog last on the desktop, and one closing slice verifies Windows
and records the measurements.

**Waves.** 01 and 02 run in parallel: 01 touches the core, the store, the
workspace manifest and the architecture doc; 02 touches only the shell's engine
seam, view holder and the frame shim script. 03 and 04 run in parallel after
01: 03 edits the core's layout and book, the store's layout mapping, the grid
and the window; 04 edits only the new remote crate. 05 and 06 run in parallel:
05 edits the remote crate's page and its one route; 06 edits the binary, the
window and the view holder. 07 runs alone after 06, because both edit the
window. 08 runs alone last.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-remote-vocabulary-ports-and-phone-record.md) | The remote vocabulary, its two ports, and the phone record on disk | back-end | — | 8/9 | in-progress |
| [02](02-capture-script-and-wake-on-the-engine-seam.md) | Capturing a frame, running a script and waking a view on both engines | front-end | — | 5/9 | in-progress |
| [03](03-the-mobile-layout-on-the-desktop.md) | The Mobile layout on the desktop | full-stack | 01 | 6/10 | in-progress |
| [04](04-the-remote-server.md) | The remote server: enrolment, the socket and its proof | back-end | 01 | 9/10 | in-progress |
| [05](05-the-phone-page.md) | The phone page | front-end | 04 | 0/10 | not-started |
| [06](06-wiring-the-phone-into-the-window.md) | Wiring the phone into the window | full-stack | 02, 03, 04 | 0/9 | not-started |
| [07](07-the-phone-dialog-and-header-menu.md) | The phone dialog and the header menu | front-end | 06 | 0/8 | not-started |
| [08](08-windows-verification-and-measurements.md) | Windows verification, the measurements and the docs | full-stack | 05, 07 | 0/6 | not-started |
