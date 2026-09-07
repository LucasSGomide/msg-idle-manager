# 09 — Interactive zoom, remembered per arrangement

Sliced along the seam items 01 to 07 used: what a size *is* lands as pure core
code with unit tests, what a file *says* lands as a store slice with integration
tests against temporary directories, and everything a screen does lands as slices
whose evidence is this folder's `test-script.md`, because a display server is
kept out of `cargo test`.

Three decisions the item left open were settled before slicing. `restore_zoom`
lands with the resolution rule in task 01 rather than with the gestures in
task 02, because it is installation rather than a transition and because without
it task 01 has no way to put a remembered size in place to test the fallback
against. The two gestures are separate slices rather than one, because they
differ in the only thing that matters — which account they act on — and because
the wheel carries the item's one unverified mechanism, a capture-phase scroll
controller over a web view that handles scrolling in its own process; task 05
settles that blocker by measurement and records which way it went. And the
readout ships with the keyboard gesture rather than on its own, because three of
the wireframe's four variants — a step at the limit, a step over a parked
account, and a run of steps collapsing into one figure — are invisible without
it, so a keyboard slice without the figure would have nothing to accept against.

Consuming remembered sizes is separate from writing them down. Task 06 leaves an
application where an evening's work survives every arrangement switch and dies at
the window close, which is exactly the state task 07's file consumes; splitting
them is what keeps "only a gesture writes" a behaviour with its own checks rather
than a condition nobody looks at.

The item's first blocker needs no slice: item 07 landed durable per-account
identity, so a remembered size can no longer attach to an account that never
chose it. The remaining unknowns are each pinned to the slice that meets them —
which key values a keyboard delivers, and whether a page already loaded reflows
sanely when resized, both in task 04's runbook steps; the scroll controller in
task 05's.

**Waves.** 01 runs alone — every other slice needs the split baseline, the
resolution rule and `Layout` as a map key. 02 and 03 then run in parallel: 02
stays inside `SessionBook` in `session.rs`, 03 inside `ports.rs` and
`idle-manager-store`, and neither opens a file the other writes. 04 needs the
book's transitions from 02 and runs alone. 05, 06 and 07 each rewrite
`window/imp.rs` throughout, so they run alone and in that order; 07 additionally
needs the port from 03.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-per-arrangement-zoom-in-the-domain.md) | Per-arrangement zoom in the domain | back-end | — | 8/8 | done |
| [02](02-the-books-zoom-transitions.md) | The book's zoom transitions and the focused account | back-end | 01 | 9/9 | done |
| [03](03-the-account-state-file.md) | The zoom memory port and the account state file | back-end | 01 | 8/8 | done |
| [04](04-the-keyboard-gesture-and-the-readout.md) | Zooming the focused account from the keyboard | front-end | 02 | 1/10 | in-progress |
| [05](05-the-wheel-gesture-over-a-place.md) | Zooming the account under the pointer | front-end | 04 | 0/9 | not-started |
| [06](06-snapping-on-an-arrangement-switch.md) | Snapping every account on an arrangement switch | front-end | 05 | 0/6 | not-started |
| [07](07-remembering-the-sizes-on-disk.md) | Remembering the sizes across a restart | full-stack | 03, 06 | 0/8 | not-started |
