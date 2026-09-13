# Test script — 10, renaming and rearranging accounts

Hand-run runbook for this item, per CLAUDE.md's planning workflow. Steps are
recorded after the fact from this item's live testing (2026-09-13); each one
reflects something actually run and observed, not a prescription for a future
run.

## Setup

- [x] Build the workspace: `cargo build --workspace` — compiles clean,
      producing `target/debug/idle-manager`.
- [x] Launch the app headless: `Xvfb :98 -screen 0 1280x800x24 -noreset &`,
      then inside `dbus-run-session -- env DISPLAY=:98 GDK_BACKEND=x11
      GSK_RENDERER=cairo XDG_CONFIG_HOME=<tmp>/config
      XDG_DATA_HOME=<tmp>/data ./target/debug/idle-manager` — a screenshot
      (`ffmpeg -f x11grab -video_size 1280x800 -i :98.0 -frames:v 1 out.png`)
      shows the window with "No games yet — add one to get started."

## Teardown

- [x] Close the app window and kill the `Xvfb` process — both exit; a fresh
      `Xvfb :98 -noreset` on the next run starts cleanly with no stale lock.

## 01 — One rule for an account name, and renaming in the book

- [x] Run `cargo test -p idle-manager-core` — 111 tests pass, including the 13
      new ones for `account_name` and `SessionBook::rename`
      (`account_name_trims_surrounding_whitespace`,
      `account_name_is_none_for_an_empty_or_whitespace_only_string`,
      `renaming_with_a_padded_name_stores_the_trimmed_name_and_returns_true`,
      `renaming_changes_only_the_display_name`,
      `renaming_with_an_empty_or_whitespace_only_name_returns_false`,
      `renaming_with_an_empty_name_leaves_the_book_unchanged`,
      `renaming_an_unknown_id_returns_false`,
      `renaming_an_unknown_id_leaves_the_book_unchanged`,
      `renaming_a_parked_account_keeps_it_parked`,
      `renaming_a_queued_account_keeps_it_queued`,
      `renaming_to_a_name_another_account_already_has_is_accepted`,
      `the_workspace_after_a_rename_carries_the_new_name`).
- [x] Run `cargo build --workspace` after the add-game dialog change
      (`crates/idle-manager-shell/src/add_game_dialog/imp.rs` now calls
      `idle_manager_core::account_name` instead of repeating the trim rule) —
      the whole workspace, including `idle-manager-shell`, compiles clean.
- [x] With the app running per Setup, click "Add game", choose "Something
      else…", type three spaces into "Name for this account" and a valid
      address into "Address" — the "Add" button stays insensitive (greyed
      out) with the address alone filled in.
- [x] Clear the name field and type "  Padded Name  " (two leading and two
      trailing spaces) — the "Add" button turns sensitive (solid blue).
- [x] Click "Add" — `XDG_CONFIG_HOME/idle-manager/sessions.toml` records
      `name = "Padded Name"` with no leading or trailing whitespace,
      confirming the add-game dialog's stored name is trimmed by the same
      rule `account_name` states.
