# 02 — Building for Windows, and games showing there

**Roadmap:** [12](../../roadmap/12-windows-support/README.md) · **Scope:** full-stack · **Depends on:** 01

## Context

The application already has one engine-neutral module that every part of its
interface uses to show a game page, and on Linux that module is backed by
WebKitGTK. This slice adds the Windows backend, backed by Microsoft Edge
WebView2, the browser component every Windows 10 and 11 machine already has. It
makes the whole program compile for Windows.

After this slice a Windows user who builds the program sees the normal window,
the sidebar and the saved accounts, and each place shows its game page. Two
accounts of the same game can both be logged in, because every account gets its
own named profile inside a single shared engine. Sharing one engine matters for
cost: every account uses the same browser process and the same graphics process.

The WebView2 view is a separate native window placed inside the GTK window. A
new, invisible GTK widget stands where the game goes, and on every resize it
moves and sizes the native view to cover exactly its own area. If WebView2 is
missing, the program shows a single dialog saying so and quits without touching
any saved data.

Development happens on Linux, so this slice also adds a check that type-checks
and lints the Windows build from Linux. That check becomes part of the project's
full verification command, so a Linux change cannot quietly break Windows. It
also records two documentation changes. The stack and architecture guides now
name both systems. The coding rule that forbids unsafe code now allows exactly
one small module per crate for Windows system calls.

Page scripts, sign-in popups, zoom, crash handling, keep-awake, the drawn
overlays, deletion and memory figures come in later slices. This one is the
smallest thing that proves the approach works, and it is checked in a Windows 11
virtual machine that runs inside a container on the Linux development machine.

## User experience

- **Entry** — In the Windows VM the developer runs the `idle-manager.exe` that
  `make windows-build` produced on Linux, copied from the shared `Z:` drive
  (the release zip comes later).
- **Flow** — Launch → the saved workspace restores one account at a time
  exactly as on Linux; each place shows its game page, sized to the place, and
  follows window resizes, moves and layout switches.
- **States** — Engine missing: a lone dialog headed "Microsoft Edge WebView2
  Runtime is required", a detail line with the reason and
  `https://developer.microsoft.com/microsoft-edge/webview2/`, and one `Quit`
  button. No account data is read or written.
- **New pattern** — the fatal start-up dialog for a missing system component.
  Design rule 9's strip needs a window, and this failure comes before one
  exists. The design doc owes a rule once this ships.

## Technical details

- **Architecture** — root `Cargo.toml` declares `wry = "0.57"`, `webview2-com =
  "0.39"`, `gdk4-win32 = { version = "0.11", features = ["win32"] }`,
  `raw-window-handle = "0.6"` and `windows = "0.62"`. The shell takes them under
  `[target.'cfg(windows)'.dependencies]` only; `wry` must never build on Linux
  (it pulls GTK 3 `webkit2gtk`). `main.rs` gates the `GSK_RENDERER` re-exec and
  `CommandExt` behind `cfg(target_os = "linux")`.
- **Architecture** — `web_engine/webview2.rs`, under `cfg(windows)`:
  - `EngineShared` owns one `wry::WebContext` over the new
    `store::paths::engine_data_root()` (`<data root>/idle-manager/webview2/`).
  - The first view is built with `BROWSER_ARGS` (wry's default
    `--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection` plus
    `--disable-backgrounding-occluded-windows`). Every later view reuses its
    environment with `with_environment`.
  - `EngineProfile` passes `with_profile_name(session_id)` and asserts the
    64-character `[A-Za-z0-9._ -]` limit.
  - `with_user_agent` is set from the preset, and `set_zoom` calls
    `webview.zoom`.
- **Architecture** — `web_engine/webview2/host.rs` + `host/imp.rs` define
  `EngineHost`, a GTK widget that draws nothing, which `EngineView::widget()`
  returns:
  - On realize it takes the toplevel `HWND` via
    `gdk4_win32::Win32Surface::handle` and calls
    `WebViewBuilder::build_as_child`.
  - On every allocation and on `notify::scale-factor` it sets the view's
    bounds to the widget's `compute_bounds` against the window root, plus
    `Native::surface_transform`, as a logical `Rect`. That math lives in a
    pure `fn place_bounds(...)`.
  - An off-grid host sits outside the grid, so its view is clipped, never made
    invisible.
  - `collapse()` sets zero-size bounds and `restore()` reapplies the last
    bounds.
  - Dispose drops the `wry::WebView`.
- **Architecture** — `connect_painted` fires on wry's page-load `Finished`
  event. `connect_terminated` and `clear_website_data` are stubbed to log at
  `warn` with `TODO(12)` until their slices land (code standards rule 20).
  Views are created on the GTK main context (architecture rule 10).
