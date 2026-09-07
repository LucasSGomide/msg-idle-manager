# 07 — Restoring the workspace on launch · test script

Hand-run runbook that proves the item works end to end, beside the tasks'
`(unit)` / `(integration)` / `(manual)` criteria rather than replacing them.
Every line is one concrete action and the observable result it must produce.
Tick a box only after the step has actually been run.

`## Setup` and `## Teardown` are shared and grow as tasks need them; each task
appends its own `## NN — Title` section and never rewrites another's.

## Setup

- [x] `export PATH="$HOME/.cargo/bin:$PATH"` and `cd` to the repo root, so
      `cargo` and `make` resolve.
- [x] `cargo build --workspace` completes without error — the whole workspace
      compiles with the workspace-restore code in place.
- [x] **Headless shell harness** (for the `(manual)` shell checks, per
      [[verifying-shell-slices-headless]]): a fresh display —
      `Xvfb :95 -screen 0 1280x800x24 -noreset &` (after `pgrep -x Xvfb` +
      `kill` and `rm -f /tmp/.X95-lock`) — then the app under
      `dbus-run-session -- env DISPLAY=:95 GDK_BACKEND=x11 GSK_RENDERER=cairo
      XDG_CONFIG_HOME=<tmp>/cfg XDG_DATA_HOME=<tmp>/data ./target/debug/idle-manager`.
      Synthetic input via `scratchpad/harness/pyin.py` (ctypes XTest,
      `XTestQueryExtension` once, `move`/`click`/`type`/`key`/`sleep`); frames
      via `ffmpeg -f x11grab -video_size 1280x800 -i :95.0 -frames:v 1`. Write
      `<tmp>/cfg/idle-manager/sessions.toml` by hand to set each restore case
      up. Game pages are `data:text/html,…` URLs — no window manager runs, so
      nothing needing a real minimise or a real game login is checkable here.
      Never `pkill -f` a target string directly in a Bash call; kill by stored
      PID or `pgrep -x`.

## 01 — The workspace in the domain, and coming back from one

- [x] `cargo test -p idle-manager-core` → `test result: ok. 65 passed; 0
      failed`, including the ten `session::tests` that pin every acceptance
      criterion: restore order and settings, running → `Queued`, parked →
      `Parked`, the start order and a parked account's absence from it, a saved
      slot the layout cannot show going off-grid then reclaimed, a fresh
      identifier after a restore, the `workspace()` round trip, a `Starting`
      account written as running, and an empty workspace.
- [x] `cargo clippy -p idle-manager-core --all-targets -- -D warnings` →
      `Finished` with no warning, so `Workspace`, `Account`, `SavedLiveness` and
      the `WorkspaceStore` port meet the code standards.

## 02 — The workspace file on disk

- [x] `cargo test -p idle-manager-store` → `test result: ok` for every binary,
      including `the-workspace-file-on-disk` (9 integration cases: the field-for
      -field round trip, the `insta` snapshot with `version = 1` as line one,
      the missing case distinct from `Ok(None)` at the port, an invalid file
      reported malformed and moved to `sessions.bad` with its bytes still
      readable, an unknown `version` kept aside its own way, an unrepresentable
      liveness and an unknown key each a parse failure, no `.tmp` left after two
      writes, and off-grid vs in-slot visibility surviving the round trip) and
      the `paths` unit test placing `sessions.toml` beside `presets/` under XDG
      config.
- [x] `cargo clippy -p idle-manager-store --all-targets -- -D warnings` →
      `Finished` with no warning.

## 03 — The queued row and the queued slot

- [x] `cargo test -p idle-manager-shell` → `test result: ok. 18 passed` for the
      lib, including: `status_key` → `"queued"` for a queued account whatever its
      slot or focus; `status_label("queued")` → `"Queued"`; `action_label` →
      `"Start"` and `action_sensitive` → `false` for a queued account;
      `name_markup` for a queued account equals the parked one (dimmed under the
      one existing not-running rule); `placeholder_panel(Queued)` → the `"Queued"`
      line with `button_visible = false` while `placeholder_panel(Parked)` is
      unchanged (`"Parked"`, button visible and sensitive); and the compiled
      bundle carries `.status-queued { #dc8add }`, distinct from the starting
      `#62a0ea` and parked `#9a9996`.
