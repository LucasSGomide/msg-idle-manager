# Test script — 18 Drawing one account without the GPU

Run on the Linux laptop the fault was measured on. Every launch below is a
separate copy with its own folders, on its own D-Bus session so a running
Idle Manager is never handed the launch. `$T` is an empty scratch folder.
Steps ticked on 2026-10-24 were run then (run names refer to the measurement
log of `docs/prompts/19-…`); the clicks inside the game were made by an
injected script in a scratch build, never committed. The unticked steps need
a person at the window.

## Setup

- [x] `cargo build --release -p idle-manager` → `Finished release profile`
- [x] `mkdir -p $T/config/idle-manager $T/data/idle-manager/profiles $T/cache && cp -a ~/.local/share/idle-manager/profiles/session-0014 $T/data/idle-manager/profiles/session-0001` — a read-only copy of a signed-in Lux Idle profile; nothing is ever copied back
- [x] Write `$T/config/idle-manager/sessions.toml` with `version = 3`, one workspace (`expanded = true`) and one account `id = "session-0001"`, `url = "https://luxidle.com.br/jogar"`, `liveness = "running"`, `keep_awake = false`, `zoom = 1.0`
- [x] `XDG_CONFIG_HOME=$T/config XDG_DATA_HOME=$T/data XDG_CACHE_HOME=$T/cache dbus-run-session -- target/release/idle-manager` → a new window opens on Lux Idle's character list, signed in; a running Idle Manager's window is not raised
- [x] GPU objects of the copy's rendering process: `PID=$(pgrep -n -f WebKitWebProcess)` then `for f in /proc/$PID/fdinfo/*; do awk '/^drm-client-id/{id=$2} /^drm-total-(system|gtt)/{print id, $1, $2, $3}' $f; done | sort -u` → one line per client, summing to ~850 MiB in town with the flag off

## 01 — The flag in the domain and the workspace file

- [x] Add `software_rendering = true` under the account in `sessions.toml`, launch, quit → the account started, and the saved entry still carries exactly one `software_rendering = true` (run r24)
- [x] Launch without the key, turn the flag on in the running page (03's handler), quit → the saved entry now carries `software_rendering = true` between `keep_awake` and `zoom` (run r27)

## 02 — The preset key and the shipped Lux Idle preset

- [x] Launch with an empty `$T/config` → `$T/config/idle-manager/presets/` holds `baiaki-idle.toml`, `huntera.toml`, `lorvath.toml` and `lux-idle.toml`, and `lux-idle.toml` has `software_rendering = true`
- [ ] Main menu → Add account → choose `Lux Idle`, name it → the new row's ⋯ menu shows `Draw without the GPU` ticked

## 03 — The menu check and the engine policy

- [x] With `software_rendering = true` saved, launch, pick the character, click `GO HUNT` → the hunting list opens; the rendering process stays at 14 MiB of GPU objects and ~950 MiB PSS for two minutes, and the desktop stays responsive (run r24; the same click with the flag off reached 5.8 GiB in 0.4 s, run r08)
- [x] Flag off, in town: turn it on in the running page → the page reloads into the character list, GPU objects fall 847 → 418 MiB; pick the character and click `GO HUNT` → flat at 418 MiB for two minutes (run r27, the window's handler called from a timer)
- [x] Ask for the value already held a second time → no reload: the game stays where it is (run r27, 45 s after the first)
- [ ] Flag off, live account: open its row ⋯ menu, tick `Draw without the GPU` → the row shows the reloading mark, then live; reopen the menu → the item is ticked
- [ ] Park that account and start it again → the item is still ticked, and the GPU reading from Setup is under 50 MiB

## Teardown

- [x] Quit the copy and `rm -rf $T` → `~/.config/idle-manager/sessions.toml` and `~/.local/share/idle-manager/profiles/` of the real instance were never written by any step
