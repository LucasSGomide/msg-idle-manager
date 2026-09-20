# 02 — Capturing a frame, running a script and waking a view on both engines

**Roadmap:** [13](../../roadmap/13-phone-operation/README.md) · **Scope:** front-end · **Depends on:** —

## Context

Each game in this application is shown by a web engine: WebKitGTK on Linux,
Microsoft Edge WebView2 on Windows. Every call into either engine already goes
through one small module so the rest of the program never names an engine.
Today that module can load, reload, zoom and terminate a page, but it cannot
take a picture of one, run a line of script inside one, or wake one that the
engine has put to rest because the window is minimised.

A later slice streams a game to a phone and delivers the phone's taps. It
needs exactly those three abilities, so this slice adds them behind the seam
with one implementation per engine. Taking a picture asks the engine for a
snapshot of the page as it is right now. On Linux that comes back as raw
pixels; on Windows the engine hands back a compressed JPEG directly. Running a
script hands the engine a string of JavaScript to execute in the page. Waking
a view means telling the engine to treat the page as if it were being looked
at, without reloading it, for as long as the phone is watching, and then
putting it back exactly as the owner's own per-account choice says.

The Linux wake needs one change to how the keep-awake script works. That
script keeps a game's animation loop running while the page is hidden. Today
it is only injected into accounts whose keep-awake switch is on, and turning
the switch requires a reload. After this slice the script is injected into
every page but stays dormant until a flag turns it on, and the flag can be set
at run time from the outside. Accounts with keep-awake on behave exactly as
before; the difference is invisible until the phone needs it.

The slice also answers the item's sharpest open question with a file on disk.
Whether either engine returns a current picture of a page while the window is
minimised has never been measured. A debug switch makes the program write
every captured frame into a folder, so a developer can minimise the window,
wait, and look at the pictures on each system.

## User experience

- **Flow** — Minimise the window → accounts with keep-awake on keep running
  at full rate; the others may be slowed by the engine; restoring shows every
  game where it was (unchanged from before this slice).
- **States** — Nothing on screen changes in this slice. The debug switch
  writes files and draws nothing.

## Technical details

- **Architecture** — `web_engine.rs` gains three crate-private calls on
  `EngineView`: `capture_frame(done: impl FnOnce(Result<Frame, EngineCaptureError>) + 'static)`,
  `run_script(source: &str)` and `set_watched(on: bool, keep_awake: bool, minimised: bool)`;
  `EngineCaptureError { reason: String }` is engine-neutral like
  `EngineDeleteError`.
- **Architecture** — `web_engine/webkit.rs`: `capture_frame` calls
  `WebView::snapshot(SnapshotRegion::Visible, SnapshotOptions::NONE, None, cb)`,
  downloads the `gdk::Texture` to RGBA on the main context and returns
  `Frame::Rgba { width, height, stride, bytes }`; `run_script` calls
  `evaluate_javascript(source, None, None, None, cb)` and logs a failure with
  the session field; `set_watched(true, ..)` calls
  `apply_keep_awake(view, true, id)` with no reload and
  `run_script("__idleManager.setAwake(true); __idleManager.setHiddenFrameInterval(33)")`;
  `set_watched(false, keep_awake, ..)` calls `apply_keep_awake(view, keep_awake, id)`
  and `run_script` restoring `setAwake(keep_awake)` and the 250 ms interval
  (rules 8, 10; code standards rule 18 names the WebKit constraint).
- **Architecture** — `web_engine/webview2.rs` and `webview2/ffi.rs`:
  `capture_frame` calls `ICoreWebView2::CapturePreview` with
  `COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_JPEG` into a memory `IStream`,
  reads it back as `Frame::Jpeg` and hops to the main context with
  `invoke_local`; `run_script` calls `wry::WebView::evaluate_script`;
  `set_watched(true, ..)` calls `set_background(false)`, `set_watched(false,
  keep_awake, minimised)` calls `set_background(background_for(minimised, keep_awake))`
  (code standards rule 28: `unsafe` stays inside `ffi.rs` with `// SAFETY:`).
