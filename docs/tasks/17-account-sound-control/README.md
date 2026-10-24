# 17 — Muting one account's sound

Sliced by what can ship on its own. The flag comes first, in the pure domain and
on disk, because it is testable in milliseconds with no display and nothing else
can be checked without it. The engine call and the holder that remembers the
flag come beside it, since they touch only the interface crate's engine seam and
compile against a plain boolean. The menu item, the window handler and the
restore path close the loop last, because they are the one slice that needs both
of the others to do anything visible.

**Waves.** 01 and 02 depend on nothing and share no files — 01 edits
`idle-manager-core` and `idle-manager-store`, 02 edits `web_engine/` and
`web_view.rs` and only passes `false` at `window/imp.rs`'s three holder call
sites — so they can run in parallel. 03 needs both and runs alone.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-the-muted-flag-in-the-domain-and-on-disk.md) | The muted flag in the domain and on disk | back-end | — | 9/9 | done |
| [02](02-page-level-mute-on-both-engines-and-the-holder.md) | Page-level mute on both engines and the holder | front-end | — | 4/4 | done |
| [03](03-the-muted-menu-item-the-window-and-restore.md) | The Muted menu item, the window handler and restore | front-end | 01, 02 | 6/6 | done |
