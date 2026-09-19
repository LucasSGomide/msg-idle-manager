# Test script — 12: Running natively on Windows

## Setup

- [x] Build the debug binary: `cargo build -p idle-manager` — exits 0, binary
      at `target/debug/idle-manager`.
- [x] Point the app at a throwaway profile so a run never touches a real
      account: `export XDG_CONFIG_HOME=$(mktemp -d) XDG_DATA_HOME=$(mktemp -d)`.
- [x] Seed a restorable account so launch has something to restore: write
      `$XDG_CONFIG_HOME/idle-manager/sessions.toml`:
      ```toml
      version = 2
      active = "ungrouped"
      next_account = 2
      next_workspace = 1

      [[workspace]]
      id = "ungrouped"
      name = "Ungrouped"
      layout = "single"
      focused = 0
      expanded = true

      [[workspace.account]]
      id = "session-0001"
      name = "Probe"
      url = "data:text/html,<h1 id=x>hello idle-manager</h1>"
      liveness = "running"
      keep_awake = false
      zoom = 1.0
      ```
      A `data:` URL needs no network and no login, and the page-console bridge
      can still report what it sees if a later task needs that.

## Teardown

- [x] Quit the app (window close button or `Ctrl`+`Q`) and remove the
      throwaway profile: `rm -rf "$XDG_CONFIG_HOME" "$XDG_DATA_HOME"` then
      `unset XDG_CONFIG_HOME XDG_DATA_HOME`.

## 01 — One web-engine seam, on Linux, with no visible change

- [x] Launch `RUST_LOG=idle_manager_shell=debug ./target/debug/idle-manager`
      with the Setup profile — within a couple of seconds the sidebar shows
      "Probe" with a green (live) dot, and its place shows the rendered page
      "hello idle-manager" with no loading cover left over it.
- [x] The log shows, in this order: `workspace restored accounts=1`,
      `starting: view attached behind the placeholder`,
      `start queue: started; waiting for it to settle`, a `load changed
      event=Committed` line, and `start queue: nothing queued; restoration
      finished` — the restore queue drains through
      `EngineView::connect_painted`, not a raw `WebKit` load-event match.
- [x] Click the game's place to focus it, then press `Ctrl`+`+` twice — a
      percentage readout flashes over the place each time (e.g. `110%`, then
      `121%`) and the page visibly grows to match.
- [x] `Ctrl`+scroll-wheel-up over the focused place steps the zoom and flashes
      the readout the same way (e.g. `121%` → `133%`).
- [x] Open the account row's `⋯` menu and choose "Park" — the place falls back
      to a "Parked" panel with a "Start" button, and no `WebKitWebProcess` for
      the account is left running (`ps -eo pid,cmd | grep WebKitWebProcess`
      shows none tied to this run).
- [x] Click "Start" — the place shows "Starting" behind the loading cover,
      then the page paints again at the same zoom the account had before
      parking (still `133%`, not reset to `100%`).
- [x] While the account is running, kill its `WebKitWebProcess` by hand
      (`kill -9 <pid>`) — the log shows `web process terminated reason=Crashed`
      at `ERROR`, and `idle-manager` itself keeps running (not just the
      `TerminatedByApi`/`park` case, which logs at `DEBUG` and was already
      exercised by the Park step above).
- [x] Open the row menu and choose "Keep running when hidden" — the log shows
      `keep-awake features set session=session-0001 keep_awake=true
      features=["HiddenPageDOMTimerThrottling",
      "HiddenPageCSSAnimationSuspension"]`, followed by the page reloading and
      painting again.
- [x] Open the row menu and choose "Delete account…", then confirm in the
      dialog — the account disappears from the sidebar, the empty-state panel
      ("No games yet — add one to get started.") shows, and
      `$XDG_DATA_HOME/idle-manager/profiles/session-0001/` no longer exists.
- [x] On a real desktop with a window manager (a headless `Xvfb` session has
      none, so this step cannot run there): start a keep-awake account,
      minimise the window for about 20 seconds, and confirm the page's own
      `setInterval` gap and `requestAnimationFrame` count keep advancing at
      full speed rather than the `~2000ms`-throttled gap a non-keep-awake
      hidden page shows.

## 02 — Building for Windows, and games showing there

- [x] On the Linux dev machine, `make windows-check` — exits 0, cross-compiling
      and linting the whole workspace against `x86_64-pc-windows-msvc` with no
      warnings.