- [x] `make verify` exits `0` — fmt, clippy, the 117-test suite, audit,
      arch-check and roadmap-check all pass with the queued vocabulary in place.

## 04 — Restoring the arrangement on launch

Run headless on `:95` against throwaway `XDG_CONFIG_HOME` / `XDG_DATA_HOME`
trees, `sessions.toml` written by hand. Recorded 2026-09-07.

- [x] `cargo test -p idle-manager-shell --lib` → `test result: ok. 19 passed`,
      including `the_message_strip_template_is_readable_from_the_registered_bundle`.
- [x] Workspace file: three accounts, `layout = "grid"`, `session-0001` and
      `session-0002` `running` in slots 0 and 1, `session-0003` `parked` in slot
      3. Launch → the sidebar lists Main, Alt, Farm; the header's **4** toggle is
      active; Main fills slot 0, Alt slot 1, Farm slot 3, slot 2 empty; no web
      view is loaded (`workspace restored accounts=3` in the log, no `load
      changed`).
- [x] Main and Alt come back queued: purple sidebar dots, dimmed names; each
      slot shows the name, the line **Queued**, and no button.
- [x] Farm comes back parked: grey sidebar dot; its slot shows **Parked** with a
      pressable **Start** — parked from the first frame, never a queued or
      starting marker.
- [x] Farm's ⋯ menu → **Start** loads its page into slot 3, the placeholder
      clears, and its row goes green. (Alt's ⋯ menu shows **Start** greyed while
      it is queued — the queue owns that turn.)
- [x] A restored parked account with `zoom = 0.5` and
      `user_agent = "IdleManagerProbe/9.9 (only-this)"`, started from its
      placeholder button: the log reads `account identity override applied
      user_agent="IdleManagerProbe/9.9 (only-this)"` then `page console error
      text="UAECHO=IdleManagerProbe/9.9 (only-this)"` (exact string, nothing
      appended), and its `<h1>` renders visibly small beside a `zoom = 1.0`
      account's placeholder heading.
- [x] **Tester, real accounts:** a restored account whose game you were signed
      into loads straight past the login when started — no new sign-in. Needs
      real credentials against a real game; not checkable headless.
- [x] No `sessions.toml` at all → the window opens as item 01's empty state
      ("No games yet…", "Add your first game"), no strip, the **1** toggle
      active. Log: `no saved workspace; opening a first run`.
- [x] A `sessions.toml` that will not parse (`name = broken no quotes [[[`) →
      a warning-tinted strip spans the window directly under the header bar,
      above the sidebar and the grid, reading "The saved workspace could not be
      read. It was kept aside at …/sessions.bad and the window below is a first
      run.", with a ✕ on its trailing edge; the window below is a first run; the
      original bytes are at `sessions.bad` and `sessions.toml` is gone. Pressing
      ✕ removes the strip and it does not return.
- [x] With one instance running on a shared session bus, a second launch of the
      binary on the same bus logs only `starting idle-manager`, exits `0`
      without ever logging `activated; presenting the main window`, and the
      first instance keeps running — no second process, no second window.

## 05 — The start queue

Run headless on `:95`; game pages are `data:text/html,…` URLs, a hung page is
one with `<img src="http://10.255.255.1/…">`. Recorded 2026-09-07.

- [x] `cargo test -p idle-manager-shell --lib start_queue` → the two `next_up`
      cases pass: it skips an identifier the book no longer reports as queued
      and returns `None` when none remain.
- [x] Workspace of four accounts, slots 0–3, `session-0002` parked and the rest
      running. Launch → the log shows `start queue: started … session-0001`,
      then `… session-0003`, then `… session-0004`, then `nothing queued;
      restoration finished` — one at a time, in book order, `session-0002`
      never started.
