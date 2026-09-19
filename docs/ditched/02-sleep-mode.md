# 02 — A sleep mode that keeps games running while the interface stops rendering

**Ditched:** 2026-09-19 · **Estimate:** 3

## Idea

One named state, entered and left deliberately, that puts the whole
application into a configuration where nothing is drawn and every game keeps
ticking at full speed — so the machine could be left for hours spending its
cycles on the games rather than on presenting them. Entry, exit, scope and its
relationship to keep-awake and parking were the open questions
(`docs/prompts/07-sleep-mode-requirements.md`).

## Why not

- Minimising the window with keep-awake on already does it. The owner observed
  on 2026-09-19 that a minimised window stops the application's GPU work
  entirely, and roadmap item 04's `## Measured` section records that a
  minimised window is the one case where keep-awake matters, so the two
  together are the wanted state. There was nothing left for a mode to add.
- Keep-awake is already a deliberate per-account choice (`FR.6.1`–`FR.6.4`;
  `FR.1.9` in Platform Support for Windows). On minimise the shell already
  marks every account without it as background to its engine and every account
  with it as visible — `background_for` in
  `crates/idle-manager-shell/src/web_view.rs`, applied by `apply_minimised` in
  `crates/idle-manager-shell/src/window/imp.rs`. A mode that flipped every flag
  on would override that choice for no gain.
- Hiding the toplevel instead of minimising it buys nothing. An unmapped view is
  hidden to the engine exactly as a minimised one is (`FR.3.3`), so it would
  lean on the same keep-awake mechanism while adding a way back — a tray item
  or a single-instance relaunch — that minimising gets for free from the desktop.
- Returning memory is parking (`FR.5.2`), which stops the game. A mode cannot
  both keep a game running and hand its memory back; that axis already has its
  own feature and its own control.
- If a future measurement shows a minimised window still costs processor time
  on presentation, that comes back as a measured finding against item 04 or a
  new item, not as a mode.
