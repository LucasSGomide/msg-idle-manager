# Test script — 03 Parking and unparking a session

A hand-run runbook that proves the item works end to end, beside the tasks'
`(unit)` / `(manual)` criteria rather than replacing them. Every line is one
concrete action and the observable result it must produce. Tick a box only
after the step has actually been run.

`## Setup` and `## Teardown` are shared and written once; each task appends its
own `## NN — Title` section and never rewrites another's.

## Setup

- [ ] `export PATH="$HOME/.cargo/bin:$PATH"` then `cargo --version` prints a version
- [ ] `make verify` from the repo root exits `0` — fmt-check, clippy, tests,
      audit, arch-check and roadmap-check all pass
- [ ] `Xvfb :99 -screen 0 1280x800x24 & echo "XVFB=$!"` — note the printed PID as `$XVFB`
- [ ] `rm -rf /tmp/im-test-data && mkdir -p /tmp/im-test-data` — an empty throwaway XDG data home
- [ ] `env DISPLAY=:99 GDK_BACKEND=x11 GSK_RENDERER=cairo XDG_DATA_HOME=/tmp/im-test-data RUST_LOG=idle_manager_shell=debug,idle_manager=debug cargo run -p idle-manager & echo "APP=$!"`
      — the main window appears on `:99`; note the PID as `$APP`
- [ ] screenshot with `ffmpeg -y -f x11grab -video_size 1280x800 -i :99.0 -frames:v 1 "$HOME/im-shot.png"`
      and open it — the empty-state ("No games yet") is shown
- [ ] add three accounts through **Add game**, each pointing at a real idle game
      you can log into (three logins of the same game is fine): names `Acct A`,
      `Acct B`, `Acct C` — three rows appear in the sidebar
- [ ] log each of the three accounts into its game so a later "no login prompt"
      check is meaningful; `find /tmp/im-test-data -name cookies.sqlite -size +1c`
      lists three non-empty cookie databases
- [ ] `ps -o rss= -C WebKitWebProcess` lists exactly three resident processes
- [ ] click the header-bar **2** toggle — `Acct A` and `Acct B` fill the two
      slots, `Acct C`'s sidebar row reads `Background`

## Teardown

- [ ] `kill "$APP"` (the stored PID — never `pkill -f target/debug/idle-manager`,
      which also matches the driving shell) — the window closes
- [ ] `kill "$XVFB"` — the display server stops
- [ ] `rm -rf /tmp/im-test-data "$HOME/im-shot.png"` — the throwaway profile tree and screenshots are gone

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