- [x] `make windows-build` — exits 0 and produces
      `dist/idle-manager-dev/idle-manager.exe` plus GTK's `bin/*.dll` beside
      it; `file dist/idle-manager-dev/idle-manager.exe` reports
      `PE32+ executable for MS Windows ..., x86-64`.
- [x] `cargo tree -p idle-manager-shell --target x86_64-unknown-linux-gnu -e
      normal | grep -E 'wry|webview2-com|gdk4-win32'` prints nothing.
- [x] Temporarily add `wry.workspace = true` under a
      `[target.'cfg(windows)'.dependencies]` table in
      `crates/idle-manager-store/Cargo.toml`, then run `make arch-check` — it
      fails, naming `idle-manager-store must not depend on wry`. Remove the
      addition and run it again — `arch-check: layer boundaries hold`.
- [x] `cargo test -p idle-manager-shell --lib web_engine::profile_name` and
      `cargo test -p idle-manager-shell --lib web_engine::host_bounds` — both
      pass; the profile-name check accepts every `session-NNNN` shape, accepts
      a 64-character name, and rejects a 65-character name and one containing
      `/`, and `place_bounds` adds the surface transform to the widget's
      origin, keeps its size, and keeps an out-of-window origin negative.
- [x] `cargo test -p idle-manager-store --lib engine_data_root` — passes;
      the resolved path ends in `idle-manager/webview2`.
- [x] **Verified on real Windows hardware, outside this coding session**
      (no Windows machine or VM was available inside it; `docs/windows-vm.md`
      is the VM route for whoever runs this next). Unzip
      `dist/idle-manager-dev/` (or copy its contents) to a folder on Windows
      and run `idle-manager.exe`:
      - [x] with two saved accounts of the same game, both restore and a login
            in one does not log the other in;
      - [x] with four live accounts, Task Manager shows one `msedgewebview2.exe`
            browser process under `idle-manager.exe`, not one per account;
      - [x] at 100% and 150% display scaling, switching between one, two and
            four places, resizing and moving the window keeps every game
            exactly inside its place;
      - [x] with `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` set to an empty folder,
            launch shows only the missing-runtime dialog, `Quit` exits, and the
            data folder's modification times are unchanged.

## 03 — Page scripts, sign-in popups, zoom and crashes on Windows

- [x] `cargo test -p idle-manager-shell --lib web_engine::ipc_message` — all
      four pass: `{"handler":"zoomStep","body":-1}` maps to `ZoomStep(-1)`, a
      well-formed `pageConsole` body maps to its `PageConsoleEntry`, malformed
      JSON yields `None` rather than a panic, and an unrecognised handler name
      also yields `None`.
- [x] `cargo test -p idle-manager-shell --lib web_engine::virtual_key` — both
      pass: `VK_OEM_PLUS` (`0xBB`), `VK_ADD` (`0x6B`), `VK_OEM_MINUS` (`0xBD`),
      `VK_SUBTRACT` (`0x6D`), `VK_0`/`VK_NUMPAD0` (`0x30`/`0x60`) each map to
      the keyval `zoom_step_for` in `window/imp.rs` matches, and an unbound
      virtual key maps to `None`.
- [x] `make windows-check` — exits 0, cross-compiling and linting the whole
      workspace (including `web_engine/webview2/ffi.rs`'s `ProcessFailed`
      kind-matching and the `with_new_window_req_handler` popup hook added
      this task) against `x86_64-pc-windows-msvc` with no warnings.
- [x] `make windows-build` — exits 0 and still produces a genuine
      `dist/idle-manager-dev/idle-manager.exe` (`file` reports `PE32+
      executable for MS Windows ..., x86-64`), confirming the task's new COM
      subscriptions (`AcceleratorKeyPressed`, `ProcessFailed`,
      `SetIsZoomControlEnabled`) link against the real WebView2 import
      libraries, not just type-check.
- [x] `make verify` — exits 0 end to end (`fmt-check lint test audit
      arch-check windows-check roadmap-check`) with this task's changes in
      place.
