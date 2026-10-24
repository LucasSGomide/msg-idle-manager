# Goal: Make the Windows build close cleanly and reopen with every account intact — today it freezes on close and again after reopening, and only deleting `%APPDATA%\idle-manager` brings it back

**Status:** not executed
**Rating:** —
**Run:** sequential — first of the pair; 18 reuses the data-root override and the native build notes this prompt adds

## Context

Reported by the owner on 2026-09-27, on a real Windows PC running the release
build from a folder on the Desktop, with the games logged in:

- Closing the application by hand freezes it: the window stops responding and
  never goes away on its own. Letting Windows end it for a reboot does the
  same.
- On the next launch the window opens, the saved accounts are listed, the game
  pages **paint**, and then the whole window locks. No click, key or wait
  unfreezes it.
- The only recovery found is deleting the whole `%APPDATA%\idle-manager`
  folder, which throws away the account list and every login.
- Whether a clean close ever completes is not known. Treat "close freezes" as
  the first symptom to reproduce, "reopen freezes after the pages paint" as the
  second, and find out whether the second is a consequence of the first (a
  kill mid-close leaving something behind) or a fault of its own.

Nobody has ever timed or watched a close on Windows. Roadmap item 16's
Blockers already note that `Update.exe` kills the program 60 s after asking it
to exit and that closing four pages on Windows was never measured. The Windows
VM in `docs/windows-vm.md` needs Docker Engine and has not run on the Linux
machine, so this is the first time the Windows build is driven for real.

Where the code is, from a survey of the current tree — pointers to start from,
not a diagnosis:

- **Files.** `crates/idle-manager-store/src/paths.rs` resolves through
  `directories::ProjectDirs`: `%APPDATA%\idle-manager\config\sessions.toml`
  for the workspace file, `%APPDATA%\idle-manager\data\webview2` for the one
  WebView2 user-data folder every account shares. `session_file.rs` writes
  through a temp file and a rename, backs up old formats, and moves an
  unreadable file aside.
- **Close.** `window/imp.rs` around line 530 calls `SaveOnChange::flush()`
  synchronously when the window closes (`save_on_change.rs`, which otherwise
  saves on a background thread after a short delay). Nothing in the tree
  closes the WebView2 controllers or the shared environment in a deliberate
  order; teardown is whatever drops first.
- **Open.** `crates/idle-manager/src/main.rs` reads the store synchronously in
  `connect_activate`; `restore_workspace` (`window/imp.rs` ~1096–1198) walks
  every account on the UI thread, reading zooms and creating profile folders;
  `start_queue.rs` then starts one account per idle turn, moving on at first
  paint or after 30 s. Each start builds its page through wry's
  `build_as_child` in `web_engine/webview2.rs` and `webview2/host/imp.rs`, and
  the first one also creates the shared environment. On Windows "first paint"
  is the page's load `Finished`.
- **Also running.** The phone server (`idle-manager-remote`) tries to bind on
  start, and the in-app updater (`idle-manager-update`, Velopack) may check for
  a release.

Hypotheses worth testing, none of them assumed: a WebView2 teardown that waits
on the UI thread it is running on; a lock or a crash marker left in the
`webview2` user-data folder after a kill, which the next environment then
waits on; a `sessions.toml` left half-renamed; the shared environment failing
to come up a second time while something waits for it forever; a focus grab
(`follow_focus_with_keyboard`, run at the end of every redraw) ping-ponging
between GTK and the native child windows once pages exist; the updater or
phone server blocking the main loop. The freeze coming **after** the pages
paint says the file read is probably not the culprit.

## Constraints

1. **Two live things on the PC are untouchable.** The release application
   already running from the Desktop folder holds games that are online, and
   its data lives in `%APPDATA%\idle-manager`. Never kill, restart, minimise,
   focus-steal from, or send anything to that process; never open, copy over,
   rename or delete anything under its data folder; never install a build over
   it. Every build this prompt runs is a separate test copy with its own data
   root (constraint 2). If reproducing needs real game profiles, a
   **read-only copy** of the live folder into the test root is allowed as a
   fallback — skip files Windows reports as locked, and never write back.
