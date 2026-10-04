# Test script — 17 Muting one account's sound

## Setup

- [x] `git switch feat/17-account-sound-control` → prints `Already on 'feat/17-account-sound-control'` (or switches to it); `git branch --show-current` shows `feat/17-account-sound-control`

## 01 — The muted flag in the domain and on disk

- [x] `cargo nextest run -p idle-manager-core -p idle-manager-store` → last line reads `315 tests run: 315 passed, 0 skipped`
- [x] `cargo nextest run -p idle-manager-store the_written_file_has_a_muted_key a_file_with_no_muted_account_has_no_muted_key the_written_file_matches_its_snapshot` → `3 tests run: 3 passed, 83 skipped`; the first writes a two-account workspace (one muted) to a temp dir and asserts the TOML holds `muted = true` exactly once, the second asserts an unmuted file has no `muted` key, the third is the unchanged snapshot test
- [x] `git status --porcelain crates/idle-manager-store/tests/snapshots` → prints nothing (no `.snap` change)

## 02 — Page-level mute on both engines and the holder

- [x] `cargo nextest run -p idle-manager-shell` → last line reads `Summary [   0.559s] 162 tests run: 162 passed, 0 skipped`
- [x] `grep -rn "set_is_muted\|ICoreWebView2_8" crates/idle-manager-shell/src --include=*.rs` → every hit is under `crates/idle-manager-shell/src/web_engine/`: `webview2.rs`, `webkit.rs` (`self.view.set_is_muted(on);`), `webview2/ffi.rs` (the import and `let webview: ICoreWebView2_8 = webview.cast()?;`), `webview2/host/imp.rs`, `webview2/host.rs`; nothing in `web_view.rs` or `window/`
- [x] `make arch-check` → prints `arch-check: layer boundaries hold`
- [ ] `make windows-check` → `cargo xwin clippy` finishes with no warnings (needs `clang-cl` and `lld-link`; not run on this machine)

## Teardown

- [ ] `git status --porcelain` → only the two untracked `docs/prompts/17-*` and `18-*` files plus this task's changes