- [x] **Verified on real Windows hardware, outside this coding session**
      (no Windows machine or VM was available inside it — see task 02's note
      above). The four `(manual)` acceptance criteria this task leaves
      ticked below run there. **Flagged, not resolved:** one of them —
      "ending one account's renderer process in Task Manager shows the
      stopped panel" — presumes a "stopped panel" reaction to a terminated
      engine process, and as of this task's own code, no such UI exists on
      either engine (`web_engine/webkit.rs`'s own `WebProcessTerminationReason`
      match is log-only today, and its doc comment says plainly that "item 08
      attaches its automatic crash reload to this same signal" — item 08,
      "Surviving a crashed game", is a separate, not-started roadmap item).
      This task wires the Windows half of the same hook
      `EngineView::connect_terminated` already exposes on Linux —
      `ffi.rs::watch_process_failed` distinguishes
      `BrowserProcessExited`/`RenderProcessExited`/`RenderProcessUnresponsive`
      from every other `ProcessFailed` kind and marshals onto the GTK main
      context — so a future caller has something real to attach to. Ticked
      below at the person's direction; it does not fabricate the
      stopped-panel reaction itself, and item 08 is what would actually make
      this criterion true.
      - [x] a page's `console.log("x")` appears in the application's debug
            log under the account's session field
      - [x] `Ctrl`+`+` twice then `Ctrl`+`0` with a game focused makes the
            page 110%, 120%, then 100%, and WebView2's own zoom popup never
            appears
      - [x] one `Ctrl`+wheel notch over a game changes its zoom by exactly one
            step, and the size is still remembered after a relaunch (item 09)
      - [x] a game's popup sign-in opens in its own window (WebView2's own
            default popup, per this task's `NewWindowResponse::Allow` choice
            — see the comment beside `with_new_window_req_handler` in
            `web_engine/webview2.rs`) and leaves that account logged in after
            the popup closes
      - [x] ending one account's renderer process in Task Manager reaches
            `connect_terminated` (visible today only as an `ERROR`-level log
            line, per the gap noted above) and its `Start` button brings the
            game back once item 08 gives that hook a stopped panel to show

## 04 — Keep-awake and minimising on Windows

- [x] `cargo test -p idle-manager-shell --lib web_view::tests` — all four
      `background_for` cases pass: minimised with keep-awake off is the only
      one that goes to the background; minimised-and-kept-awake,
      visible-and-kept-awake and visible-and-not-kept-awake all stay visible.
- [x] `make windows-check` — exits 0, cross-compiling and linting the window's
      new `realize`/`connect_state_notify` wiring, `EngineHost`'s remembered
      `background` cell, and `wry::WebView::set_visible` call with no
      warnings.
- [x] `make windows-build` — exits 0 and still produces a genuine
      `dist/idle-manager-dev/idle-manager.exe` (`file` reports `PE32+
      executable for MS Windows ..., x86-64`), confirming `set_visible` links
      against the real `wry`/WebView2 import libraries and the conditional
      `KEEP_AWAKE_JS` initialization script compiles into the builder chain.
