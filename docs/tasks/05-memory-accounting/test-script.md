# Test script — 05, memory accounting and performance

Hand-run runbook for this item, per CLAUDE.md's planning workflow. Steps are
recorded after the fact from this item's live testing (2026-09-11/12); each
one reflects something actually run and observed, not a prescription for a
future run.

## Setup

- [x] Build a debug binary and launch it with `make dev` — the application
      window opens and the sidebar shows the account list.
- [x] Start at least one account so a `WebKitWebProces` descendant exists —
      confirmed via `./scripts/memory-report.sh`, which lists it nested two
      `bwrap` levels under `idle-manager`.
- [x] Start four accounts for the multi-account checks below — confirmed live
      (2026-09-11/12), e.g. one `memory-report` run showed
      `own 96178 KiB`, `descendants 2974258 KiB`, `total 3070436 KiB`,
      `processes 26` across four `WebKitWebProces` entries.

## Teardown

- [x] Quit the application (`pkill -f target/debug/idle-manager` or the
      window's own quit) — a second `make dev` launches cleanly afterward
      (`gtk::Application` is single-instance).

## 03 — The memory report script and its soak mode

- [x] Run `./scripts/memory-report.sh` with the application running — prints
      one line per process (identifier, command, PSS/KiB, RSS/KiB) and a
      total; observed repeatedly against a real 4-account session
      (2026-09-11/12).
- [x] Confirm the rendering process appears nested under two `bwrap`
      processes rather than as a direct child — each `WebKitWebProces` entry
      in the same output sits two `bwrap` levels under `idle-manager`.
- [x] Compare the total proportional (PSS) figure against the total resident
      (RSS) figure in the same run — PSS is materially lower (e.g. one
      process at 313,490 KiB PSS vs 431,776 KiB RSS).
- [x] Park every account but one, rerun the script, and compare against the
      run with all accounts live — the process count and total both drop;
      confirmed live (2026-09-12).
- [x] Run the script with no application started — it reports that and exits
      without printing a total.

## 04 — The sidebar footer

- [x] Launch the application and check the memory footer before the first
      sample — all four values (own, running, aggregate, total) read as
      dashes, never zeroes; confirmed live (2026-09-12).
- [x] Wait for the first sample — the footer shows the application's own
      figure, the running count, the aggregate and the total, refreshing with
      no interaction; confirmed live (2026-09-12).
- [x] Unpark a parked account — all three figures and the running count rise
      back; confirmed live (2026-09-12).
- [x] Collapse and expand the sidebar — the footer hides with the list and
      reappears still refreshing; confirmed live (2026-09-12), same session
      that found and fixed the collapsed-sidebar width bug.
- [x] Hover the footer block — the tooltip states the figures are
      proportional and that per-account figures are not available; confirmed
      live (2026-09-12).

## 05 — The soak, and whether it is a leak

- [x] Read `docs/memory-budget.md` — it states, in one sentence, that a live
      account's memory does not settle and grows without bound.
- [x] Read the recorded growth rate in the same doc — ≈690-1030 MB/hour
      across two live soaks, with the finding stating the remaining slices
      cap the leak rather than fix it (the leak was in fact found and fixed
      directly, per "Round 3" in the same doc).

## 06 — Telling the engine it has a limit

- [x] Start an account and check `RUST_LOG=idle_manager_shell=debug` output at
      startup — one line names the memory-pressure limit and both
      thresholds, confirming `configure_web_engine` applied them once to the
      networking process.
- [x] Confirm in `crates/idle-manager-shell/src/lib.rs` that
      `WEB_PROCESS_MEMORY_LIMIT_MIB`, `CONSERVATIVE_PRESSURE_THRESHOLD` and
      `STRICT_PRESSURE_THRESHOLD` each carry a comment naming the
      `docs/memory-budget.md` figure they were chosen against, and that
      `KILL_PRESSURE_THRESHOLD` is `0.0`, held there explicitly as a product
      decision.
- [x] Confirm every view is built through `shared_web_context()`
      (`crates/idle-manager-shell/src/web_view.rs`) rather than a default
      context, with a warning logged if the shared context is absent.

## 07 — Diagnostics off by default, WebGL per game

- [x] Build a release binary with `IDLE_MANAGER_DIAGNOSTICS` unset — right
      click offers no Inspect Element and no per-resource lines appear in
      the log; confirmed live (2026-09-12).
- [x] Same release binary with the switch set, press F12 on a loaded page —
      the inspector opens and per-resource lines return; confirmed live
      (2026-09-12), after this item's F12 keybinding fix
      (`wire_inspector_key` in `web_view.rs`).
- [x] Build a debug binary (`make dev`) with the switch unset — both the
      inspector and per-resource logging are already on; confirmed live
      (2026-09-12).
- [x] Start one account whose preset disables WebGL and one whose preset
      leaves it enabled — both load and play their game; confirmed live
      (2026-09-12).

## 08 — The budget, measured against the browser, and the warning

- [x] Read `crates/idle-manager-shell/src/memory_footer.rs` — `MEMORY_BUDGET_MIB`
      is `None`, and its own test (`nothing_is_over_budget_while_no_budget_has_been_measured`)
      confirms the footer never shows the warning tint while it is unset.
- [x] Read `docs/design.md` — it carries the over-budget footer rule and
      states how it differs from rule 9's message strip (no latch, no
      dismissal, clears itself).
