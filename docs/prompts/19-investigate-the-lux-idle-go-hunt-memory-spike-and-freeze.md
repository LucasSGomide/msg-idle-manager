# Goal: Find out why opening Lux Idle's "go hunt" tab makes memory spike and the whole application freeze inside idle-manager when the same steps in Microsoft Edge do not, and fix it in the application if the cause is there

**Status:** executed 2026-10-24 — roadmap item 18, branch `fix/18-lux-idle-hunt-tab-spike`
**Rating:** —

## Context

Reported by the owner on 2026-10-24. With the game Lux Idle running as an
account in the application, clicking the game's "go hunt" tab makes memory
climb sharply and everything freezes — not just the page, the window. The
owner reproduced the same click path in Microsoft Edge and memory stayed
flat. Other players of the game, on far weaker machines, do not see it. So
the working assumption is that the fault is in idle-manager, not in the game
or the hardware; the investigation has to confirm or overturn that with
measurements, not restate it.

Two facts are not in the report and have to be established first, by asking
the owner or from the saved workspace, before anything is measured:

- **Which build showed it.** The owner runs the application two ways: a
  debug build via `make dev` on the Linux laptop (GNOME, Intel Arc 130V/140V,
  WebKitGTK 6.0 — `docs/memory-budget.md` says this is how real, day-long
  accounts are actually run) and the Windows release zip on a real Windows PC
  (Edge WebView2; prompts 17 and 18 cover that machine). "Microsoft Edge" as
  the control suggests Windows but does not settle it — Edge installs on
  Linux too. The two engines share almost nothing, so this decides which half
  of the hypotheses below applies. If both builds show it, that in itself
  narrows the cause to what the two engines share: the injected scripts and
  the shell.
- **What the game is.** "Lux Idle" has no preset under `presets/`
  (`baiaki-idle`, `huntera`, `lorvath` are the three), so it is a typed-address
  account; the address is in the workspace file (`sessions.toml` under the
  data root `paths.rs` resolves). A web search on the name found nothing, so
  open the address and find out how the hunt tab draws — a `<canvas>` with a
  2D or WebGL context, a Unity or Phaser build, a DOM-heavy list, large
  sprite sheets — and whether it logs to the console while hunting. What the
  tab does is what decides which of the application's choices it collides
  with.

Where the application differs from a plain browser, from a survey of the
current tree — the places a game-specific spike can come from, none of them
assumed:

- **Injected scripts** (`crates/idle-manager-shell/src/web_engine/script_set.rs`,
  sources under `crates/idle-manager-shell/resources/js/`). Every page gets
  `page-console.js` and `keep-awake.js` at document start; on Windows
  `webview2-bridge.js` runs before both. `page-console.js` overrides
  `console.*` and renders every argument with `JSON.stringify` **before**
  cutting the text to 2000 characters — a game that logs a large state object
  per hunt tick pays a full serialisation each time, then an IPC post and a
  JSON parse on the Rust side. On Linux the handler only exists when
  `IDLE_MANAGER_DIAGNOSTICS` is set (`register_page_console_handler`, gated by
  `page_console_forwarding_enabled` in `web_engine/webkit.rs`), so the shim
  returns early otherwise; on Windows the bridge always defines
  `webkit.messageHandlers.pageConsole`, so every `console.log` goes through
  it unconditionally. `keep-awake.js` replaces `requestAnimationFrame`,
  `document.hidden` and `visibilityState` and, while armed on a hidden page,
  re-delivers `ResizeObserver` and `IntersectionObserver` observations from a
  timer of its own — check whether this account has keep-awake on and
  whether the shim is armed while the page is on screen.
- **Linux: the debug build instruments the page.** `diagnostics_enabled()`
  is true in every debug build, which turns on `set_enable_developer_extras`
  (the inspector backend, which the code's own comment says makes the engine
  "instrument a page differently when something might attach") and
  `set_enable_write_console_messages_to_stdout` (every console message
  written synchronously to the terminal). A game that logs at frame rate
  under both is a candidate for exactly this symptom. Compare a release build
  (`cargo run --release`) against `make dev` on the same tab.
- **Linux: memory-pressure thresholds.** `web_engine/webkit.rs` sets a
  3072 MiB limit with strict at 0.5 (1536 MiB) and conservative at 0.33
  (~1 GiB) on every rendering process and the networking process. Past
  strict, on every poll the engine throws away all compiled JavaScript and
  decoded images and the game recompiles and re-decodes them — the comment
  records four Huntera accounts held at 6.6 cores by exactly this before the
  limit was raised. If the hunt tab takes one rendering process past 1536 MiB
  the engine's own release loop would read as a freeze. The memory footer
  samples PSS every 5 s (`memory_footer.rs`); its per-process readings say
  which process grows and how fast.
- **Linux: graphics.** `main.rs` re-execs with `GSK_RENDERER=opengl` because
  GTK's Vulkan renderer could not import WebKitGTK's buffers on this GPU.
  WebGL is on for a typed-address account (`WEBGL_DEFAULT_ENABLED`,
  `idle-manager-store/src/preset.rs`). A WebGL or large-canvas tab is the one
  place where the engine, the GTK renderer and the Intel driver all meet;
  `GDK_DEBUG=dmabuf` and `WEBKIT_DISABLE_DMABUF_RENDERER=1` are the switches
  to try, and `settings.set_enable_webgl(false)` through a preset file for
  the game is the one-line test of the WebGL half.
