# Test script — 13 Operating an account from a phone

## Setup

- [x] `make build` → `Finished \`dev\` profile`
- [ ] Tailscale installed and signed in on the desktop and the phone; `tailscale ip -4` on the desktop prints a `100.x.y.z` address
- [ ] At least one account exists in `~/.config/idle-manager/sessions.toml` and is live

## 01 — The remote vocabulary, its two ports, and the phone record on disk

- [x] `cargo nextest run -p idle-manager-core remote::` → `20 tests run: 20 passed`
- [x] `cargo nextest run -p idle-manager-store --test the-phone-record-on-disk` → `16 tests run: 16 passed`
- [x] `cargo nextest run -p idle-manager-store the_file_is_created_readable_and_writable_by_the_owner_alone` → `PASS`; the test asserts the written `phone.toml` has mode `0600`
- [x] `cargo nextest run -p idle-manager-store a_malformed_file_is_moved_aside_and_reported_as_unreadable` → `PASS`; `phone.toml` is gone and `phone.toml.unreadable` holds the same bytes
- [x] `make arch-check` → `arch-check: layer boundaries hold`
- [x] Add `idle-manager-shell.workspace = true` under `[dependencies]` in `crates/idle-manager-remote/Cargo.toml`, run `./scripts/arch-check.sh` → stderr `arch-check: idle-manager-remote must not depend on idle-manager-shell`, exit 1; revert the line

## 02 — Capturing a frame, running a script and waking a view on both engines

- [x] `mkdir -p /tmp/frames-13 && IDLE_MANAGER_DUMP_FRAMES=/tmp/frames-13 RUST_LOG=idle_manager_shell=debug timeout 60 cargo run --package idle-manager` with at least one account in `sessions.toml` → the log shows one `frame dump armed session=session-NNNN directory=/tmp/frames-13` line per started account, then `frame written session=session-NNNN path=/tmp/frames-13/session-NNNN-0.ppm bytes=…` every 2 s per account (if another instance of the app is already running, GTK hands the launch to it and nothing is written — quit it first, or run under `dbus-run-session` with throwaway `XDG_*_HOME` directories)
- [x] `ls /tmp/frames-13` → files `session-NNNN-0.ppm`, `-1.ppm`, … numbered from 0 per account; `head -c 15 session-NNNN-0.ppm` prints `P6`, the slot size and `255`
- [x] Open one frame in an image viewer → it is a picture of the account's page as shown in the slot, not black or blank
- [x] `sha256sum` two consecutive frames → hashes differ when the page changed between captures and are equal when it did not
- [x] `IDLE_MANAGER_MINIMISE_AFTER_SECS=16` with the dump switch, two accounts on an animating local page (one keep-awake on, one off), 80 s under `dbus-run-session` with throwaway `XDG_*_HOME` → the log shows `debug switch: minimising the window after_secs=16`, 39 frames per account are written, and every frame's `sha256sum` differs from the previous one for both accounts through the minimised minute (2026-09-19: the engine paints a current picture of a minimised page; the first Blocker is cleared)
- [ ] `IDLE_MANAGER_DUMP_FRAMES=/nonexistent/dir …` → one `frame not written; the frame dump for this account stops here` warning per account and no further frame lines; the app keeps running

## 03 — The Mobile layout on the desktop

- [x] `cargo nextest run -p idle-manager-core -E 'test(/workspace_book::tests::.*(mobile|mode)/)'` → `16 tests run: 16 passed`
- [x] `cargo nextest run -p idle-manager-core -E 'test(/session::tests::zoom.*mobile/)'` → `3 tests run: 3 passed`; `zoom_in_in_mobile_returns_none_and_writes_no_mobile_key` proves `RememberedZoom` stays empty
- [x] `cargo nextest run -p idle-manager-store --test the-workspace-file-on-disk mobile` → `3 tests run: 3 passed`; the insta snapshot `the_workspace_file_on_disk__workspace-file-during-mobile-mode.snap` shows `layout = "grid"` and `slot = 0` / `slot = 1`, never `mobile`
- [x] `cargo nextest run -p idle-manager-shell session_grid::imp::tests` → `8 tests run: 8 passed`; `the_mobile_slot_is_the_viewport_centred_horizontally_and_top_aligned` pins `x=294 y=0 412×915` in a 1000×800 grid
- [x] Copy `~/.config/idle-manager/sessions.toml` and `presets/` into a throwaway `$T/config/idle-manager/`, then `XDG_CONFIG_HOME=$T/config XDG_DATA_HOME=$T/data XDG_CACHE_HOME=$T/cache RUST_LOG=idle_manager_shell=debug IDLE_MANAGER_DEBUG_LAYOUT=mobile timeout 40 dbus-run-session -- target/debug/idle-manager` → the log shows `debug switch: selecting the layout toggle layout=Mobile` at ~5 s, then `mobile slot allocated size=412x915 x=334 y=0 grid_width=1080 grid_height=753` (clipped, not scaled), then `workspace saved`; `grep -c Gtk-CRITICAL` → `0`
- [x] After that run, `diff` the throwaway `sessions.toml` against its pre-run copy → no output; a save made during mobile mode records the pre-mobile layouts
- [ ] With a screen: press `Phone` → one outlined 412 × 915 slot centred at the top with the focused game inside, no grip on hover, bottom edge clipped at the default 800 px height; `Ctrl`+`+` and `Ctrl`+wheel over it change nothing and flash no readout
- [ ] Press `2` → the two-slot arrangement returns with the same accounts in the same slots; press `Phone`, quit, relaunch → the window opens in `2`

