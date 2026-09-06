# Test script — 03 Parking and unparking a session

A hand-run runbook that proves the item works end to end, beside the tasks'
`(unit)` / `(manual)` criteria rather than replacing them. Every line is one
concrete action and the observable result it must produce. Tick a box only
after the step has actually been run.

`## Setup` and `## Teardown` are shared and written once; each task appends its
own `## NN — Title` section and never rewrites another's.

The run recorded here drove the shell headless (`Xvfb` + `ffmpeg` screenshots +
a `dlopen`-XTest input helper — see the `verifying-shell-slices-headless`
note) against public pages (`http://example.com` / `.org` / `.net`) that need
no login. Every box left unchecked needs a human at a screen with a real game
account they can log into — the login-survival, no-login-prompt and sign-in
popup checks cannot be made without credentials.

## Setup

- [x] `export PATH="$HOME/.cargo/bin:$PATH"` then `cargo --version` prints a version
- [x] `make verify` from the repo root exits `0` — fmt-check, clippy, tests,
      audit, arch-check and roadmap-check all pass
- [x] `Xvfb :99 -screen 0 1280x800x24 & echo "XVFB=$!"` — note the printed PID as `$XVFB`
- [x] `rm -rf /tmp/im-test-data && mkdir -p /tmp/im-test-data` — an empty throwaway XDG data home
- [x] `dbus-run-session -- env DISPLAY=:99 GDK_BACKEND=x11 GSK_RENDERER=cairo XDG_DATA_HOME=/tmp/im-test-data RUST_LOG=idle_manager_shell=debug,idle_manager=debug cargo run -p idle-manager & echo "APP=$!"`
      — the main window appears on `:99`; note the PID as `$APP`.
      (`dbus-run-session` isolates the D-Bus name so a stray earlier instance
      does not swallow the activation.)
- [x] screenshot with `ffmpeg -y -f x11grab -video_size 1280x800 -i :99.0 -frames:v 1 "$HOME/im-shot.png"`
      and open it — the empty-state ("No games yet") is shown
- [x] add three accounts through **Add game**: names `Acct A`, `Acct B`,
      `Acct C`. For the login-gated checks below, point each at a real idle game
      you can log into; the run recorded here used `http://example.com` / `.org`
      / `.net`. Three rows appear in the sidebar, each with a `Park` button.
- [ ] log each of the three accounts into its game so the "no login prompt"
      checks are meaningful; `find /tmp/im-test-data -name cookies.sqlite -size +1c`
      lists three non-empty cookie databases
- [x] `ps -C WebKitWebProcess` lists exactly three resident web processes
      (one per account, descendants of `$APP`)
- [x] click the header-bar **2** toggle — `Acct A` and `Acct B` fill the two
      slots, `Acct C`'s sidebar row reads `Background`

## Teardown

- [x] `kill "$APP"` (the stored PID — never `pkill -f target/debug/idle-manager`,
      which also matches the driving shell) — the window closes
- [x] `kill "$XVFB"` — the display server stops
- [x] `rm -rf /tmp/im-test-data "$HOME/im-shot.png"` — the throwaway profile tree and screenshots are gone

## 01 — Liveness in the session book

- [x] `cargo test -p idle-manager-core` exits `0` and these eight tests pass:
      `a_session_added_to_the_book_starts_live`,
      `parking_a_live_session_returns_parked`,
      `parking_a_session_leaves_its_visibility_unchanged`,
      `unparking_a_parked_session_returns_starting`,
      `unparking_a_session_leaves_its_visibility_unchanged`,
      `ending_the_starting_interval_returns_live`,
      `parking_an_already_parked_session_returns_parked_and_changes_nothing_else`,
      `unparking_a_live_session_returns_live_and_changes_nothing_else`

## 02 — The session view holder and the cache model

- [x] on launch every account loads its page as before; `ps -C WebKitWebProcess`
      shows exactly one web process per account (three accounts → three
      processes, each a descendant of `$APP`)
- [x] the sidebar draws `Current` (green glow, bold name), `Visible` (green),
      `Background` (amber glow, dimmed name) and the empty-list line exactly as
      before the restructure (screenshot `01-launch` / `05-three-accts-2up`)
- [ ] a login made in an account survives quitting and restarting the
      application — after restart the game loads straight in with no login page,
      and its `cookies.sqlite` under `/tmp/im-test-data` is unchanged
- [ ] a sign-in popup still opens from a game's login button and shares the
      opening account's session
