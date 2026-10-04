# Test script — 17 Muting one account's sound

## Setup

- [x] `git switch feat/17-account-sound-control` → prints `Already on 'feat/17-account-sound-control'` (or switches to it); `git branch --show-current` shows `feat/17-account-sound-control`

## 01 — The muted flag in the domain and on disk

- [x] `cargo nextest run -p idle-manager-core -p idle-manager-store` → last line reads `315 tests run: 315 passed, 0 skipped`
- [x] `cargo nextest run -p idle-manager-store the_written_file_has_a_muted_key a_file_with_no_muted_account_has_no_muted_key the_written_file_matches_its_snapshot` → `3 tests run: 3 passed, 83 skipped`; the first writes a two-account workspace (one muted) to a temp dir and asserts the TOML holds `muted = true` exactly once, the second asserts an unmuted file has no `muted` key, the third is the unchanged snapshot test
- [x] `git status --porcelain crates/idle-manager-store/tests/snapshots` → prints nothing (no `.snap` change)

## Teardown

- [ ] `git status --porcelain` → only the two untracked `docs/prompts/17-*` and `18-*` files plus this task's changes
