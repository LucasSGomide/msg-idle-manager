# 03 — Clearing a game's cache to get its memory back

**Ditched:** 2026-10-04 · **Estimate:** 3

## Idea

A per-account `Clear cache` action, with a one-click `Restart` beside it, so a
game that grows the longer it stays open — mu idle, https://muidle.xyz/,
measured by the owner at close to 10 GB — could be brought back to a small
footprint without logging out or losing its save. Recorded on 2026-10-04 as
needs UN.29 and UN.30 of Session Management, then withdrawn the same day
before anything was committed; the rows were removed rather than kept, since
the log had never carried them.

## Why not

- Clearing cache does not touch the memory that grows. A web game's growth is
  its rendering process's heap, not the HTTP cache, and
  `WebsiteDataManager::clear` on the account's `NetworkSession`
  (`crates/idle-manager-shell/src/web_engine/webkit.rs`, used today only by
  account deletion) empties files on disk and the engine's resource cache. The
  heap it would leave exactly as it was.
- Returning the memory is already a feature: parking ends the rendering
  process before the view is dropped (`FR.5.2`), and roadmap item 05 measured
  that the memory really does come back. `Ctrl`+`P` then `Ctrl`+`S`
  (`FR.26.3`) is the one-click restart in two keystrokes, so a separate
  `Restart` action would have added a menu item for a path that exists.
- The owner's one stated need was the memory, and cache clean-up was the
  guessed remedy. With the remedy shown not to work, nothing was left that a
  cache clear answers: no game has shipped stale or broken assets that a
  reload did not fix.
- If the growth is to be bounded without the user noticing it first, that is
  an automatic recycle of the rendering process, which
  `docs/prompts/04-bound-leaky-renderer-memory.md` already proposes and which
  wants its own requirements. It would come back as that, not as a cache
  action.
