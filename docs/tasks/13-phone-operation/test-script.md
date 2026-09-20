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

## 04 — The remote server: enrolment, the socket and its proof

- [x] `cargo nextest run -p idle-manager-remote` → `85 tests run: 85 passed, 1 skipped`
- [x] `IDLE_MANAGER_REMOTE_HOLD_SECS=90 cargo nextest run -p idle-manager-remote --run-ignored only --no-capture` → stderr `listening on 127.0.0.1:7466` then `enrol at http://127.0.0.1:7466/enrol/<64 hex> (valid 600 s)`; the server stays up for 90 s
- [x] `curl -si http://127.0.0.1:7466/enrol/<that code>` → `HTTP/1.1 200 OK`, `Content-Type: text/html; charset=utf-8`, `Set-Cookie: idle-manager-phone=<32 hex>; Path=/; Max-Age=31536000; SameSite=Strict; HttpOnly`; body holds `<meta name="idle-manager-device-id" content="<32 hex>">` and `<meta name="idle-manager-secret" content="<64 hex>">`
- [x] The same `curl` a second time → `HTTP/1.1 404 Not Found`, `Content-Length: 0`, no `Server:` line, no body
- [x] `curl -si http://127.0.0.1:7466/` and `curl -si http://127.0.0.1:7466/anything` without a cookie → both `HTTP/1.1 404 Not Found`, `Content-Length: 0`, no `Server:` header
- [x] `curl -si -H 'Cookie: idle-manager-phone=<device id>' http://127.0.0.1:7466/` → `HTTP/1.1 200 OK`, no `Set-Cookie`, body holds `<title>Idle Manager</title>` and an empty `idle-manager-secret` meta
- [x] `curl -si --max-time 2 -H 'Cookie: idle-manager-phone=<device id>' -H 'Upgrade: websocket' -H 'Connection: Upgrade' -H 'Sec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==' -H 'Sec-WebSocket-Version: 13' http://127.0.0.1:7466/ws` → `HTTP/1.1 101 Switching Protocols`, `Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=`

## Teardown

- [ ] `rm -rf /tmp/frames-13` and remove any `[listen]` override added to `~/.config/idle-manager/phone.toml` for testing
