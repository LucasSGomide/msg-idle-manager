# 02 — The session sidebar

Sliced down the same seam as item 01: the placement rule lands as pure core code
with real unit tests, and everything the shell draws lands as slices whose
evidence is this folder's `test-script.md`, because a display server is kept out
of `cargo test`. The list is split from the click — one slice makes the sidebar
appear and fold, the next makes a row do something — so a failure in the
two-surface wiring is isolated to the slice that introduced it.

**Waves.** 01 and 02 depend on nothing and touch disjoint crates — 01 works only
in `idle-manager-core`, 02 only in `idle-manager-shell` — so they are safe to run
in parallel. 03 needs the core intent from 01 and edits the widget and window
wiring 02 created, so it runs alone in the second wave.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-bringing-a-session-into-the-focused-slot.md) | Bringing a session into the focused slot | back-end | — | 5/5 | done |
| [02](02-the-sidebar-and-its-fold.md) | The sidebar and its fold | front-end | — | 7/7 | done |
| [03](03-clicking-a-row-to-swap-or-focus.md) | Clicking a row to swap or focus | front-end | 01, 02 | 5/5 | done |