- **Architecture** — `resources/js/keep-awake.js` is injected into every
  page at document start by both engines' script sets and reads a runtime
  flag: it defines `window.__idleManager = { setAwake(on), setHiddenFrameInterval(ms) }`,
  intercepts `requestAnimationFrame` only while `document.hidden && awake`,
  and a one-line prelude script sets `awake = true` before the page runs for
  a keep-awake account, so item 04's behaviour is unchanged; `install_script_set`
  and `rebuild_script_set` change accordingly and `set_keep_awake` keeps its
  reload.
- **Architecture** — `web_view.rs`: `SessionView` gains
  `set_watched(on, minimised)` forwarding to the live view with the remembered
  keep-awake flag (a no-op while parked), and `run_script` and `capture_frame`
  pass-throughs; `WATCHED_FRAME_INTERVAL_MS = 33` and the existing
  `HIDDEN_FRAME_INTERVAL_MS` are named constants (code standards rule 5).
- **Architecture** — the debug switch: when `IDLE_MANAGER_DUMP_FRAMES=<dir>`
  is set, `SessionView::start` arms a `glib::timeout_add_local` every 2 s
  after the first paint that calls `capture_frame` and writes
  `<dir>/<session id>-<n>.{ppm|jpg}` through `gio::spawn_blocking`, logging
  each write; off by default like `FR.19.6`'s diagnostics.
- **Code standards** — no new `pub` beyond `pub(crate)` (rule 8); every
  failure is logged with `tracing` fields, never dropped (rules 14, 15).

## Acceptance criteria

- [x] `(unit)` the runtime-flag shim script, evaluated in a test harness or
      inspected as a string, exposes `__idleManager.setAwake` and
      `__idleManager.setHiddenFrameInterval` and intercepts
      `requestAnimationFrame` only while hidden and awake
- [x] `(unit)` the keep-awake prelude sets the flag on and the plain script
      set leaves it off, checked on the script set builder for both engines
      (`cfg(any(windows, test))` for the Windows half)
- [x] `(unit)` `background_for` and the `set_watched(false, ..)` restore path
      agree: watched-off with keep-awake off and minimised yields background
- [x] `(integration)` `make verify` and `make windows-check` pass
- [x] `(integration)` `make windows-check` passes with the workspace
      `unsafe_code` lint still denied, so the `CapturePreview` and `IStream`
      calls compile only inside `web_engine/webview2/ffi.rs`'s scoped allow,
      each `unsafe` block carrying a `// SAFETY:` line (code standards rule 28)
- [x] `(manual)` on Linux with `IDLE_MANAGER_DUMP_FRAMES=/tmp/frames`, a
      running game writes a new frame every 2 s; after minimising the window
      for 30 s the frames written during that time differ from each other
      (or the item's first Blocker is recorded as failed, with the files kept)
- [ ] `(manual)` in the Windows VM with the same switch, the same check
      produces JPEG files that differ over 30 s minimised
- [ ] `(manual)` on Linux, an account with keep-awake off keeps ticking in the
      repository's `vischeck` page while minimised after `set_watched(true)`
      is triggered from a debug key, and stops again after `set_watched(false)`
- [ ] `(manual)` toggling keep-awake from a row's menu still reloads the page
      and the account behaves as item 04's test script describes

## References

- [Roadmap item](../../roadmap/13-phone-operation/README.md) — Front-end
  "The engine seam grows three calls" and "`web_view.rs`"; Technical
  References: WebKitGTK's `webkit_web_view_get_snapshot` paints in the web
  process independently of the widget's on-screen frame, and WebView2's
  `ICoreWebView2::CapturePreview` returns PNG or JPEG into an `IStream` under
  `--disable-backgrounding-occluded-windows`; Blockers 1–3
- [`openapi.json`](../../roadmap/13-phone-operation/openapi.json) —
  `BinaryFrame` for the shapes the capture feeds
- [Sequence diagrams](../../roadmap/13-phone-operation/sequence-diagrams.md)
  — the `publish_frame` and `run_script` arrows in `GET /ws`
- [Wireframes](../../roadmap/13-phone-operation/wireframes/) — the whole item
- [`docs/requirements.md`](../../requirements.md) — Remote Access `FR.1.4`,
  `FR.4.3`; Session Management `FR.6.1`–`FR.6.3`, `FR.19.6`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 8, 14, 15,
  18, 28
- [`docs/design.md`](../../design.md) — nothing drawn; no rule exercised

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The Windows `(manual)` step
runs in the VM ([`docs/windows-vm.md`](../../windows-vm.md)).
