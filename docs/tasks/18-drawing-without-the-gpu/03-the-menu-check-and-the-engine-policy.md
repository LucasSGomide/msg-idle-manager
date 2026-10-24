# 03 — The menu check and the engine policy

**Roadmap:** [18](../../roadmap/18-drawing-without-the-gpu/README.md) · **Scope:** front-end · **Depends on:** 01

## Context

Each account in the application's sidebar has a three-dot menu holding its
settings: park or start, keep running when hidden, and muted. This slice adds a
fourth, a check item labelled "Draw without the GPU", and makes it do something.

Ticking it on an account whose game is running tells the web engine to draw that
account's page with the processor rather than the graphics chip, and reloads the
page once, because the engine decides how a page is drawn when the page is
built. While the page reloads the row shows its usual reloading mark, and it
returns to live at the first painted frame, exactly as changing the keep-awake
setting already behaves. Unticking it reverses both. Ticking it on a parked
account only records the choice; the page is built that way when the account is
started. The choice survives a restart because the earlier slice saves it.

On Linux the engine's per-page switch is a drawing policy that can be set to
"never use hardware acceleration". On the 2026-10-24 measurement it kept the
Lux Idle hunting list at almost no graphics memory, where the default drawing
path claimed gigabytes in under a second. On Windows the engine is the same one
Microsoft Edge uses, the fault was never seen there, and its only equivalent
switch applies to every account at once, so the Windows build shows no menu item
and its engine ignores the stored flag.

This is its own slice because it is all interface code: the menu, the window's
handler, the object that holds each account's engine view, and the engine seam.
It needs the stored flag from the first slice and nothing from the preset
slice, so it can be built beside it. It also adds a note to the memory budget
document that graphics buffers never show in the application's memory footer.

## User experience

- **Entry** — the ⋯ menu on an account's sidebar row (button, right-click or
  `Shift`+`F10`); first section `Park`/`Start`, `Keep running when hidden`,
  `Muted`, `Draw without the GPU` (Linux build only).
- **Flow** — tick it on a live account → menu closes, the row shows the
  reloading mark, the page reloads once drawn by the processor, the row returns
  to live at the first paint; untick → the same single reload back.
- **Flow** — tick it on a parked, queued or starting account → stored, shows
  ticked, the page is built that way when it starts; park/start and relaunch
  keep it.
- **States** — ticking an already-ticked item (or the reverse) changes nothing:
  no reload, no save.
- **States** — Windows: no menu item; the stored key is kept and ignored.
- **Pattern** — stateful check item reporting intent, never setting its own
  state (design rule 5; `session_sidebar/imp.rs:932`); reload through
  `Starting` like keep-awake (design rule 1; `window/imp.rs:2311`); no row
  mark, no accelerator (design rules 6 and 27).

## Technical details

- **Architecture** — no engine type leaves `web_engine/`; the handler reports
  intent and redraws from the book (architecture rules 8, 10, 12).
- **Code standards** — `AccountSettings` in `web_view.rs` gains
  `pub is_software_rendered: bool`, filled from `session.is_software_rendered()`
  at the three construction sites in `window/imp.rs`.
- **Code standards** — `webkit.rs`: `apply_account_settings` sets
  `HardwareAccelerationPolicy::Never` when on and leaves the policy alone when
  off, with a comment recording the measurement (rule 18);
  `EngineView::set_software_rendered(on, id)` sets `Never` or the default read
  from a fresh `webkit6::Settings::new()`, logs at `debug`, and reloads.
- **Code standards** — `webview2.rs`: `EngineView::set_software_rendered` logs at
  `debug` and does nothing else; the build path ignores the field, with a
  comment saying why.
- **Code standards** — `SessionView::set_software_rendered(&mut self, on) ->
  Option<&EngineView>` writes the held settings and applies to a live view.
- **Design** — `session_sidebar`: an `is-software-rendered` row property, a
  stateful `software-rendering` action reporting `(id, requested)` through
  `connect_software_rendering_toggled`, and the menu item after `Muted` under
  `#[cfg(target_os = "linux")]` (design rules 5, 21, 27).
- **Code standards** — `window/imp.rs`: `toggle_software_rendered` mirrors
  `toggle_keep_awake` (book set, no-op return, redraw, save, holder set, arm
  `connect_painted` → `finish_starting` on a live view).
- **Code standards** — `docs/memory-budget.md` gains a paragraph that GPU
  buffers are invisible to the PSS footer, citing roadmap item 18.

## Acceptance criteria

- [x] `(unit)` the row menu model for a Linux build carries `Draw without the GPU` after `Muted` in the first section, with no accelerator
- [x] `(unit)` a parked holder remembers the flag and builds its next view's settings with it
- [x] `(unit)` `AccountSettings` from a software-rendered session carries the flag at every construction site (restore, dormant add, new account)
- [x] `(manual)` with the flag on, clicking the game's `GO HUNT` keeps the web process's GPU objects flat and the machine responsive for two minutes — whether the flag was saved before launch (14 MiB throughout) or turned on in the running page (847 → 418 MiB at the reload, then flat)
- [x] `(manual)` a second request for the value already held does not reload the page
- [x] `(manual)` relaunching with `software_rendering = true` saved on the account builds its page without the GPU (web process under 50 MiB of GPU objects)

## References

- [Roadmap item](../../roadmap/18-drawing-without-the-gpu/README.md) — the full picture, including the bisect table
- [Wireframes](../../roadmap/18-drawing-without-the-gpu/wireframes/) — the row menu this slice renders
- [Design](../../design.md), [Architecture](../../architecture.md), [Code standards](../../code-standards.md)

## Implement with

Direct implementation, TDD where a unit test reaches; the engine behaviour is
checked by hand against the real game with the measurement harness.