- **Windows: WebView2 is the same Chromium as Edge**, so an Edge-flat /
  WebView2-spiking result points at what the application adds: the three
  injected scripts, `BROWSER_ARGS` in `web_engine/webview2.rs`
  (`--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection
  --disable-backgrounding-occluded-windows`), the host child-window sizing
  and zoom in `webview2/host/imp.rs`, and the process-tree memory probe
  `idle-manager-metrics` walks every 5 s. Ditched record 01 says GPU use on
  Windows was never measured on real hardware; this is the first chance.
- **Shared: the shell itself.** A spike in the application's own process
  rather than a rendering process points at the shell — the memory footer's
  sampling, the phone server's frame capture (`frame_dump.rs` is off by
  default; the phone's watched mode captures only while a phone is
  connected), the `tracing` sink receiving forwarded console output.

Prior art to read before measuring: `docs/memory-budget.md` (how memory was
measured in rounds, and the leak the page-console switch exists to stop),
`docs/roadmap/05-memory-accounting/README.md` (the PSS accounting and the
751 MiB record), `docs/ditched/03-cache-clean-up-as-a-memory-fix.md` (the mu
idle leak that was not the application's fault — the same conclusion is
possible here and is a valid outcome).

## Constraints

1. **The running application and its data are untouchable.** On whichever
   machine reproduces this, the owner's live instance holds games that are
   online. Never kill, restart or send input to it, never write under its
   data root. Every build this prompt runs is a separate copy with its own
   data root — `IDLE_MANAGER_DATA_ROOT` if prompt 17 has landed it, else the
   `under(...)` constructors in `idle-manager-store/src/paths.rs` behind a
   temporary switch. If the game needs a login to reach the hunt tab, make a
   fresh login in the test copy; a read-only copy of the live profile folder
   is the fallback, never a write-back.
2. **Establish platform and game before measuring.** Record which build
   showed it (Linux `make dev`, Linux release, Windows release) and the game's
   address and rendering method in the finding. Do not run the Linux-only
   hypotheses against a Windows report or the other way round.
3. **Reproduce with numbers before touching code.** Per-process memory over
   time from the footer's probe (or `smem` / Task Manager per process) for
   the shell, the rendering process and the networking process, before the
   click, during the climb and at the freeze; what the UI thread is doing at
   the freeze (a `gdb` / `eu-stack` backtrace on Linux, Process Explorer
   thread stacks or a `procdump` on Windows); the page's own console while
   hunting, with `IDLE_MANAGER_DIAGNOSTICS=1` only in a run that is marked as
   such because the switch changes the behaviour under test. Then the same
   click path in Edge on the same machine with the same per-process view, so
   the comparison is like for like.
4. **Bisect the application's additions one at a time.** Each injected script
   removed, the debug/release difference, WebGL off through a preset, the
   memory-pressure limit raised, the renderer and dmabuf switches, the
   browser arguments — one change per run, result written down, the rest held
   constant. Stop when a single change makes the spike go away, then confirm
   by putting only that change back.
5. **The fix lives in the right layer.** Engine-specific behaviour stays in
   `idle-manager-shell`'s engine seam (`web_engine/webkit.rs`,
   `web_engine/webview2.rs`), shared script changes in `resources/js/` with
   `script_set.rs`'s unit tests updated, preset defaults in
   `idle-manager-store`; `idle-manager-core` stays pure (`make arch-check`).
   A fix must not regress what the existing comments record as measured —
   the 3072 MiB limit's reasoning, the page-console switch's leak, the
   OpenGL renderer choice — and if it changes one of those numbers the
   comment and `docs/memory-budget.md` change with it.
6. **If the cause is the game, say so and stop.** The outcome "Lux Idle's
   hunt tab allocates this much in any engine and Edge merely hides it
   behind its tab discard" is acceptable if the numbers show it; then the
   deliverable is the finding and, if useful, a preset for the game with
   whatever setting makes it bearable. Do not add a game-specific workaround
   to the shell.
7. **Checks and branch.** Create `fix/<item>-lux-idle-hunt-tab-spike` before
   the first code edit, numbered after the roadmap item the finding lands on
   (05 for a memory-accounting cause, 12 for a Windows-only one, a new item
   through `/msg-pre-roadmap` if it is neither). `make verify` on Linux must
   pass before it is called done (windows-check needs LLVM this machine does
   not have; run `make roadmap-check` separately). Commit with `/msg-commit`
   where available, else small conventional commits. Never push to `main`,
   merge, land or tag.
8. **Write the finding where the project will look for it.** Root cause,
   the measurements that proved it, and the bisect table go into the
   roadmap item's `## As built` or `## Blockers`; a game-is-the-cause outcome
   goes to `docs/ditched/` as a new numbered record so it is not re-proposed.
   Run `make roadmap-sync` after any doc change. Nothing is written as a
   cause that the bisect did not isolate.

## Output

A diagnosis backed by per-process measurements and a one-change-at-a-time
bisect table, written into the roadmap or ditched doc of constraint 8. If
the cause is in the application: the fix on the branch with its tests, and
the same click path re-run with the fix showing a flat memory line and no
freeze. If the cause is the game: the ditched record and, where one helps, a
`presets/lux-idle.toml`. In chat, a short summary: which build, what the
hunt tab does, the one change that made the difference, and the one thing
left for the owner to confirm on their own machine.