- [x] Mid-drain screenshot: `session-0002` and the not-yet-reached accounts
      show their **Queued** panels with no button; the account in progress shows
      its slot (the "Starting" panel is up only until the page's first commit —
      item 03's accepted behaviour, inherited unchanged — after which the
      painting page shows).
- [x] `session-0002` (parked) keeps its grey dot and **Parked** panel with a
      pressable **Start** through the whole restore; the queue never touches it.
- [x] After the drain: no row reads queued, no panel stands over a loaded game,
      and parking / starting by hand work as before (verified in task 04's run).
- [x] Two accounts, the first with a hung `<img>`: the log shows
      `start queue: started … session-0001` at `T`, then exactly
      `LOAD_SETTLE_TIMEOUT_SECS` (30 s) later `a page did not settle in time;
      moving on` and `start queue: started … session-0002`, then `restoration
      finished`. The hung account holds the queue no longer than the timeout.
- [x] Killing the process while `session-0001` is still settling leaves the log
      at `start queue: started …` with no error and no further `start queue`
      lines — the restore ends and nothing else starts.
- [x] **Tester, real game:** `LOAD_SETTLE_TIMEOUT_SECS` is a provisional 30 s
      fallback. Time three real idle games from `start queue: started` to
      `load changed event=Finished` on a normal connection and set the constant
      from that measurement (the roadmap item's third blocker); the queue's
      one-at-a-time behaviour above does not depend on the exact value.

## 06 — Saving after every change

Run headless on `:95` against a throwaway `XDG_CONFIG_HOME`; `pyin.fastclick`
issues a burst with no delay between clicks. Recorded 2026-09-07.

- [x] Add an account, park one, start one, toggle "Keep running when hidden",
      switch arrangement — each writes `sessions.toml` (`workspace saved` in the
      log). Close the window (its header ✕) and reopen: the grid layout, the
      running Alpha, the parked Beta and Beta's keep-awake mark all come back.
- [x] Five layout-toggle `fastclick`s in ~16 ms → the log shows exactly one
      `workspace saved` ~400 ms later, and the file holds the last layout.
- [x] Switch layout then click ✕ within the 400 ms window → the log shows
      `workspace flushed on close` and the file holds the new layout: the last
      change before quitting survived.
- [x] During the five-click burst the header toggles keep updating and no click
      is dropped; the write runs on `gio::spawn_blocking`, so the main context
      is never blocked.
- [x] After every burst, `ls <config>/idle-manager/*.tmp` is empty and
      `sessions.toml` parses (`grep '^layout'` returns a value each time).
- [x] `chmod 555` the configuration directory, then switch layout → the strip
      reads "The arrangement could not be saved: could not write the workspace
      file …sessions.toml.<pid>.tmp: Permission denied (os error 13)", the same
      is logged at `warn` with the path, and the layout still changes — the
      program keeps working.
- [x] Reopen a workspace of two `running` accounts behind a hung `<img>`, then
      click ✕ while the second is still queued → the file keeps both at
      `liveness = "running"` (a queued or starting account is flattened to
      running by `SessionBook::workspace`).
- [x] Press the strip's ✕ to dismiss the failed-save message, `chmod 755` the
      directory back, switch layout again → the save succeeds (`workspace
      saved`) and the strip stays dismissed.
- [x] **Tester, real desktop:** confirm on a normal window-managed session that
      switching arrangement, dragging a game between slots and adding an account
      never stutter the window while a write is in flight.

## Teardown

- [x] Kill the harness: `kill` the stored Xvfb and `idle-manager` PIDs (or
      `pgrep -x`), `rm -f /tmp/.X95-lock`, `chmod 755` any directory a test made
      read-only, remove the throwaway `XDG_CONFIG_HOME` / `XDG_DATA_HOME` trees.
      Nothing is left under the real config or data directory.
- [x] Slices 01–03 touch no disk outside `cargo`'s target directory (the
      integration tests clean their own throwaway directories on drop).
