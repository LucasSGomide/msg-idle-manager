# Test script — 04 Keep-awake for hidden games

## Setup

- [x] `Xvfb :99 -screen 0 1280x800x24 &` then `dbus-run-session -- env DISPLAY=:99 GDK_BACKEND=x11 GSK_RENDERER=cairo XDG_DATA_HOME=<throwaway dir> RUST_LOG=idle_manager_shell=debug cargo run -p idle-manager` — the app activates and presents its window. `dbus-run-session` is required: the single-instance `org.idlemanager.IdleManager` name means a stray earlier `target/debug/idle-manager` swallows the activation and the process exits silently.

## 01 — Keep-awake in the session book

- [x] `cargo test -p idle-manager-core session::tests::an_account_added_to_the_book_starts_with_keep_awake_off` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::turning_keep_awake_on_for_an_account_that_had_it_off_returns_true` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::turning_keep_awake_on_for_an_account_that_had_it_off_leaves_the_flag_on` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::setting_keep_awake_to_its_current_value_reports_no_change` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::setting_keep_awake_to_its_current_value_leaves_liveness_unchanged` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::turning_keep_awake_on_for_a_live_account_leaves_it_starting` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::turning_keep_awake_on_for_a_parked_account_leaves_it_parked` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::keep_awake_survives_a_layout_change_that_moves_the_account_between_slots` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::keep_awake_survives_being_pushed_off_grid_and_brought_back_into_focus` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::keep_awake_survives_being_parked` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::keep_awake_survives_being_unparked` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::setting_keep_awake_on_an_unknown_id_reports_no_change` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-core session::tests::setting_keep_awake_on_an_unknown_id_leaves_the_book_untouched` — `test result: ok. 1 passed`
- [x] `make verify` — all six stages pass; nextest reports `56 tests run: 56 passed, 0 skipped`

## 02 — What the engine does to a hidden page

- [x] Start the app per `## Setup` and read the start-up log — 486 `engine feature` lines are logged in one walk, before any account is added, followed by one `keep-awake feature lookup complete timer_throttling_found=true css_animation_suspension_found=true`.
- [x] Grep that log for the two switches — `HiddenPageDOMTimerThrottling` and `HiddenPageCSSAnimationSuspension` are both present, `category="Other"`, `is_default_value=true`, on WebKitGTK 2.52.6 / `webkit6` 0.6.1 (2026-09-06). They became `FEATURE_ID_HIDDEN_PAGE_TIMER_THROTTLING` and `FEATURE_ID_HIDDEN_PAGE_CSS_ANIMATION_SUSPENSION` in `web_view.rs`.
- [x] Point both constants at a nonexistent identifier and restart — the log shows `WARN keep-awake feature not found in this engine build identifier="PENDING_DISCOVERY"` twice, the application still activates, and two accounts added afterwards (`Add game` → name/address → `Add`) still load their pages normally.
- [x] Add an account whose page logs `console.warn('visibility-check', document.visibilityState, 'hidden=' + document.hidden)` every second, then add a second account so the first is pushed off-grid (its sidebar row reads `Background`) — the page-console bridge logs `page console warning text="visibility-check visible hidden=false"` repeatedly while off-grid. **An out-of-sight account is not marked hidden by the engine**: the off-grid layout trick from item 01 works, and keep-awake buys nothing in this case.
- [x] Minimise the window on a real desktop with a window manager (the Xvfb display used above has none, so the iconify request goes unanswered there) and read the same page for ~20s, then restore — the page reports `hidden=true` throughout, the one-second timer's gap stretches from `1004ms` to a steady `2000ms`, and the frame counter freezes completely at 1246 for 22 seconds (`fps=0`) before returning to `fps=60` on restore. **A minimised window is marked hidden**: a timer-driven game runs at half speed and a frame-driven game stops dead, which is what both engine switches and the shim exist to prevent.
- [x] Screenshot the sidebar and grid before and after the change — two accounts, one `Current` and one `Background`, each row showing its name, state word, coloured dot and `Park` button, and the grid rendering the current account's page. Layout and controls are identical to pre-slice, and both accounts' pages load.

## 03 — The row menu and the engine's switches

- [x] Start the app per `## Setup`, `Add game` → name/address → `Add` — the new row shows a name, state word, `Park` button and a `⋮` three-dot menu button after it, on every row added.
- [x] Click the row's `⋮` button — a popover opens with one checkable item, "Keep running when hidden", unchecked.
- [x] Click that item — the popover closes; the log shows one `DEBUG keep-awake features set session=session-0001 keep_awake=true features=["HiddenPageDOMTimerThrottling", "HiddenPageCSSAnimationSuspension"]`, and exactly one `load changed event=Started` → `Finished` cycle for that account follows.
- [x] Reopen the row's menu — the item now shows a checkmark. Click it again — the popover closes, the log shows `keep_awake=false` with the same two identifiers, and one more reload cycle for that account.
- [x] Reopen the menu a third time — the item shows unchecked again, matching the last toggle. The check always matches the stored flag, so no click can request the value already held.
- [x] Add a second account whose address hangs on connect (`http://10.255.255.1/`), open its menu and choose the item — the row reads `Starting` with a blue dot and its action button reads `Start`, greyed out, for as long as the reload is in flight, then returns to `Current`/`Park`.
- [x] While the second account's toggle fires (`session=session-0002`), the first account's row stays `Background`/`Park` and only ever logs against `session=session-0001` on its own toggles — one account's toggle never touches another's.
- [x] With the first account's keep-awake left on, click `Park` on its row, then `Start` — the log shows `keep-awake features set session=session-0001 keep_awake=true ...` firing again from the fresh view `SessionView::start` builds, and reopening the menu shows the item still checked.
- [x] Switch the layout to "2" and back to "1", moving the first account's slot and focus — the menu still reads checked afterwards, and the layout change triggers no reload of its own.
- [ ] With keep-awake **on** for an account running the `## Setup` timer page, minimise the window for ~20s and read the timer's gap, then restore — expect `~1000ms`, against the `2000ms` recorded above with the flag off. **Not run here:** the Xvfb display has no window manager, so minimising is impossible; run this on a real desktop.

## Teardown

- [x] `kill <dbus-run-session PID> <idle-manager PID>` then `kill <Xvfb PID>` — all three are gone from `ps -eo pid,cmd`. Never `pkill -f target/debug/idle-manager`: that pattern matches the driving shell's own argv and kills it.
