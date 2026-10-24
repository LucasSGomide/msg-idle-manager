# 18 — Drawing one account without the GPU

Sliced the way item 17 was: the flag lands in the domain and the workspace file
first, then the preset key and the menu-and-engine half build on it.

**Waves.** 01 runs alone. 02 (preset key and the shipped Lux Idle file, store and
core preset code only) and 03 (sidebar, window, holder and engine seam, shell
only) both depend on 01 and touch no shared file — safe to run in parallel.
04 holds the checks that need a person at the window (the real menu click,
park and start, adding from the preset) and runs after 02 and 03; it was split
out at merge time so 01–03 could land accepted while it stays open.

| # | Task | Scope | Depends on | Criteria | Status |
|---|---|---|---|---|---|
| [01](01-the-flag-in-the-domain-and-the-workspace-file.md) | The flag in the domain and the workspace file | back-end | — | 9/9 | done |
| [02](02-the-preset-key-and-the-lux-idle-preset.md) | The preset key and the shipped Lux Idle preset | back-end | 01 | 6/6 | done |
| [03](03-the-menu-check-and-the-engine-policy.md) | The menu check and the engine policy | front-end | 01 | 6/6 | done |
| [04](04-the-check-at-the-window.md) | The check at the window | front-end | 02, 03 | 0/3 | not-started |

## As built

- **Which build, which game.** The owner's freezes on 2026-10-24 were the
  Linux release AppImage (`~/Downloads/IdleManager.AppImage`, built
  2026-09-24) on the Intel Arc 130V/140V laptop: WebKitGTK 2.52.6, Mesa
  26.0.8, the `xe` driver, kernel 7.0. Lux Idle (`https://luxidle.com.br/jogar`)
  is a 2D-canvas game; its `GO HUNT` button opens a modal list of ninety
  hunting grounds in an 11,398-pixel scroller, and every ground the level-8
  character cannot enter yet is drawn through
  `.hunt-card:not(.boss).locked { filter: grayscale(.5) brightness(.7) }`.
- **Root cause.** WebKitGTK's GPU drawing of those filtered cards. After the
  click the rendering process claimed `xe` buffers at ~10 GB/s (0.9 → 1.6 →
  4.2 → 5.8 GiB across 0.4 s, run r08); the kernel log of the first crash
  shows `gpu_active` at 27 GiB with every process's RSS small, and the OOM
  killer taking Edge, Bitwarden and other bystanders. None of it is PSS, so
  the memory footer saw nothing. A bare WebKitGTK window with none of the
  application's scripts, settings, memory-pressure limits or renderer choice
  spikes the same way (r07); removing only that one CSS rule from the page
  stops it (r19). Edge was flat because it is a different engine.
- **The bisect** is the table in the roadmap item's Technical References: the
  application's own additions (scripts, WebGL, 2D-canvas acceleration, painting
  threads) changed nothing; `WEBKIT_DISABLE_DMABUF_RENDERER=1` and per-view
  `hardware-acceleration-policy = never` were each flat; Skia CPU painting
  bounded it at a 2.5 GiB PSS burst. Per view `never` won because it is
  per account and costs ~0.15 cores for this game while saving ~830 MiB.
- **What shipped.** A per-account `Draw without the GPU` check in the row's ⋯
  menu (Linux only), stored as `software_rendering = true` on the account's
  `sessions.toml` entry, settable from a preset, and a shipped
  `presets/lux-idle.toml` with it on. Built with the flag, Lux Idle's process
  holds 14 MiB of GPU objects with the hunting list open (r24); turned on in a
  running page it stops the growth at the reload but keeps the 418 MiB it
  already held until the next park/start (r27).
- **Reproduction for an upstream report.** `filtered-cards-repro.html` in the
  roadmap item folder builds 90 cards, 84 under the same filter, 30 s after
  load: +457 MiB of GPU objects in a bare WebKitGTK window against +54 MiB
  without the filter (r15, r16), +780 MiB and creeping with animated sprites
  (r18). It shows the cost but not yet the runaway; reducing the game's page
  further is the remaining work for the report.
