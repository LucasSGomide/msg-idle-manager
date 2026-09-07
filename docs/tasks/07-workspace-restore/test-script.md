# 07 — Restoring the workspace on launch · test script

Hand-run runbook that proves the item works end to end, beside the tasks'
`(unit)` / `(integration)` / `(manual)` criteria rather than replacing them.
Every line is one concrete action and the observable result it must produce.
Tick a box only after the step has actually been run.

`## Setup` and `## Teardown` are shared and grow as tasks need them; each task
appends its own `## NN — Title` section and never rewrites another's.

## Setup

- [x] `export PATH="$HOME/.cargo/bin:$PATH"` and `cd` to the repo root, so
      `cargo` and `make` resolve.
- [x] `cargo build --workspace` completes without error — the whole workspace
      compiles with the workspace-restore code in place.

## 01 — The workspace in the domain, and coming back from one

- [x] `cargo test -p idle-manager-core` → `test result: ok. 65 passed; 0
      failed`, including the ten `session::tests` that pin every acceptance
      criterion: restore order and settings, running → `Queued`, parked →
      `Parked`, the start order and a parked account's absence from it, a saved
      slot the layout cannot show going off-grid then reclaimed, a fresh
      identifier after a restore, the `workspace()` round trip, a `Starting`
      account written as running, and an empty workspace.
- [x] `cargo clippy -p idle-manager-core --all-targets -- -D warnings` →
      `Finished` with no warning, so `Workspace`, `Account`, `SavedLiveness` and
      the `WorkspaceStore` port meet the code standards.

## 02 — The workspace file on disk

- [x] `cargo test -p idle-manager-store` → `test result: ok` for every binary,
      including `the-workspace-file-on-disk` (9 integration cases: the field-for
      -field round trip, the `insta` snapshot with `version = 1` as line one,
      the missing case distinct from `Ok(None)` at the port, an invalid file
      reported malformed and moved to `sessions.bad` with its bytes still
      readable, an unknown `version` kept aside its own way, an unrepresentable
      liveness and an unknown key each a parse failure, no `.tmp` left after two
      writes, and off-grid vs in-slot visibility surviving the round trip) and
      the `paths` unit test placing `sessions.toml` beside `presets/` under XDG
      config.
- [x] `cargo clippy -p idle-manager-store --all-targets -- -D warnings` →
      `Finished` with no warning.

## 03 — The queued row and the queued slot

- [x] `cargo test -p idle-manager-shell` → `test result: ok. 18 passed` for the
      lib, including: `status_key` → `"queued"` for a queued account whatever its
      slot or focus; `status_label("queued")` → `"Queued"`; `action_label` →
      `"Start"` and `action_sensitive` → `false` for a queued account;
      `name_markup` for a queued account equals the parked one (dimmed under the
      one existing not-running rule); `placeholder_panel(Queued)` → the `"Queued"`
      line with `button_visible = false` while `placeholder_panel(Parked)` is
      unchanged (`"Parked"`, button visible and sensitive); and the compiled
      bundle carries `.status-queued { #dc8add }`, distinct from the starting
      `#62a0ea` and parked `#9a9996`.
- [x] `make verify` exits `0` — fmt, clippy, the 117-test suite, audit,
      arch-check and roadmap-check all pass with the queued vocabulary in place.

## Teardown

- [x] Slices 01–03 touch no disk outside `cargo`'s target directory (the
      integration tests clean their own throwaway directories on drop) and
      leave nothing to remove.
