# 01 — One web-engine seam, on Linux, with no visible change

**Roadmap:** [12](../../roadmap/12-windows-support/README.md) · **Scope:** front-end · **Depends on:** —

## Context

This application runs several accounts of browser idle games side by side in one
GTK 4 window. Each account's game page is shown by WebKitGTK, a web engine that
exists only on Linux. The goal of the larger piece of work is to also run on
Windows, where a different engine, Microsoft Edge WebView2, will show the pages.

Today the interface code calls WebKit by name in six files: the window, the grid
of game places, the queue that starts accounts one at a time on launch, the
per-account view holder, the account-deletion steps and the library's shared
setup. A second engine cannot be slotted in while that is true.

This slice gathers every one of those calls into one new module inside the
interface crate. The module offers a small, engine-neutral set of types: one for
the process-wide engine state, one for an account's long-lived engine profile,
and one for the view of a single start. The rest of the interface then talks
only to those types. On Linux the module's only implementation wraps exactly
the WebKit code that exists today, moved rather than rewritten. The per-account
keep-awake switches, the memory-pressure settings and the measured
clear-then-remove order for deleting an account all stay as they are.

Nobody using the application sees any difference. That is the point, and why
this is a slice on its own: it is a pure move on Linux, provable with the
existing tests and a short hand-run check, before any Windows code exists. Every
later slice builds on it, so it runs alone.

## User experience

- **Flow** — Every existing flow (launch and restore, start, park, keep-awake,
  zoom by key and wheel, rename, drag to rearrange, workspaces, delete) behaves
  exactly as before on Linux.
- **States** — Loading cover, stopped panel, parked and queued places are
  unchanged.
- **Pattern** — No screen changes (design rules 1–13 unaffected).

## Technical details

- **Architecture** — add crate-private `web_engine.rs` in
  `crates/idle-manager-shell/src/` with `web_engine/webkit.rs` under
  `#[cfg(target_os = "linux")]`. `web_engine.rs` re-exports `EngineShared`,
  `EngineProfile` and `EngineView` from whichever backend is compiled. No port
  goes in `idle-manager-core`: the choice is compile-time and never faked
  (architecture rules 3, 6).
- **Architecture** — `EngineView` offers `widget() -> gtk::Widget`, `load_uri`,
  `reload`, `set_zoom(ZoomLevel)`, `terminate`, `grab_focus`,
  `connect_painted(f)` (fires once on the first `Committed` or `Finished`), and
  `connect_terminated(f)`. `EngineProfile` builds an `EngineView` from user
  agent, zoom, keep-awake and the script set, and offers
  `clear_website_data(done)`. `EngineShared` is the shared `WebContext` and the
  memory-pressure settings now in `lib.rs:43-180`.
- **Architecture** — add `EngineView::set_background(bool)` as a no-op on Linux,
  with a `///` saying WebKit's own hidden-page handling covers it. A later slice
  gives it meaning on Windows.
- **Architecture** — `web_view.rs`'s `SessionView` keeps liveness and lifecycle
  and holds an `EngineProfile` plus `Option<EngineView>`. `view()` returns
  `Option<&EngineView>`. All `webkit6` imports and helpers (`build_content_manager`,
  `install_script_set`, `apply_keep_awake`, `KeepAwakeFeatures`, popup handling)
  move to `web_engine/webkit.rs`.
- **Architecture** — `session_grid.rs`, `session_grid/imp.rs` (`add_session`,
  `register_slot`, `attach_view`, `hide_cover_once_painted`, the `downcast::<WebView>`
  at `:469`), `start_queue.rs` (`arm_next`) and `window/imp.rs`
  (`start_session`, the load-event matches at `:1293` and `:1377`) take
  `EngineView` and `connect_painted` instead of `WebView` and `LoadEvent`.
  `account_deletion.rs` calls `EngineProfile::clear_website_data` and then drops
  the profile, keeping item 11's order and quarter-second wait.
- **Code standards** — this is a move: keep existing comments with the code they
  explain (rules 16, 18), give each new crate-private type a `///` contract
  (rule 17), and add no `unsafe`.
- **Naming** — module files are snake_case (rule 2). Types are named for the
  thing, not the pattern (rule 9), and none repeats the module name (rule 6).
- **Code standards** — `webkit6` moves under
  `[target.'cfg(target_os = "linux")'.dependencies]` in
  `crates/idle-manager-shell/Cargo.toml`, keeping its `v2_42` feature and
  comment.

## Acceptance criteria

- [ ] `(integration)` `rg -l 'webkit6' crates/idle-manager-shell/src` lists only
      `web_engine/webkit.rs`
- [ ] `(integration)` `make verify` passes with no test removed or ignored
      compared with `main`
- [ ] `(unit)` the existing `start_queue.rs` next-up tests pass unchanged against
      the engine-neutral queue
- [ ] `(integration)` `cargo tree -p idle-manager-shell --target
      x86_64-unknown-linux-gnu` still lists `webkit6` 0.6 with `v2_42`
- [ ] `(manual)` on Linux, launch restores the saved workspace one account at a
      time, and each place shows the loading cover with the account's name until
      its page paints
- [ ] `(manual)` on Linux, `Ctrl`+`+` and `Ctrl`+wheel over a focused game each
      step the zoom and flash the percentage, as before
- [ ] `(manual)` on Linux, parking then starting an account, killing its
      `WebKitWebProcess`, and deleting a throwaway account each behave as they
      did before this slice
- [ ] `(manual)` on Linux, a keep-awake account keeps advancing while the window
      is minimised (item 04's existing test-script step, re-run)

## References

- [Roadmap item](../../roadmap/12-windows-support/README.md) — Front-end, "The
  engine seam"
- [Wireframes](../../roadmap/12-windows-support/wireframes/) — none change in
  this slice
- [`docs/architecture.md`](../../architecture.md) — rules 3, 6, 8, 10, 12
- [`docs/code-standards.md`](../../code-standards.md) — rules 9, 16–18
- [`docs/naming.md`](../../naming.md) — rules 2, 6, 9
- [`docs/design.md`](../../design.md)

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above on a branch named `feat/12-windows-support`, then run
`/msg-roadmap-sync`.