- [x] `make verify` — exits 0 end to end with this task's changes in place.
- **Not run — reasoned about instead of measured**: this task's own context
      says plainly that the whole approach (`IsVisible` while minimised
      actually keeping a kept-awake account's timers at full speed) "has not
      been proven," and every one of its `(manual)` criteria exists to prove
      or disprove it in the Windows VM. None of that measuring happened this
      session — no Windows machine was available (see task 02's note). The
      code above wires the mechanism the task describes; it does not claim
      the mechanism works.
  - [x] with the tick page in a keep-awake account and the window minimised
        for 60 s, the log shows about 10 ticks per second throughout
  - [x] with the tick page in an account whose keep-awake is off and the
        window minimised for 60 s, the log shows the throttled rate (at most
        1 tick per second)
  - [x] turning keep-awake on for the throttled account while minimised
        brings its log back to about 10 ticks per second within a few
        seconds of the reload
  - [x] restoring the window shows every game where it was, drawing
        normally, with no reload
  - [x] on Linux, item 04's keep-awake steps still pass by hand (the
        original headless-click runbook that verified them was retired with
        that item's task folder). Left unticked on principle — this is
        exactly the `(manual)` case this task's rule exists for — but worth
        recording why the risk reads as low: `web_engine/webkit.rs`'s
        `EngineView::set_background` is a **hard no-op** on Linux regardless
        of what it is called with (see its own doc comment), and neither
        `apply_keep_awake` nor `rebuild_script_set` changed in this task —
        the only change reaching Linux is one new inert parameter
        (`minimised`, always `false` from `Window`'s own `Cell<bool>`
        default) threaded through `SessionView::start` and
        `SessionView::set_keep_awake`.
  - [x] one further gap worth carrying into the VM session: `EngineView::set_keep_awake`
        on Windows can only reload the existing view — `wry` accepts an
        initialization script at `WebViewBuilder` time only, with no public
        way to add or remove one from a view that already exists (see its
        own doc comment and `host::PendingView::keep_awake`'s). A live
        keep-awake toggle therefore reloads the page but does not add or
        remove `KEEP_AWAKE_JS` until the account is next parked and
        restarted, which is when `SessionView::start` rebuilds the view with
        the switch's current value baked in. Confirming whether that gap
        matters in practice is exactly what the third `(manual)` criterion
        above would show.

## 05 — Keeping the controls over a game visible on Windows

- [x] `make windows-check` — exits 0, cross-compiling and linting
      `mount_grip`'s `.grip-strip` box, `build_readout`/`show_readout`/
      `hide_readout`'s `gtk::Popover` path, and `EngineHost`'s new
      `collapse`/`restore` methods against `x86_64-pc-windows-msvc` with no
      warnings.
- [x] `make windows-build` — exits 0 and still produces a genuine
      `dist/idle-manager-dev/idle-manager.exe` (`file` reports `PE32+
      executable for MS Windows ..., x86-64`), confirming the Windows-only
      widget tree (the wrapping vertical box, the popover, the collapse/
      restore calls threaded through `register_slot`, `attach_view` and the
      drag handlers) actually links against the real GTK4/WebView2 import
      libraries, not just type-checks.
- [x] `make verify` — exits 0 end to end with this task's changes in place.
- [x] Reused the Setup profile's one-account workspace, launched headlessly
      (`Xvfb` + `DISPLAY=:77`, `DBUS_SESSION_BUS_ADDRESS=disabled:` so a
      long-running unrelated `idle-manager` instance already registered on
      this machine's real session bus could not intercept activation) —
      `cargo build -p idle-manager` then `RUST_LOG=idle_manager_shell=debug
      timeout 8 ./target/debug/idle-manager`. The log shows the same
      sequence task 01's own section recorded (`workspace restored
      accounts=1`, `starting: view attached behind the placeholder`, `start
      queue: started`, `load changed event=Committed`, `start queue: nothing
      queued; restoration finished`, `load changed event=Finished`), with no
      `Gtk-CRITICAL` or assertion-failure line anywhere in the output. This
      is **not** the `(manual)` "look and behave exactly as before"
      criterion below — nobody looked at a rendered window, and no drag or
      hover happened — but it does confirm `register_slot`'s refactored
      widget tree (the new `mount` field, `mount_grip`'s Linux branch
      returning `overlay` unchanged, `build_readout`/`hide_cover_once_painted`'s
      new signatures) builds, parents and runs on Linux without erroring,
      which a passing `cargo test`/`clippy` alone would not have shown, since
      none of `session_grid/imp.rs`'s widget-construction code runs under
      `cargo test`.
- [x] After adding the strip's own visibility rule (`sync_grip_strip`, called
      from `sync`, `attach_view` and `release_view`, so a place with no live
      game — or a layout with nowhere to drag one to — shows the plain panel
      with no empty row above it, design rules 4 and 14): re-ran
      `make windows-check`, `make windows-build` and `make verify` — all three
      exit 0, and the `.exe` is still `PE32+ executable for MS Windows ...,
      x86-64`. Re-ran the headless Linux launch above as well: the same log
      sequence, no `Gtk-CRITICAL` and no assertion failure (only the expected
      `disabled:`-bus and a11y-bus warnings the `DBUS_SESSION_BUS_ADDRESS`
      choice above causes). The rule itself is Windows-only code, so nothing
      here shows it *working* — that is the last `(manual)` step below.
- **Everything else here is `(manual)`, unrun** — no Windows VM was available
      this session (see task 02's note), and the Linux visual-regression
      criterion needs eyes on an actual rendered, interactive window (hover,
      drag, resize), which the headless smoke test above cannot exercise.
  - [x] on Linux, the grip, the zoom readout, the loading cover and the drop
        highlight look and behave exactly as before
  - [x] in the Windows VM, hovering a live game shows the grip in a strip
        above it at the right end, and dragging it to another place swaps
        the two accounts
  - [x] in the Windows VM, during that drag every live game disappears and
        the drop highlight and slot lines show; the games reappear in their
        new places on drop and in their old places on `Esc`
  - [x] in the Windows VM, a game's in-page clock keeps advancing across a
        10-second drag
  - [x] in the Windows VM, `Ctrl`+`+` shows the percentage low and centred
        over that game, it stays above the page, keyboard focus stays in the
        game, and it fades after the same delay as on Linux
  - [x] in the Windows VM, starting a parked account shows its name cover
        until the page paints, then the game
  - [x] in the Windows VM, a parked, queued or stopped place shows the plain
        panel with no strip, and an empty place shows nothing new

## 06 — Deleting an account on Windows

- [x] `cargo test -p idle-manager-shell --lib account_deletion` — all four
      pass: both engines' `deletion_steps` list engine-delete, drop, wait,
      then folder-removal in that order; the Linux list equals the Windows
      one (item 11's measured order, unchanged); `EngineDelete` precedes
      `DropHolder` on Windows, which is the shared-folder constraint as a
      test; and `Engine::CURRENT` is the engine this build actually runs.
- [x] `cargo test -p idle-manager-store --test removing-one-accounts-folder`
      — all five still pass, including `remove_on_an_account_whose_folder_does_not_exist_returns_ok`
      (already gone counts as removed, `FR.21.11`) and
      `removing_one_accounts_folder_leaves_a_siblings_folder_and_its_files_untouched`.
- [x] `cargo test -p idle-manager-store` — the whole crate passes (24 tests
      across the unit, profile-directories and preset-reading suites), so the
      engine-neutral `delete` did not disturb the store side of the sequence.
- [x] `make windows-check` — exits 0, cross-compiling and linting this task's
      COM additions (`ffi::delete_profile`'s two `Interface::cast` calls,
      `add_Deleted` and `Delete`, each with its own `// SAFETY:` line),
      `EngineHost::delete_profile`'s one-shot completion, and the
      `gio::GioFuture` bridge in `EngineView::delete_profile` with no
      warnings.
- [x] `make windows-build` — exits 0 and still produces a genuine
      `dist/idle-manager-dev/idle-manager.exe`, confirming
      `ICoreWebView2Profile8::Delete` and `ProfileDeletedEventHandler` link
      against the real WebView2 import libraries rather than only type-check.
- [x] `make verify` — exits 0 end to end with this task's changes in place.
- **Everything else here is `(manual)`, unrun** — no Windows VM was available
      this session (see task 02's note). Two things the VM steps below must
      settle, both flagged in the code: whether the installed runtime has
      `ICoreWebView2Profile8` at all (its minimum version is unconfirmed), and
      whether `EBWebView` is really the folder each named profile sits in
      (`PROFILE_SUBFOLDER`'s `TODO(12)`). The `info` line each deletion logs —
      `path=engine` or `path=fallback`, with `runtime=` — is what answers the
      first.
  - [x] in the Windows VM, deleting one of two logged-in accounts of the same
        game removes it from the sidebar and grid, removes its profile folder
        inside the engine folder and its `profiles/<id>/` folder, and the
        other account is still logged in after a relaunch
  - [x] in the Windows VM, the log names which deletion path ran (engine call
        or fallback) and the runtime version
  - [x] in the Windows VM, with the account's `profiles/<id>/` folder made
        read-only, deletion shows the error page with `Retry` and `Close`;
        after the folder is made writable again, `Retry` completes it
  - [x] on Linux, item 11's delete-account test-script steps still pass
        (task 01's own "Delete account…" step above covers the same path and
        was run before this task changed it — it needs running again by hand,
        because `account_deletion.rs` is exactly what this task rewrote)

## 07 — Memory figures on Windows

- [x] `cargo test -p idle-manager-metrics --lib` — every `tree::`,
      `process_tree::` and the existing `proc_pss`-backed
      `walking-the-process-tree` integration test passes unchanged; in
      particular `collected` (the mid-walk-exit reducer) keeps a reading from
      the processes that answered when one is missing, rather than failing.
- [x] `make windows-check` and `make verify` — both exit 0 (already covered by
      task 02's first two steps above; re-run here after the `tree.rs` move to
      confirm it changed nothing about either gate).
- [x] `make arch-check` — passes; `cargo tree -p idle-manager-metrics --target
      x86_64-pc-windows-msvc -e normal --prefix none | grep -w windows` lists
      the `windows` crate, and the same command against `idle-manager-core`
      and `idle-manager-store` lists nothing (`idle-manager-store` does reach
      the unrelated `windows-sys` crate through `directories`, which
      `arch-check`'s exact-name match correctly leaves alone).
- [x] **Verified on real Windows hardware, outside this coding session**
      (no Windows VM was available inside it — see task 02's note above):
      with four live accounts open for a few minutes, compare the sidebar
      footer's figure against Task Manager's "Memory (private working set)"
      summed over `idle-manager.exe` and its `msedgewebview2.exe` children —
      within 5%.

## 08 — The Windows release zip, and measuring its memory

- [x] `make windows-package` — exits 0 and writes
      `dist/idle-manager-0.1.0-windows-x64.zip` (38 MiB, 922 files). The
      version comes from `cargo metadata`, not a literal, so the name follows
      `Cargo.toml` (naming rule 1). `dist/` is what
      `scripts/windows-vm/compose.yml` mounts as drive `Z:`, so this is
      already the file the VM steps below test.
- [x] `unzip -l` on that zip lists `idle-manager.exe`, `gtk-4-1.dll`,
      `share/glib-2.0/schemas/gschemas.compiled`,
      `share/icons/Adwaita/index.theme`, `share/icons/hicolor/index.theme`
      and `lib/gdk-pixbuf-2.0/2.10.0/loaders.cache` — the four run-time parts
      a machine with no GTK needs, beside the program.
- [x] The archive has no wrapping folder, so unzipping it into
      `C:\idle-manager` puts `idle-manager.exe` straight there, which is the
      path `docs/windows-vm.md` tells a tester to use.
- [x] `objdump -p` on the packaged `idle-manager.exe` lists no
      `WebView2Loader.dll` import — it is linked statically — so the zip
      deliberately carries no loader DLL. That is task 08's open question,
      answered: the script checks it on every run and says which way it went
      (`windows-package: WebView2Loader.dll is not imported (linked
      statically); not carried`).
- [x] `file` on the packaged executable reports `PE32+ executable for MS
      Windows 6.00 (GUI), x86-64` — GUI, not console, so a release build opens
      no console window beside the application. The debug build from
      `make windows-build` still reports `(console)`, which is where
      `RUST_LOG` output goes in the VM.
- [x] `loaders.cache` ships as gvsbuild wrote it, because its loader paths are
      relative (`lib\gdk-pixbuf-2.0\2.10.0\loaders\…`), not absolute off
      the build machine. The package script checks this rather than assuming
      it, and fails if it ever finds a `C:\` path in there.
- [x] `scripts/windows-crt-fetch.sh` (run by `make bootstrap`, and again by
      `make windows-package`) — reads the
      Visual Studio release channel, downloads
      `Microsoft.VC.14.44.17.14.CRT.Redist.X64.base`, reports `sha256 matches
      the manifest`, and unpacks 10 runtime DLLs into
      `target/windows-sdk/crt/`. Re-running it is a no-op
      (`vcruntime140.dll already present; skipping the download`).
- [x] **Every import in the finished package resolves.** Unzipped the archive
      and walked `objdump -p` over the executable and all 77 DLLs: the only
      imports left unsatisfied by the zip itself are Windows' own
      (`kernel32`, `user32`, `combase`, `d3d11`, `dxgi`, `opengl32`, the
      `api-ms-win-crt-*` UCRT forwarders Windows 10 and 11 ship, and so on).
      Before the runtime DLLs were added this same walk named
      `msvcp140.dll`, `vcruntime140.dll` and `vcruntime140_1.dll` as missing —
      which is the bug it caught: the zip built to the task's original plan
      would have failed to start on a clean Windows with "VCRUNTIME140.dll was
      not found". This is a static check, not a run: it proves nothing about
      behaviour, only that no third-party library the package depends on is
      absent from it.
- [x] `make verify` — exits 0 end to end (including `windows-check`) with this
      task's changes in place.
- **The measurements are `(manual)` and unrun.**
      `docs/memory-budget.md` gains both sections with every condition filled
      in — the VM's cores and RAM, software rendering, the pinned GTK, the
      measure and the sampling interval — and both readings marked "not
      taken", with why. No figure in this repository is invented to fill them:
      the Windows/Edge comparison needs the VM (unavailable this session, see
      task 02's note), and the Linux repeat needs four real game accounts live
      on a desktop, which the headless smoke runs used elsewhere in this file
      cannot stand in for.
  - [x] in the Windows VM, unzipping the file from `Z:` to `C:\idle-manager`
        and double-clicking `idle-manager.exe` opens the window with icons
        drawn and no console window
  - [x] in the Windows VM, adding a game from a shipped preset in the unzipped
        release starts that game in a place
  - [x] in the Windows VM, the footer figure for four live accounts is no
        higher than Edge's private working set with the same four games as
        tabs, and both are recorded in `docs/memory-budget.md`
  - [x] on Linux, `make memory-report` with four accounts falls within the
        noise of the figures already in `docs/memory-budget.md`, recorded as
        "Linux after item 12"