- **Code standards** — `web_engine/webview2/ffi.rs` is the only shell module
  allowed `#[allow(unsafe_code)]`. It holds the `HasWindowHandle` wrapper
  (`WindowHandle::borrow_raw`), with a `// SAFETY:` line on every `unsafe`
  block, and exposes only safe functions. Amend `docs/code-standards.md`
  rule 28 in the same commit to name this second allowed case.
- **Architecture** — `lib.rs` adds `runtime_version() -> Result<String,
  String>` (`GetAvailableCoreWebView2BrowserVersionString`) and
  `show_missing_engine(app, reason)` (`gtk::AlertDialog`), both
  `cfg(windows)`. `main.rs` calls them before opening any store and returns an
  error on failure.
- **Architecture** — `make windows-check`:
  - It runs `cargo clippy --workspace --all-targets --target
    x86_64-pc-windows-msvc -- -D warnings` with `PKG_CONFIG_ALLOW_CROSS=1`,
    `PKG_CONFIG_PATH=target/windows-sdk/gtk/lib/pkgconfig` and
    `--define-prefix`. If a build script needs the SDK, it falls back to
    `cargo-xwin` and records why.
  - `make bootstrap` adds the rustup target and unpacks the pinned
    `GTK4_Gvsbuild_<ver>_x64.zip` into `target/windows-sdk/gtk/`.
  - `verify` gains `windows-check`.
  - `scripts/arch-check.sh` forbids `wry webview2-com gdk4-win32` in core,
    store and metrics, and `windows` in core and store. It runs a second
    `cargo tree` pass with `--target x86_64-pc-windows-msvc`.
- **Architecture** — `make windows-build` runs `cargo xwin build --locked
  --target x86_64-pc-windows-msvc` (add `--release` with `PROFILE=release`) and
  copies the executable and GTK's `bin/*.dll` into `dist/idle-manager-dev/`, which
  the VM sees as `Z:\idle-manager-dev`. `make bootstrap` installs `cargo-xwin`.
  `docs/stack.md` links `docs/windows-vm.md`, and `docs/stack.md` "Target", "GUI" and "System packages", and
  `docs/architecture.md`'s shape diagram and "Where a change goes" table name
  both systems and both engines.

## Acceptance criteria

- [ ] `(integration)` `make windows-check` exits 0 on the Linux dev machine
- [ ] `(integration)` `cargo tree -p idle-manager-shell --target
      x86_64-unknown-linux-gnu -e normal` lists no `wry`, `webview2-com` or
      `gdk4-win32`
- [ ] `(integration)` `make arch-check` fails when `wry` is temporarily added to
      `idle-manager-store`'s Windows dependencies, and passes once it is removed
- [ ] `(unit)` the profile-name check accepts every identifier the session
      book mints and rejects a 65-character name and a name containing `/`
- [ ] `(unit)` `place_bounds` adds the surface transform to the widget origin
      and keeps the widget's size, and an origin outside the window stays
      outside
- [ ] `(integration)` `make windows-build` produces
      `dist/idle-manager-dev/idle-manager.exe` on the Linux dev machine, and
      `engine_data_root()` ends in `idle-manager/webview2`
- [ ] `(manual)` in the Windows VM, launching with two saved accounts of the same game
      restores both, and a login in one does not log the other in
- [ ] `(manual)` in the Windows VM, with four live accounts, Task Manager shows one
      WebView2 browser process under `idle-manager.exe`, not one per account
- [ ] `(manual)` in the Windows VM at 100% and 150% display scaling (set in
      Windows' display settings over RDP), switching
      between one, two and four places, resizing and moving the window keeps
      every game exactly inside its place
- [ ] `(manual)` in the Windows VM with `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`
      set to an empty folder, launch shows only the missing-runtime dialog,
      `Quit` exits, and the data folder's modification times are unchanged

## References

- [Roadmap item](../../roadmap/12-windows-support/README.md) — Back-end,
  Front-end "The Windows engine" and "Start-up dialog", Technical References,
  Blockers 2–3 and 6
- [Wireframes](../../roadmap/12-windows-support/wireframes/) —
  `missing-engine-dialog.md`
- [`docs/requirements.md`](../../requirements.md) — Platform Support
  `FR.1.1`, `FR.1.3`, `FR.1.4`, `FR.1.10`, `FR.2.1`, `FR.2.4`, `FR.3.2`, `FR.3.3`,
  `FR.3.5`
- [`docs/windows-vm.md`](../../windows-vm.md) — the VM these steps run in
- [`docs/architecture.md`](../../architecture.md) — rules 1–4, 6, 10, 12
- [`docs/code-standards.md`](../../code-standards.md) — rules 17, 20, 25, 28
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 7, 10
- [`docs/design.md`](../../design.md) — rule 9
- [`docs/stack.md`](../../stack.md)

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The `(manual)` steps run in the
Windows VM ([`docs/windows-vm.md`](../../windows-vm.md)).
