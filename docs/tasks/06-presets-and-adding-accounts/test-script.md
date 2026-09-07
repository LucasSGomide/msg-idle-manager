# 06 — Presets and adding an account · test script

Hand-run runbook that proves the item works end to end. Each `## MM` section is
appended by the task of that number; Setup and Teardown are shared. A box is
ticked only after the step has actually been run.

## Setup

- [x] `export PATH="$HOME/.cargo/bin:$PATH"` and `cd` to the repo root, so
      `cargo` and `make` resolve.
- [x] `cargo build --workspace` completes without error — the whole workspace
      compiles with the preset code in place.
- [x] **Headless shell harness** (for the `(manual)` shell checks, per
      [[verifying-shell-slices-headless]]): `Xvfb :98 -screen 0 1280x800x24
      -noreset &`, then the app under
      `dbus-run-session -- env DISPLAY=:98 GDK_BACKEND=x11 GSK_RENDERER=cairo
      XDG_CONFIG_HOME=<tmp>/config XDG_DATA_HOME=<tmp>/data
      RUST_LOG=idle_manager=info,idle_manager_shell=debug
      ./target/debug/idle-manager`. Synthetic input via a ctypes XTest driver
      (`pyin.py`: `move`/`click`/`type`/`key`); frames via
      `ffmpeg -f x11grab -video_size 1280x800 -i :98.0 -frames:v 1`. An
      **echo page** for identity: a preset whose `url` is
      `data:text/html,<script>console.error("UAECHO="+navigator.userAgent)</script>`
      — the page-console bridge forwards `console.error` to `tracing` at `warn`,
      so `grep UAECHO` on the app log reads the identity the page saw. No window
      manager runs on `:98`, so nothing that needs a minimise or a real game
      login is checkable here.

## 01 — The preset in the domain and the catalogue port

- [x] `cargo test -p idle-manager-core` → `test result: ok. 55 passed; 0 failed`,
      including `preset::tests::*` (three zoom checks) and
      `session::tests::an_account_from_a_preset_*` (nine `add_from_preset` /
      typed-address checks that pin every acceptance criterion).
- [x] `cargo clippy -p idle-manager-core --all-targets -- --deny warnings` →
      `Finished` with no warning, so `PresetId`, `ZoomLevel`, `Preset` and the
      `PresetCatalogue` port meet the code standards.

## 02 — Reading the game files from the configuration folder

- [x] `cargo test -p idle-manager-store --test reading-the-game-files` →
      `test result: ok. 11 passed; 0 failed`. The eleven cases cover: three
      well-formed files sorted by name; every field mapped unchanged; a missing
      `user_agent` → no identity and an empty one → failure; an unparseable
      file and a missing key each one named failure with the rest still read;
      zero and negative zoom → failures; an empty directory → nothing; a
      non-`.toml` file ignored entirely; and the reported source plus the real
      presets directory resolving under XDG config, not data.
- [x] `cargo clippy -p idle-manager-store --all-targets -- --deny warnings` →
      `Finished` with no warning.

## 03 — The shipped game files and first-run seeding

- [x] `cargo test -p idle-manager-store --test shipped-games-and-seeding` →
      `test result: ok. 7 passed; 0 failed`. Covers: a missing path is created
      and gets exactly `baiaki-idle.toml`, `huntera.toml`, `lorvath.toml`; the
      seeded folder reads back as Huntera / Baiaki Idle / Lorvath sorted by
      name, each with its real address and no browser identity; a second
      construction over a hand-edited-and-partly-deleted folder puts nothing
      back; an existing empty folder is left empty; a read-only parent leaves
      the catalogue empty and construction still succeeds.
- [x] Snapshot `crates/idle-manager-store/tests/snapshots/shipped_games_and_seeding__shipped-huntera-toml.snap`
      is committed and `cargo test` (no `INSTA_UPDATE`) passes it, pinning
      `huntera.toml` byte for byte as it is written to disk.
- [x] **Zoom readings.** All three ship `zoom = 1.0`. The tester opened each game
      in a single full-window slot on their desktop (2026-09-07) and found all
      three comfortably readable at `1.0`, so no smaller multiplier was needed.
      `keep_awake` ships `false` for all three — none is known to lose progress
      while throttled.

