# 10 — Renaming and rearranging accounts

Sliced along the seam items 01 to 09 used: what a rename or a move *means* lands
as pure core code with unit tests, and everything the screen does lands as shell
slices whose evidence is this folder's `test-script.md`. Rename and rearrange are
independent features that happen to share the grid, so each gets its own core
slice and its own shell slice.

The drag is split at the grip. Task 04 ships a grip that shows on hover, never
steals a click from the game underneath, and starts a drag carrying a private
type. With no drop target yet, GTK ends that drag and nothing moves. Task 05 adds
the drop, the highlight and the window's handler. Keeping them apart puts the
checks that the grip leaves the game alone in one runbook section, and the checks
that a drop does exactly one thing in another.

**Waves.** 01 and 04 run first, in parallel: 01 stays in `session.rs` and the
add-game dialog, 04 in the grid module and its stylesheet. 02 and 03 then run in
parallel. 02 needs 01's rename and edits `session_grid/imp.rs` after 04 has. 03
edits `session.rs` after 01 has. 02 is shell-only and 03 is core-only, so they
share no file. 05 needs 03's move and 04's dragged-account type, and edits
`window/imp.rs` and `session_grid/imp.rs` after 02, so it runs alone and last.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-one-rule-for-an-account-name.md) | One rule for an account name, and renaming in the book | back-end | — | 9/9 | done |
| [02](02-renaming-from-the-sidebar.md) | Renaming an account from the sidebar | front-end | 01, 04 | 9/9 | done |
| [03](03-moving-an-account-in-the-book.md) | Moving an account between places in the book | back-end | 01 | 0/10 | not-started |
| [04](04-the-grip-over-each-place.md) | The grip over each place | front-end | — | 9/9 | done |
| [05](05-dropping-an-account-on-a-place.md) | Dropping an account on a place | front-end | 02, 03, 04 | 0/9 | not-started |
