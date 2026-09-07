# 04 — Keep-awake for hidden games

Sliced on the seam items 01–03 used: the state rule lands as pure core code with
real unit tests, and everything the engine or the screen does lands as slices
whose evidence is this folder's `test-script.md`, because a display server is
kept out of `cargo test`. The two unknowns the item records as blockers — whether
an out-of-sight account's page is marked hidden at all, and what the engine
actually calls the two switches — are pulled into their own measurement slice at
the front, because the item says plainly that nothing else in it can be judged
until they are answered, and because that slice needs no caller to be verified.
The switch and what it switches ship together, since a menu that sets a flag
nothing reads is not a slice anyone can accept. The injected frame-callback shim
is separate from the engine's own switches even though both live in the session
view holder: the switches are a supported setting, the shim is a workaround with
a guessed interval, and a game that stops running should be traceable to one or
the other rather than to both at once. The row's persistent indication comes last
because the row already reads as reloading the moment the toggle works.

**Waves.** 01 and 02 depend on nothing and touch disjoint crates — 01 works only
in `idle-manager-core`, 02 only in `idle-manager-shell`'s `web_view.rs` — so they
are safe to run in parallel. 03 wires the menu into both of them, 04 extends the
same holder 03 leaves behind, and 05 draws what 03 sets, so each of the three
runs alone in its own wave.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-keep-awake-in-the-session-book.md) | Keep-awake in the session book | back-end | — | 10/10 | done |
| [02](02-what-the-engine-does-to-a-hidden-page.md) | What the engine does to a hidden page | front-end | — | 6/6 | done |
| [03](03-the-row-menu-and-the-engines-switches.md) | The row menu and the engine's switches | front-end | 01, 02 | 9/9 | done |
| [04](04-the-frame-callback-shim.md) | The frame-callback shim | front-end | 03 | 9/9 | done |
| [05](05-the-rows-keep-awake-indication.md) | The row's keep-awake indication | front-end | 04 | 6/6 | done |
