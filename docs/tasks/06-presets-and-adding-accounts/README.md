# 06 — Presets and adding an account

Sliced along the seam items 01 to 04 used: what a game *is* lands as pure core
code with real unit tests, what a folder of files *says* lands as store slices
with integration tests against temporary directories, and everything a screen or
the engine does lands as slices whose evidence is this folder's
`test-script.md`, because a display server is kept out of `cargo test`.

The two decisions the item recorded as blockers were settled before slicing.
Zoom is a plain multiplier, exactly what the engine takes, because the item's own
rule is that a preset's values are consumed once at creation and never read
again — a screen-relative value would have to be recomputed on every layout
change, which is a different item. The three shipped games are Huntera, Baiaki
Idle and Lorvath.

Reading the folder is separate from filling it: task 02 ships a reader that
copes with an empty folder and a bad file, and task 03 ships the three files and
the first-run write. Splitting them is what makes the "never overwrite what the
user edited" rule a behaviour with its own tests rather than a line in a
constructor nobody exercises. The dialog is one slice and not two, because a
two-stage dialog cannot ship half-built without leaving a list you can look at
but not act on. The zoom and the browser identity come *after* the dialog rather
than before it: until an account can be created from a game, every account gets
the defaults and there is nothing to see.

**Waves.** 01 runs alone, and 02 runs alone after it — everything else needs the
port and the reader. 03 and 04 then run in parallel: 03 stays inside
`idle-manager-store` and `presets/`, 04 inside `idle-manager-shell` and
`main.rs`, and 03 lands seeding in the body of the constructor 02 fixed, so it
never touches the composition root 04 edits. 05 extends the holder and the
window 04 leaves behind and checks itself against the three games 03 ships, so
it depends on both and runs alone. A tester on 04, before 03 has landed, writes
a game file into the presets folder by hand — which is the path a user adding
their own game takes anyway.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-the-preset-in-the-domain.md) | The preset in the domain and the catalogue port | back-end | — | 9/9 | done |
| [02](02-reading-the-game-files.md) | Reading the game files from the configuration folder | back-end | 01 | 9/9 | done |
| [03](03-the-shipped-games-and-seeding.md) | The shipped game files and first-run seeding | back-end | 02 | 7/7 | done |
| [04](04-the-dialogs-game-chooser.md) | The add-game dialog's game chooser | front-end | 02 | 10/10 | done |
| [05](05-the-games-zoom-and-identity.md) | A view drawn at the game's zoom and browser identity | front-end | 03, 04 | 9/9 | done |
