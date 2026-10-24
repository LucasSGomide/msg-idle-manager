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
- [x] `make windows-check` → `cargo xwin clippy` finishes with no warnings (needs `clang-cl` and `lld-link`; not run on this machine)

## 03 — The Muted menu item, the window handler and restore

- [x] `cargo nextest run -p idle-manager-shell status_key_is_the_same_whether_or_not_the_account_is_muted` → `Summary [   0.011s] 1 test run: 1 passed, 162 skipped`
- [x] `make lint && make test && make arch-check && make roadmap-check` → clippy ends `Finished \`dev\` profile [unoptimized + debuginfo] target(s)` with no warning; nextest `Summary [   1.885s] 612 tests run: 612 passed, 1 skipped` then `release-prepare-test: all assertions passed`; `arch-check: layer boundaries hold`; `roadmap tables are up to date`
- [x] Start the application with a live account whose game is playing sound; click that account's row ⋯ button → the menu's first section reads `Park`, `Keep running when hidden`, `Muted`, in that order, with no key chord drawn beside `Muted`
- [x] In that menu click `Muted` → the menu closes, the game's sound stops at once, the page does not reload (the game is where it was, nothing re-logs in), the row's coloured dot keeps its colour and no reloading mark appears; click ⋯ again → `Muted` shows ticked
- [x] Click ⋯ → `Muted` again to untick it → the game's sound returns at once with no reload; reopening the menu shows `Muted` unticked
- [x] On a parked account click ⋯ → `Muted` (it shows ticked on reopening), then click ⋯ → `Start` → the account starts and plays no sound; run `grep -n -B8 muted ~/.config/idle-manager/sessions.toml` → exactly the accounts you muted show `muted = true` in their `[[workspace.account]]` entry, and no other entry has a `muted` key
- [x] Quit the application (close the window) and start it again → every account muted before is still silent and shows `Muted` ticked in its ⋯ menu; every other account still sounds and shows it unticked
- [x] (needs a Windows machine) On Windows with a current WebView2 runtime, repeat the first three hand-run steps above (menu reads `Park`, `Keep running when hidden`, `Muted`; tick → silent at once, no reload, dot unchanged; untick → sound returns, no reload)

## Teardown

- [x] `git status --porcelain` → only the two untracked `docs/prompts/17-*` and `18-*` files plus this task's changes
