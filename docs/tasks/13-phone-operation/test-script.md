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

## Teardown

- [ ] `rm -rf /tmp/frames-13` and remove any `[listen]` override added to `~/.config/idle-manager/phone.toml` for testing