## 04 — The remote server: enrolment, the socket and its proof

- [x] `cargo nextest run -p idle-manager-remote` → `85 tests run: 85 passed, 1 skipped`
- [x] `IDLE_MANAGER_REMOTE_HOLD_SECS=90 cargo nextest run -p idle-manager-remote --run-ignored only --no-capture` → stderr `listening on 127.0.0.1:7466` then `enrol at http://127.0.0.1:7466/enrol/<64 hex> (valid 600 s)`; the server stays up for 90 s
- [x] `curl -si http://127.0.0.1:7466/enrol/<that code>` → `HTTP/1.1 200 OK`, `Content-Type: text/html; charset=utf-8`, `Set-Cookie: idle-manager-phone=<32 hex>; Path=/; Max-Age=31536000; SameSite=Strict; HttpOnly`; body holds `<meta name="idle-manager-device-id" content="<32 hex>">` and `<meta name="idle-manager-secret" content="<64 hex>">`
- [x] The same `curl` a second time → `HTTP/1.1 404 Not Found`, `Content-Length: 0`, no `Server:` line, no body
- [x] `curl -si http://127.0.0.1:7466/` and `curl -si http://127.0.0.1:7466/anything` without a cookie → both `HTTP/1.1 404 Not Found`, `Content-Length: 0`, no `Server:` header
- [x] `curl -si -H 'Cookie: idle-manager-phone=<device id>' http://127.0.0.1:7466/` → `HTTP/1.1 200 OK`, no `Set-Cookie`, body holds `<title>Idle Manager</title>` and an empty `idle-manager-secret` meta
- [x] `curl -si --max-time 2 -H 'Cookie: idle-manager-phone=<device id>' -H 'Upgrade: websocket' -H 'Connection: Upgrade' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' -H 'Sec-WebSocket-Version: 13' http://127.0.0.1:7466/ws` → `HTTP/1.1 101 Switching Protocols`, `Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`

## 05 — The phone page

- [x] `cargo nextest run -p idle-manager-remote` → `94 tests run: 94 passed, 1 skipped`; the seven `the-phone-pages-pure-script-under-node` tests print `PASS` (with `node` hidden from `PATH` each prints `note: … is skipped` and still passes)
- [x] With the hand-run server up (`IDLE_MANAGER_REMOTE_HOLD_SECS=150 cargo nextest run -p idle-manager-remote --run-ignored only --no-capture`), `curl -si http://127.0.0.1:7466/enrol/<code>` → `200`, `Set-Cookie: idle-manager-phone=<32 hex>`; the body is balanced HTML with no `{{` and both metas filled; `curl -s -H 'Cookie: idle-manager-phone=<id>' http://127.0.0.1:7466/ | grep -c <secret hex>` → `0`
- [x] A Node WebSocket client built from the page's own `// BEGIN pure` block against that server: `hello` → `auth` → `welcome` with the proof verified and `state.mobileMode` false; the same with a wrong secret → the socket closes with no `welcome`
- [ ] On the phone (Android, then iPhone), open the enrolment address from the desktop's phone dialog → the page shows `Mobile mode`, a large centred switch (off) and the dim line `Turn it on to see a game here`; the address bar now reads `/`; "Add to Home Screen", open it from there → standalone, same screen
- [ ] Tap the switch → the desktop enters the `Mobile` layout at the phone's screen size and the page shows the current game filling the screen within a second; a short touch on a game button takes effect; a drag scrolls the page; the top-left round handle opens the list
- [ ] In the list: workspace names as plain headings, each account with its name and the word `live`/`parked`/`starting`/`queued` beneath, `Park` on live rows and `Start` on the others (insensitive for `starting`/`queued`), the current row highlighted; tap another name → the other game appears; tap `Park` on the current row → its word turns `parked` and the game screen becomes the name, `Parked` and a `Start` button; a workspace with no accounts shows `No games in this workspace`
- [ ] Switch to another app and back → if the socket dropped, the dimmed last picture under `Reconnecting…` for a few seconds, then pictures resume on the same account
- [ ] `Un-enrol the phone` on the desktop → the page shows only `This phone is no longer enrolled`; reopening it shows the same line

## Teardown

- [ ] `rm -rf /tmp/frames-13` and remove any `[listen]` override added to `~/.config/idle-manager/phone.toml` for testing
