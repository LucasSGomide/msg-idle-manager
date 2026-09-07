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

## Teardown

- [x] Slices 01–03 touch no disk outside `cargo`'s target directory and leave
      nothing to remove.
