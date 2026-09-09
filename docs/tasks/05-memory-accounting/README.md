# 05 — Memory accounting and performance

Sliced along the seam items 01 to 07 used: what a reading *is* lands as pure core
code with unit tests, what the kernel *says* lands as an adapter slice with
integration tests against captured `/proc` output, and everything a screen does
lands as slices whose evidence is this folder's `test-script.md`, because a
display server is kept out of `cargo test`. This item adds a fourth kind the
repository has not had before — a slice whose deliverable is a measurement — and
it is treated the same way: it has acceptance criteria, and they are checkable
by reading the file it produces.

The item's two halves are not two independent tracks. Accounting comes first
because every performance claim in the second half is a before-and-after pair of
figures, and there is nothing to take them with until task 03 exists. That
ordering is the reason folding performance into this item is cheaper than
opening a separate one: the instrument and its first user ship together.

**What the measurement changed before slicing.** The plan was written against an
unmeasured application and one sentence of it was wrong. It said to find the
engine's processes by matching direct children against our own identifier;
measured, the rendering process holding 751 of the application's 884 MiB turns
out to be a *grandchild*, started inside two nested sandboxes, and matching
direct children would have reported an application costing 130 MiB and called it
good news. Both task 02 and task 03 walk the whole descendant tree, and both
have an acceptance criterion that fails if they stop at one level.

**Three decisions settled before slicing.** The budget *rule* lands in task 01
with the reading, and the budget *number* in task 08 with the warning that
consumes it — because the comparison is pure domain logic that can be finished
and tested against any threshold, while the number cannot exist until three
games have been measured against a browser. Splitting them is what stops task 01
waiting on an afternoon of measuring. The soak is its own slice rather than a
step inside another, because it is bounded by wall-clock time rather than
working time and has to start as early as its dependency allows. And the two
reductions are separate slices — task 06 needs the soak's number and task 07
does not — so the cheaper one is not held behind four hours of sampling.

**What this item does not do.** It never acts on an account by itself. The
warning states a fact and stops there, because the engine offers no way to link
a process to an account (`FR.7.4`) and so the application cannot know which
account to give up even if it were entitled to choose. Automatic parking would
need attribution first, and attribution is a separate item that starts by
finding a way to make the link at all.

**Waves.** 01 and 03 run in parallel — one is `ports.rs` in the core, the other
is `scripts/` and the Makefile, and neither opens a file the other writes. Then
02, 05 and 07 run in parallel: 02 is the metrics crate, 05 touches nothing but
`docs/memory-budget.md` and a terminal left running, and 07 is the two
`preset.rs` files plus `web_view.rs`. 05 should be started first of the three
and left going, since its four hours are the item's critical path. 04 and 06
then run in that order rather than in parallel — 04 adds its module to the
shell's `lib.rs` and 06 rewrites `configure_web_engine` in the same file. 08
runs last and alone: it needs the readout to warn on, the reductions in place
before the budget is measured, and the soak's figures to write beside it.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-the-reading-and-the-budget-verdict.md) | The reading, the port and the budget verdict | back-end | — | 7/7 | done |
| [02](02-walking-the-process-tree.md) | Walking the process tree and reading its proportional memory | back-end | 01, 03 | 8/8 | done |
| [03](03-the-memory-report-script.md) | The memory report script and its soak mode | tooling | — | 0/7 | not-started |
| [04](04-the-sidebar-footer.md) | The sidebar footer | front-end | 01, 02 | 0/10 | not-started |
| [05](05-the-soak-and-what-it-found.md) | The soak, and whether it is a leak | measurement | 03 | 0/7 | not-started |
| [06](06-telling-the-engine-it-has-a-limit.md) | Telling the engine it has a limit | front-end | 05 | 0/7 | not-started |
| [07](07-diagnostics-off-and-webgl-per-game.md) | Diagnostics off by default, WebGL per game | full-stack | 03 | 0/8 | not-started |
| [08](08-the-budget-and-the-warning.md) | The budget, measured against the browser, and the warning | full-stack | 04, 05, 06, 07 | 0/9 | not-started |
