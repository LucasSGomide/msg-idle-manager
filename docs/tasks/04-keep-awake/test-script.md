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
- [x] With keep-awake **on** for an account running the `## Setup` timer page, minimise the window for ~20s and read the timer's gap, then restore — every minimised line reads `gap=1004ms`, against the `gap=2000ms` the same page reported minimised with the flag off minutes earlier in the same session. Hidden-page timer throttling is genuinely off for that account. Run on a real desktop; the Xvfb display has no window manager and cannot minimise.

## 04 — The frame-callback shim

- [x] Read `crates/idle-manager-shell/resources/js/keep-awake.js` — it opens with the constraint that forced it (the engine hands a hidden page zero frame callbacks, with no setting to prevent it), and both `HIDDEN_FRAME_INTERVAL_MS = 250` and `FIRST_SHIM_FRAME_ID` are named constants. The 250ms interval is a guess, not a measurement: tried against the `vischeck` frame counter and against Kittens Game, which played normally with the shim installed.
- [x] Add an account at a page whose own script logs whether `window.requestAnimationFrame` is the replacement, then turn "Keep running when hidden" on from its `⋮` menu — the log shows `keep_awake=true`, exactly one reload, then `SHIM-CHECK requestAnimationFrame-is-shim=true`: the replacement was in place before the page's own script ran. Before the toggle, and after toggling back off, the same page logs `is-shim=false`.
- [x] Add an account whose page shadows `document.hidden`, requests one frame it cancels immediately and one it leaves pending, then flips the shadowed value back after 150ms — the log reports `CANCEL-CHECK cancelled-fired=false pending-fire-count=1`: a cancelled request never fires, and a pending one fires exactly once when the page becomes visible.
- [x] With the shim installed on one account, toggle it back off and watch the log — `keep_awake=false`, one reload, `is-shim=false`, and both that account's console output and an unrelated account's ongoing `VIS …` lines keep arriving. `remove_all_scripts` cleared the page-console bridge and the rebuild put it back.
- [x] Add `https://kittensgame.com/web/` with keep-awake off — it loads and renders its Bonfire panel. Turn keep-awake on, and after the single reload it renders identically; clicking "Gather catnip" three times raises the catnip counter to 3. A real idle game plays normally with the patched frame callback.
- [x] Screenshot every row through the sequence — each still shows only its name, state word, coloured dot, `Park` button and `⋮` menu, with the menu's check matching the last toggle. No new marker or layout appears (design rules 1–3 unchanged).
- [x] With keep-awake **on** for an account running the `## Setup` frame-counter page, minimise for ~20s on a real desktop and read the counter, then restore — the toggle logs both features set and exactly one reload, the injected script count goes from `user-script:1:` to `user-script:2:`, and every minimised line reads `fps=4` with `frames` climbing unbroken by four a second (307, 311, 315 … 347). A second minimise held it for 24 seconds, 521 → 617. Four a second is the shim's 250ms interval answering. This also proves the hand-over of the request outstanding at the moment of hiding: without it the counter would have frozen at the value it held when the window went down, exactly as it does with the flag off.
- [x] With keep-awake **off** for the same account, minimise for ~20s and read the same counter, then restore — `gap` stretches to `2000ms` and `frames` freezes at 1281 for the full 14 seconds minimised, resuming the instant the window is restored.

## 05 — The row's keep-awake indication

- [x] `cargo test -p idle-manager-shell session_sidebar::row::tests::the_keep_awake_indication_shows_only_when_the_flag_is_on` — `test result: ok. 1 passed`
- [x] `cargo test -p idle-manager-shell session_sidebar::row::tests::status_key_is_the_same_whether_or_not_the_account_is_kept_awake` — `test result: ok. 1 passed`
- [x] `make verify` — all stages pass; nextest reports `58 tests run: 58 passed, 0 skipped`, the two above being new.
- [x] Start the app per `## Setup`, add an account "Main" and read its row — name, `Current`, green dot, `Park`, `⋮`, and no keep-awake mark anywhere on the row.
- [x] Open the row's `⋮` menu and choose "Keep running when hidden" — the popover closes, the row runs one `Starting`/blue-dot reload cycle, then settles back to `Current`/`Park` with a small grey diamond after the dot.
- [x] Choose the item again to turn it off — one more reload cycle, and the diamond is gone on the next redraw.
- [x] With keep-awake on for "Main", add a second account "Farm" so "Farm" becomes `Current` and "Main" drops to `Background` — "Main" shows the amber glowing dot plus the diamond; "Farm" shows no diamond.
- [x] Click `Park` on "Main" — the row reads `Parked` with the unlit grey dot and a `Start` button, and the diamond is still shown beside the dimmed name.
- [x] Add an account at the hanging address `http://10.255.255.1/` and turn its keep-awake on — the row holds at `Starting` with the blue glowing dot and an insensitive `Start`, with the diamond shown throughout.
- [x] Switch the layout to "2" and turn keep-awake on for the account sitting `Visible` (in-slot, not focused) — its plain green dot and the diamond read distinctly from the focused row's glowing dot.
- [ ] With four rows on screen across different states, one of them keep-awake on and one named with 70 characters, read the sidebar at its `240px` width — the long name ellipsises, every trailing element stays un-clipped, and a short name like "Main" renders in full rather than collapsing to an ellipsis. **Not re-run at the final width:** the width was raised from 220 to 240 to fix exactly that collapse, and the run that would have confirmed it was cut short. Confirm by eye on the next real run.

## Teardown

- [x] `kill <dbus-run-session PID> <idle-manager PID>` then `kill <Xvfb PID>` — all three are gone from `ps -eo pid,cmd`. Never `pkill -f target/debug/idle-manager`: that pattern matches the driving shell's own argv and kills it.