- [ ] first load of one game is timed before and after the
      `CacheModel::DocumentViewer` change (stash the `configure_web_engine`
      call, rebuild, time; restore, rebuild, time) and both numbers recorded
      here. Recorded so far (public page, `http://example.com`, not a real
      game): Started→Finished ≈ 79 ms first load, ≈ 73 ms on a post-park
      restart — the cache-model applied cleanly (no "no default
      WebKitWebContext" warning) and every load completed.
- [x] `kill -9` on one account's `WebKitWebProcess` is logged by the terminated
      handler as `ERROR ... reason=Crashed`, distinct from a park's
      `DEBUG ... terminated by API (parked)`, and no reload is attempted

## 03 — Parking a running account

- [x] `cargo test -p idle-manager-shell session_sidebar::row` passes:
      `a_parked_account_keys_as_parked_whatever_its_visibility` and
      `a_live_account_keeps_the_current_visible_and_background_keys`
- [x] every running account's row shows a `Park` button on its trailing edge,
      after the state marker (screenshots `04-one-account`, `05-three-accts-2up`)
- [x] pressing `Park` leaves the row reading `Parked` with a grey unlit dot and
      the name in the dimmed style (screenshot `06-acctC-parked`)
- [x] pressing `Park` ends the account's rendering process: recorded the
      account's `WebKitWebProcess` RSS before (≈ 180 MB) and after — the
      process is gone from `ps` and the memory is returned to the system; the
      log shows `terminated by API (parked)` then `showing the slot placeholder`
- [x] parking an account that holds a slot leaves every other slot's game
      running and untouched, and the parked slot shows the account's name
      (parked `Acct A`, `Acct B`'s game kept rendering — screenshot
      `09-acctA-parked-panel`)
- [x] a parked account keeps its place: its row still sits where it did, and
      switching layouts 1 → 4 → 2 moves its slot exactly as a running account's
      (screenshots `11-parked-layout1`, `12-parked-layout4`)
- [x] parking an out-of-sight account (`Acct C`, `Background`) changes nothing
      on screen except its own row (screenshot `06-acctC-parked` vs
      `05-three-accts-2up`)

## 04 — Starting a parked account

- [x] `cargo test -p idle-manager-shell session_sidebar::row` passes
      `a_starting_account_keys_as_starting_whatever_its_visibility`,
      `the_action_label_inverts_with_liveness` and
      `the_action_button_is_insensitive_only_while_starting`
- [x] a parked account's row button reads `Start`, a running account's reads
      `Park` (screenshots `06-acctC-parked`, `08-acctC-restarted`)
- [ ] pressing `Start` loads the game already logged in — no login prompt
      appears (needs a real logged-in account; with a public page the load
      completes and the same start address is re-fetched — log shows
      `example.net` reloaded after `view attached behind the placeholder`)
- [ ] from the press until the page paints the marker reads `Starting` with a
      blue dot and the button is insensitive. Verified by the log order
      (`starting: view attached behind the placeholder` → `Committed` →
      `mark_started`) and the unit test; the blue `Starting` dot was not
      screenshot-caught because a public page commits in ≈ 30 ms. Re-check
      against a real game, which loads slowly enough to see it.
- [x] once the page paints the marker reads as running plus the account's place
      and the button reads `Park` again (screenshot `08-acctC-restarted` —
      `Acct C` back to `Background`, `10-acctA-restarted-from-panel` — `Acct A`
      back to `Current`)
- [x] `ps` shows a new `WebKitWebProcess` for the account after starting, RSS
      ≈ 181 MB — comparable to the ≈ 180 MB it held before parking
- [x] starting an out-of-sight account leaves it out of sight (`Acct C` stayed
      `Background`); starting one holding a slot puts the new view in that same
      slot and leaves every other slot untouched (`Acct A` returned to slot 0,
      `Acct B` untouched)

## 05 — The parked slot placeholder

- [x] `cargo test -p idle-manager-shell` passes
      `the_slot_placeholder_template_is_readable_from_the_registered_bundle` —
      `ui/slot-placeholder.ui` resolves from the registered `GResource` bundle
- [x] parking an account that holds a slot replaces its game with a panel
      showing the account's name, a line reading `Parked` and a `Start` button,
      centred on the window's background (screenshot `09-acctA-parked-panel`)
- [x] pressing the panel's `Start` starts the account, and the new view
      replaces the panel once the page paints (screenshot
      `10-acctA-restarted-from-panel`; log `starting: view attached behind the
      placeholder session=session-0001` → `example.com` load)
- [ ] while the account is starting the panel's line reads `Starting` and its
      button is insensitive — same fast-page limitation as task 04's marker
      check; the code sets `state-text` to `Starting` and
      `button-sensitive` to `false` in `attach_view` before the load. Re-check
      against a real game.
- [ ] the panel's state line and button label are `glib` properties: with
      `GTK_DEBUG=interactive`, `state-text` and `button-label` show on the
      `IdleManagerSlotPlaceholder` widget and editing them changes the panel
      with no markup change
- [x] switching layouts carries the parked account's panel with its slot
      (1 → 4 → 2 kept `Acct A`'s panel in slot 0 — screenshots
      `11-parked-layout1`, `12-parked-layout4`); moving a parked account fully
      out of sight leaves its old slot empty — re-check by focusing another
      account into a parked-in-slot account's slot
