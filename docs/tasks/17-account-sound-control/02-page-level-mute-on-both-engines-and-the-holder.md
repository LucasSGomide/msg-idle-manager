# 02 — Page-level mute on both engines and the holder

**Roadmap:** [17](../../roadmap/17-account-sound-control/README.md) · **Scope:** front-end · **Depends on:** —

## Context

Each game in this application is drawn by the operating system's web engine:
WebKitGTK on Linux and Microsoft Edge WebView2 on Windows. The interface crate
hides which engine is in use behind a small seam, so the rest of the interface
only ever talks to an engine-neutral "profile" that builds a view and a "view"
that shows a page. Beside that seam sits a holder, one per account, that keeps
the account's settings and the view it currently has, so a parked account can
be started again with the same settings.

This slice teaches the seam and the holder to mute. Both engines already have a
switch that silences everything a page plays, media elements and synthesised
audio alike, and flips in place without reloading. On Linux it is a property on
the view. On Windows it lives on a newer version of the engine's interface than
the application talks to today, so the code asks for that version and, when an
older runtime cannot provide it, logs one warning naming the account and the
runtime version and does nothing else; the choice is kept and applies when a
capable runtime next builds the page.

The holder gains the flag too. It is given the flag when it is created, so an
account restored from disk carries it before any page exists; it passes the
flag in when it builds a view, so a muted account's first sound is already
silent; and it has a setter that flips the live view in place and remembers the
value for the next build. The setter has nothing to do with whether the window
is minimised, so it does not take or touch the visibility state the keep-awake
setter does.

It is its own slice because it compiles against a plain boolean and needs no
menu and no domain flag to be correct. The three places in the window that
create holders pass `false` for now; the closing slice replaces that with the
account's stored flag.

## User experience

- **Flow** — Mute a live account → the game goes silent at once, the page keeps
  running, no reload.
- **Flow** — Mute an account with no page → the flag is remembered; the page is
  built muted the moment it starts.
- **States** — The engine refuses the mute call (a WebView2 runtime older than
  1.0.864): the page is not silenced and one warning naming the account and the
  runtime version goes to the log. No message strip.
- **States** — Toggling mute never shows the reloading mark and never disturbs a
  phone watching the account.

## Technical details

- **Architecture** — `docs/architecture.md` "Where a change goes": anything
  naming `WebKitGTK` or `WebView2` stays under
  `crates/idle-manager-shell/src/web_engine/`; `make arch-check` must still hold.
  Rule 10: every call on the GTK main context.
- **Architecture** — `web_engine/webkit.rs`: `EngineProfile::build_view` gains
  `muted: bool` and passes `.is_muted(muted)` to the `WebView::builder()`;
  `EngineView::set_muted(&self, on: bool, id: &SessionId)` calls
  `self.view.set_is_muted(on)` and logs at `debug` with the `session` field.
- **Architecture** — `web_engine/webview2.rs`: `build_view` gains the same
  `muted: bool`, stored on a new `muted: bool` field of `PendingView` in
  `webview2/host.rs`; `EngineView::set_muted` delegates to a new
  `EngineHost::set_muted(on)`. In `webview2/host/imp.rs` the setter applies to
  the live view when one exists and otherwise writes onto the pending view so a
  toggle before realisation is not lost; the post-creation path that applies
  `pending.zoom` also applies `pending.muted` when `true`.
- **Architecture** — `webview2/ffi.rs`: `pub(super) fn set_muted(webview:
  &ICoreWebView2, muted: bool) -> windows::core::Result<()>` casting to
  `ICoreWebView2_8` and calling `put_IsMuted`, following `delete_profile`'s cast
  pattern. A failure is one `tracing::warn!` with `session` and `runtime =
  wry::webview_version()` (code-standards rules 14, 15), nothing more.
- **Architecture** — `web_view.rs`: `SessionView` gains `muted: bool` beside
  `keep_awake`; `dormant` and `new` take `muted: bool`; `start` passes
  `self.muted` to `build_view`; `pub fn set_muted(&mut self, on: bool) ->
  Option<&EngineView>` remembers `on`, applies it to the live view, returns it.
  No `minimised` argument, no `set_background`.
- **Architecture** — `window/imp.rs`: the three `SessionView::dormant` /
  `SessionView::new` call sites pass `false` so the crate compiles; task 03
  replaces the literal with the book's flag.
- **Code standards** — rules 17 (`///` contracts naming `FR.28.2`, `FR.28.3`),
  18 (comment the `ICoreWebView2_8` minimum runtime as the constraint), 28 (the
  only `unsafe` is the COM call inside `ffi.rs`, with a `SAFETY` comment like
  its neighbours).

## Acceptance criteria

- [ ] `(integration)` `cargo test -p idle-manager-shell` passes on Linux with `muted` threaded through `build_view`, `dormant`, `new`, `start` and `set_muted`
- [ ] `(integration)` `make fmt-check` and `make clippy` pass: no `unwrap`, no discarded `Result`, the refusal warning carries `session` and `runtime` fields
- [ ] `(integration)` `make arch-check` holds: `set_is_muted`, `is_muted(` and `ICoreWebView2_8` appear only under `web_engine/`
- [ ] `(integration)` `make windows-check` compiles `ffi::set_muted`, `PendingView::muted` and `EngineHost::set_muted` (needs the LLVM cross toolchain; left unticked where it is absent, as the roadmap item's blocker records)

## References

- [Roadmap item](../../roadmap/17-account-sound-control/README.md) — the full picture
- [Wireframes](../../roadmap/17-account-sound-control/wireframes/) — the menu that will drive this seam
- `docs/architecture.md` · `docs/code-standards.md` · `docs/design.md`

## Implement with

A fresh implementation session with `Scope: front-end`; `make verify` before acceptance.
