# 03 — Parking and unparking a session

Sliced along the same seam as items 01 and 02: the state rule lands as pure core
code with real unit tests, and everything the shell draws lands as slices whose
evidence is this folder's `test-script.md`, because a display server is kept out
of `cargo test`. The risky restructure — turning the web view into a holder that
outlives it, and telling the engine to keep no process cache — is separated from
the feature that needs it, so a page that stops loading is traceable to the
refactor rather than to the button. Stopping is split from starting because
stopping is the half proved with `ps` rather than a screenshot, and it is where
the item's cache-model blocker is either settled or the item grows. The
placeholder panel comes last: parking already works without it, and it is the
one piece item 08 inherits, so it is written once both directions of the
transition are known.

**Waves.** 01 and 02 depend on nothing and touch disjoint crates — 01 works only
in `idle-manager-core`, 02 only in `idle-manager-shell` — so they are safe to
run in parallel. 03, 04 and 05 each edit `session_sidebar`, `session_grid` and
`window/imp.rs`, and each builds on the state the one before it left on screen,
so each runs alone in its own wave.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-liveness-in-the-session-book.md) | Liveness in the session book | back-end | — | 8/8 | done |
| [02](02-the-session-view-holder-and-the-cache-model.md) | The session view holder and the cache model | front-end | — | 3/6 | in-progress |
| [03](03-parking-a-running-account.md) | Parking a running account | front-end | 01, 02 | 7/7 | done |
| [04](04-starting-a-parked-account.md) | Starting a parked account | front-end | 03 | 5/7 | in-progress |
| [05](05-the-parked-slot-placeholder.md) | The parked slot placeholder | front-end | 04 | 3/6 | in-progress |