2. **First code change: a data-root override.** Add one environment variable
   (name it in the style of the existing `IDLE_MANAGER_*` variables, e.g.
   `IDLE_MANAGER_DATA_ROOT`) that every helper in `paths.rs` honours on both
   systems — workspace file, presets, `phone.toml`, engine data root and the
   profile locators, whose `under(...)` constructors already exist for tests.
   With it set, a test copy shares nothing with the live app. If the test copy
   would ever listen for a phone, give its `phone.toml` a port other than
   7466. Document the variable in `docs/stack.md`.
3. **A native build on the PC comes before any fix.** The folder on the
   Desktop is the release program, not a toolchain, and the repository only
   knows how to cross-compile Windows from Linux (`make windows-build`,
   `cargo xwin`, `docs/stack.md` "Cross-compiling"). Set up what a native
   build needs — the Rust MSVC toolchain pinned by `rust-toolchain.toml`, the
   gvsbuild GTK zip pinned as `GVSBUILD_VERSION` in the `Makefile` with
   `pkg-config` pointed at it, the Visual C++ runtime DLLs the release zip
   already carries — and write the exact steps into a new `docs/windows-dev.md`
   so the next session repeats none of it. If a working toolchain is already
   there, say so in the doc and move on. A debug build run from `cmd` with
   `set RUST_LOG=idle_manager=debug` is the right tool for the repro.
4. **Reproduce before fixing, and record it.** With the override set: add at
   least two accounts, let them paint, close by hand, and write down what
   happens and how long it takes; then end the process the way Windows would
   at shutdown, reopen, and write down where it locks. Capture the debug log
   and, for a hang, a thread dump or the equivalent evidence of what the UI
   thread is waiting on. The repro steps and their observed results go into
   `docs/windows-dev.md` as a checklist that is re-run after the fix.
5. **The fix lives in the right layer.** Windows-only behaviour stays behind
   `cfg(windows)` inside `idle-manager-shell`'s engine seam, or in
   `idle-manager-store` for file handling; `idle-manager-core` stays pure and
   the Linux build's behaviour does not change (`docs/architecture.md`,
   `make arch-check`). Whatever the cause, the outcome must hold: a close
   finishes on its own well inside Velopack's 60 s, a kill mid-close or a
   stale lock never again requires deleting the folder, and the account list
   and logins survive both. Where a damaged state is found on open, recover it
   — move it aside, retry, log it — rather than hang.
6. **Checks run where they can.** On the PC: `cargo fmt --check`,
   `cargo clippy` with warnings as errors, `cargo test` for the workspace, all
   natively. On the Linux side, CI's `make verify` (formatting, clippy, tests,
   audit, `arch-check`, `windows-check`, `roadmap-check`) must pass on the
   branch before it is called done; if `make` is not available on the PC,
   push the branch and read CI. Unit-test what can be unit-tested without a
   display — the override's path resolution, any new recovery step in
   `session_file.rs`.
7. **Branch first, commit in the project's shape, never ship.** Create
   `fix/17-windows-close-and-reopen-freeze` before the first code edit
   (GitButler `but branch new` if it is set up on the PC, else
   `git checkout -b`). Commit with `/msg-commit` where the skill is available,
   otherwise as small conventional commits of the same shape (`fix(shell):`,
   `feat(store):`, `docs(windows):`), because `git-cliff` builds the release
   notes from them. Do not push to `main`, merge, land, or tag: the owner ships.
8. **Write the finding down where the project will look for it.** The root
   cause and the fix go into roadmap item 12's `## As built` or `## Blockers`
   in `docs/roadmap/12-windows-support/README.md`; if the close timing is now
   known, close item 16's matching blocker with the figure. Run
   `make roadmap-sync` after any doc change on the Linux side.
9. **Nothing is claimed that was not seen.** A step in the checklist is ticked
   only after it ran on the PC. Anything that could not be exercised — a
   shutdown-initiated close, for example — is written as an open item, not
   as working.

## Output

Working code on the branch: the data-root override with its tests, the fix
for the close and reopen freeze, `docs/windows-dev.md` holding the native
build steps and the ticked repro checklist, the `docs/stack.md` line for the
variable, and the roadmap notes of constraint 8. In chat, a short summary of
the root cause and what changed, and the one thing left for the owner: install
the new build over the live app when the games can be paused, and confirm a
close and reopen with the real accounts.
