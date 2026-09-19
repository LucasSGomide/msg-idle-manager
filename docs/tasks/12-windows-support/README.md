# 12 — Running natively on Windows

This item is split at the web-engine seam. Task 01 is a pure move on Linux:
every WebKit call goes behind one shell-internal module, and nothing changes on
screen. Task 02 adds the Windows backend and the cross-check that keeps Windows
compiling from Linux, enough for games to show in the Windows VM.
Tasks 03–06 then bring each remaining behaviour across. Task 07 adds the Windows
memory reader. Task 08 packages the zip on Linux and compares memory against Edge.
Most `(manual)` steps run in the Windows 11 VM described in
[`docs/windows-vm.md`](../../windows-vm.md). Acceptance covers only what that
VM can show. Every Linux feature is still implemented for Windows, and a
developer with a real Windows machine checks the rest informally.

**Waves.** Task 01 runs alone, then task 02 runs alone.

Tasks 03, 05 and 07 then run in parallel, because they touch different files:
- 03 edits the Windows engine files, `ffi.rs` and `window/imp.rs`;
- 05 edits only the grid, its CSS and the engine host widget;
- 07 edits only the metrics crate and one line of `main.rs`.

Tasks 04 and 06 run one at a time after 03, in that order. They edit the same
engine files, and 04 also edits `window/imp.rs`.

Task 08 needs everything else finished and runs alone.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-engine-seam-on-linux.md) | One web-engine seam, on Linux, with no visible change | front-end | — | 8/8 | done |
| [02](02-windows-build-and-first-light.md) | Building for Windows, and games showing there | full-stack | 01 | 10/10 | done |
| [03](03-scripts-popups-zoom-and-crashes-on-windows.md) | Page scripts, sign-in popups, zoom and crashes on Windows | front-end | 02 | 8/8 | done |
| [04](04-keep-awake-and-minimising-on-windows.md) | Keep-awake and minimising on Windows | front-end | 03 | 7/7 | done |
| [05](05-controls-over-a-game-on-windows.md) | Keeping the controls over a game visible on Windows | front-end | 02 | 8/8 | done |
| [06](06-deleting-an-account-on-windows.md) | Deleting an account on Windows | full-stack | 04 | 7/7 | done |
| [07](07-memory-figures-on-windows.md) | Memory figures on Windows | back-end | 02 | 7/7 | done |
| [08](08-release-zip-and-measurements.md) | The Windows release zip, and measuring its memory | full-stack | 05, 06, 07 | 6/6 | done |