## 04 — The add-game dialog's game chooser

Driven headless on `:98` with a throwaway `XDG_CONFIG_HOME`; the app seeds the
three shipped files into it on first launch.

- [x] Press "Add game" → the dialog's first stage is a `ListView` reading
      `Baiaki Idle`, `Huntera`, `Lorvath` — one row each, display name only,
      sorted by display name.
- [x] The row after the games reads `Something else…` with a hairline
      `GtkSeparator` above it; the separator is absent when no games list.
- [x] Click `Lorvath` → stage two shows one field, "Name for this account", with
      "Add" insensitive; typing `Alt` makes "Add" sensitive.
- [x] Click `Something else…` → the name **and** address fields show; with only
      the name filled "Add" stays insensitive (`e1-name-only`), with both filled
      it becomes sensitive (`e2-both-filled`).
- [x] Confirm with `Baiaki Idle` named `Alt` → an account is created, its sidebar
      row reads `Alt` (the typed name, not the game name), and the view loads
      `https://baiakidle.com/` (`f4-created`).
- [x] Confirm through `Something else…` with a name and `https://oddgame.example/`
      → an account is created exactly as the item-01 path did: sidebar row
      `OddOne`, the view attempts that address (`e4-final`).
- [x] Seed a preset with `keep_awake = true`, create an account from it → its row
      menu shows "Keep running when hidden" already checked (`a4-rowmenu`).
- [x] Seed a `broken.toml` (`url =` with no value) alongside a good file → the
      dialog opens, the good game lists, and one dim line under the list reads
      `broken.toml: TOML parse error …` (`a1-chooser-with-failure`).
- [x] Seed a presets folder whose only file is `broken.toml` → the list holds
      only `Something else…`, a line above it names the folder to add files to,
      and the failure line still shows (`b1-nothing-readable`).
- [x] With an account already added, write a new `late.toml` into the folder and
      press "Add game" again → `Late Addition` appears in the list with no
      restart (`a5-late-appears`).

## 05 — A view drawn at the game's zoom and browser identity

- [x] Two presets pointing at `https://example.org/`, one `zoom = 1.0` and one
      `zoom = 0.5`, added side by side → the `0.5` account's page is visibly
      smaller on its first paint, with no resize flash (`z1-side-by-side`).
- [x] An account added through `Something else…` draws at the engine's ordinary
      size (`e4-final`) and, like any preset with no `user_agent` key, presents
      the engine's own identity (see the echo check below).
- [x] Echo page, preset with **no** `user_agent` → log reads
      `UAECHO=Mozilla/5.0 (X11; Ubuntu; Linux x86_64) AppleWebKit/605.1.15 …
      Safari/605.1.15` — the engine's own, no substitution.
- [x] Echo page, preset with `user_agent = "IdleManagerProbe/9.9 (only-this)"` →
      log reads `UAECHO=IdleManagerProbe/9.9 (only-this)` — exactly that string,
      nothing appended (`set_user_agent`, not the `_with_application_details`
      form).
- [x] Park then start the identity-override account → the fresh page reports the
      same `UAECHO=ProbeUA/1.0`; the size and identity are re-applied by
      `SessionView::start`, not lost with the old view.
- [x] Each of the three shipped games loads at its file's zoom and reaches its
      sign-in without being turned away as an unrecognised browser — run by the
      tester on their desktop with real accounts, 2026-09-07.
- [x] A sign-in popup opened by an identity-override account reports the same
      string as the account that opened it — checked in the web inspector on
      opener and popup, 2026-09-07.
- [x] Edit a preset's `zoom` from `1.0` to `0.4`, then add a second account from
      it → the first account keeps drawing at `1.0` and the second draws at `0.4`
      (`g1-edit-then-add`).
- [x] Adding the second (`0.5`) account left the first (`1.0`) account's page
      loaded and running, untouched (`z1-side-by-side`).

## Teardown

- [x] `rm -rf "$PRESET_TMP"` for any throwaway `XDG_CONFIG_HOME` /
      `XDG_DATA_HOME` a later task exported — nothing is left under the real
      config or data directory.
