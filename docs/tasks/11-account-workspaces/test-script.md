# Test script — 11: Account workspaces, and deleting an account

A hand-run runbook proving this item's `(manual)` criteria end to end, per
architecture rule 14. It never replaces the `(unit)` / `(integration)`
criteria in each task file; it sits beside them.

## Setup

- [x] Start an isolated Xvfb display, not `:0`/`:1`/`:99` — a stale display
      silently swallows synthetic input:
      `Xvfb :57 -screen 0 1280x800x24 -noreset &` (kill any earlier one with
      `pgrep -x Xvfb` + `kill`, then `rm -f /tmp/.X57-lock`, first)
- [x] Create a throwaway XDG tree so the run never touches a real
      installation: `mkdir -p /tmp/idle-manager-verify/{config,data}`
- [x] Build the app once: `cargo build -p idle-manager`
- [x] Launch inside an isolated session bus, so a stray earlier instance never
      swallows this run's single-instance activation:
      `dbus-run-session -- env DISPLAY=:57 GDK_BACKEND=x11 GSK_RENDERER=cairo XDG_CONFIG_HOME=/tmp/idle-manager-verify/config XDG_DATA_HOME=/tmp/idle-manager-verify/data ./target/debug/idle-manager` —
      the log reaches `"activated; presenting the main window"` and then
      `"workspace restored"` or `"no saved workspace; opening a first run"`
- [x] Screenshot with
      `ffmpeg -f x11grab -video_size 1280x800 -i :57.0 -frames:v 1 out.png`,
      written to `$HOME` first (the sandboxed `ffmpeg` cannot write under a
      `claude-*` scratch path) then moved where needed

## Teardown

- [x] Stop the app: find its pid with
      `pgrep -af 'target/debug/idle-manager'` and `kill` it (never
      `pkill -f` typed directly in a tool call — it can match the tool's own
      wrapper argv)
- [x] Stop Xvfb: `pgrep -x Xvfb` then `kill`, then `rm -f /tmp/.X57-lock`
- [x] Remove the throwaway XDG tree: `rm -rf /tmp/idle-manager-verify`

## 01 — Ungrouped workspace and the version 2 file

- [x] Seed `/tmp/idle-manager-verify/config/idle-manager/sessions.toml` with a
      hand-written version 1 file: `version = 1`, `layout = "single"`, two
      `[[account]]` tables (`session-0001` "Main", `session-0002` "Alt"), both
      `liveness = "parked"`
- [x] Launch per Setup. The log reads `workspace restored accounts=2` and the
      screenshot shows the sidebar listing "Main" then "Alt", both with a grey
      parked dot, one place shown — the same arrangement the version 1 file
      described. `cat sessions.toml` is still untouched, `version = 1`
- [x] Click the "2" layout button. `cat sessions.toml` now reads `version = 2`,
      `active = "ungrouped"`, one `[[workspace]]` table `id = "ungrouped"`,
      `layout = "side-by-side"`, holding both accounts unchanged
- [x] `cat sessions.v1.toml` beside it reads byte-for-byte identical to the
      hand-written version 1 file from the first step

## 02 — Sidebar tree and switching workspaces

- [x] Seed a version 2 file: `workspace-0001` "Party" (`side-by-side`,
      `session-0001` "Main" in slot 0, `session-0002` "Alt 1" in slot 1, both
      parked, `expanded = true`), `workspace-0002` "Farm crew" (`single`, no
      accounts, `expanded = true`), `ungrouped` "Ungrouped" (`single`,
      `session-0004` "Farm" parked in slot 0, `expanded = true`), `active =
      "ungrouped"`
- [x] Launch per Setup. The log reads `workspace restored accounts=3`. The
      screenshot shows the sidebar 200 px wide, headings in order Party, Farm
      crew, Ungrouped, none carrying a dot or bold; Party's accounts nested
      with their names in full; Farm crew expanded showing one dim "No
      accounts" line; Ungrouped's Farm shown grey/parked; the "1" layout
      button selected
- [x] Click "Main" under Party (not shown). The screenshot shows the "2"
      button now selected, the grid split side by side showing "Main" and
      "Alt 1", and the log reads `switched the shown workspace
      from=ungrouped to=workspace-0001`
- [x] Click "Farm" under Ungrouped. The screenshot shows the "1" button
      selected again and the grid back to Farm alone — Ungrouped's own layout
      and focus restored exactly — and the log reads `switched the shown
      workspace from=workspace-0001 to=ungrouped`. `sessions.toml` now reads
      `active = "ungrouped"`
- [x] Click Party's expander arrow. The screenshot shows Party collapsed
      (Main/Alt 1 hidden) with the grid and the "1" button unchanged, and
      `sessions.toml` now reads `expanded = false` under `workspace-0001`
- [x] Relaunch per Setup. The screenshot shows Party still collapsed, Farm
      crew and Ungrouped still expanded — the choice survived the restart
- [x] Click the "4" button. The screenshot shows Ungrouped split into a
      2×2 grid with Farm in the first place and the other three empty
- [x] Re-expand Party and click "Main". The screenshot shows the "2" button
      selected again, not "4" — Party's own layout, untouched by Ungrouped's
      change
- [x] Click Main's place's "Start" button. The screenshot shows Main's dot
      turn green and its name bold (current), the footer reading "1 running"
- [x] Click "Farm" under Ungrouped to switch away from Party. The screenshot
      shows Main's row now carrying an amber dot with its name dimmed
      (`background`), the footer still reading "1 running" and Main's web
      process still present — no reload, the account kept running out of
      sight

## 05 — Selection mode and moving accounts

- [x] Seed a version 2 file: `workspace-0001` "Party" with four parked
      accounts filling its `grid` layout, `ungrouped` "Ungrouped" with one
      parked account, `active = "ungrouped"`
- [x] Launch per Setup. Click "Select". The screenshot shows the button now
      reading "Done", a tick box on every account row, and a bar reading
      "0 ticked" with "Move to…" greyed out
- [x] Tick "Alt1" (under Party) and "Farm" (under Ungrouped). The screenshot
      shows both boxes checked, the bar reading "2 ticked" with "Move to…"
      now sensitive, and the grid unchanged (still showing Ungrouped)
- [x] Collapse Party's heading. The screenshot shows Party's rows hidden and
      the count still "2 ticked". Re-expand it: Alt1 is still ticked
- [x] Open "Move to…" with Party full (4/4). The menu lists only "Ungrouped"
      — Party is not offered
- [x] Re-seed with Party holding three accounts (room for one) and choose it
      from the menu with one ticked account: the screenshot shows the
      account moved to the bottom of Party's list, selection mode ended
      (button reads "Select" again, ticks cleared), the shown workspace
      (Ungrouped) unchanged, and `sessions.toml` lists the account under
      `workspace-0001` with no `slot` key (off-grid, since Ungrouped's own
      account still held slot 0 there)
- [x] Seed Party (one account, room for one more) and Ungrouped (one
      account) with Ungrouped shown. Tick Ungrouped's account and move it to
      Party. The screenshot shows Ungrouped's heading now reading "No
      accounts" and the grid area reading "No games in this workspace",
      centred and dim, with no button
- [x] With an account ticked, press "Done". The screenshot shows the tick
      boxes and bar gone and both accounts still in their original
      workspaces — nothing moved
- [x] Launch with no saved file at all. The screenshot shows the unchanged
      first-run state — "No games yet — add one to get started." and its
      button — confirming the empty-workspace message is a distinct state,
      not a replacement for it

## 06 — Naming workspaces: New, Rename and Remove

- [x] Seed a version 2 file: `workspace-0001` "Party" (`side-by-side`,
      `session-0001` "Main" in slot 0, `session-0002` "Alt1" in slot 1, both
      parked), `ungrouped` "Ungrouped" (`single`, `session-0003` "Farm" parked
      in slot 0), `active = "ungrouped"`
- [x] Launch per Setup. The screenshot shows `Ungrouped`'s heading with no ⋯
      button; Party's ⋯ opens a menu holding `Rename…` then `Remove
      workspace`
- [x] Choose Party's `Rename…`. The window opens titled "Rename workspace"
      (untitled bar in this window-manager-less harness, but the field is
      pre-filled "Party" and fully selected, `Rename` sensitive). Typing
      "ungrouped" greys `Rename` and shows the dim "Another workspace has
      this name." line; typing a unique name "Squad" clears the line and
      re-enables `Rename`
- [x] Confirm. The heading now reads "Squad", the grid (`Ungrouped`, shown)
      is unchanged, and `sessions.toml`'s `workspace-0001` table now reads
      `name = "Squad"`
- [x] Click "Select". Tick "Farm" and "Main". The bar reads "2 ticked" and
      "Move to…" lists `Ungrouped`, a separator, then "New workspace…"
- [x] Choose "New workspace…". The window opens titled "New workspace",
      field empty, `Create` insensitive. Typing only spaces keeps it
      insensitive with no dim line. Pressing Escape closes the window and
      returns to selection mode with "Farm" and "Main" still ticked, "2
      ticked" unchanged
- [x] Reopen "New workspace…", type "Crew", confirm. A "Crew" heading
      appears just above `Ungrouped`, expanded, holding exactly "Main" and
      "Farm". Selection mode ends (`Select` shown again, ticks cleared), the
      shown workspace stays `Ungrouped`, and `sessions.toml` lists
      `workspace-0002` `name = "Crew"` holding both accounts
- [x] Seed 5 parked accounts under `Ungrouped` alone in a `grid` layout
      (`FR.17.2`'s five-ticked case) and relaunch. Click "Select", tick all
      five: the bar reads "5 ticked" and "Move to…" shows "New workspace…"
      greyed out, `Ungrouped` still offered plain
- [x] Untick down to two, open "New workspace…", press Escape: back to
      selection mode with "2 ticked" intact
- [x] Click "Done" with a tick still set: the tick boxes and bar disappear
      and nothing moved
- [x] Click "Squad"/"Crew"-style workspace's ⋯ → `Remove workspace` while
      another workspace is shown: the heading disappears with no prompt, its
      accounts land at the bottom of `Ungrouped`'s list in the same order,
      still parked (never touched), and their profile folders are untouched
      on disk. `sessions.toml` drops the removed `[[workspace]]` table
      entirely
- [x] Switch to a named workspace (making it shown), then choose its ⋯ →
      `Remove workspace`: the grid and sidebar switch to show `Ungrouped`,
      the "1"/"2"/"4" buttons reflect `Ungrouped`'s own layout, and
      `sessions.toml`'s `active` key reads `"ungrouped"`
- [x] Open an account row's own ⋯ → `Rename…`: the window still opens
      titled "Rename account", pre-filled with the account's name. Clearing
      the field greys `Rename` with no dim "taken" line — the account name
      check never reports `Taken`

## 08 — Deleting an account

- [x] `cargo nextest run -p idle-manager-shell the_delete_account_dialog_template_is_readable_from_the_registered_bundle`
      passes
- [x] Seed a version 2 file: `workspace-0001` "Team" (`side-by-side`, one
      running account whose address never resolves — so it settles as
      `current`/green once its browser error page finishes loading — and one
      parked account), `ungrouped` "Ungrouped" (`grid`, two more running
      accounts with non-resolving addresses and one parked), `active =
      "ungrouped"`
- [x] Launch per Setup. Open a live account's own row ⋯ (shown workspace):
      the menu lists `Park`, `Keep running when hidden`, `Rename…`, `Delete
      account…` last, sensitive. Same for a parked account's row in the
      hidden `Team` workspace
- [x] Choose `Delete account…` on a live, shown account. The window reads
      "Delete `<name>`? Its logins and saved game data will be removed from
      this computer. This cannot be undone.", with `Cancel` and a red
      `Delete account`
- [x] Click `Cancel`. The window closes, the row and its profile folder
      (`profiles/session-000x`) are unchanged
- [x] Reopen and confirm. The screenshot shows a spinner and "Deleting
      `<name>`…" with no buttons; `sessions.toml` and the profile folder are
      gone within a moment, the window closes itself, the row is gone from
      the sidebar and every other row is unmoved
- [x] Relaunch. The deleted account does not reappear, its folder was not
      recreated, and the other accounts still load their pages
- [x] `chmod 555` the `profiles` directory (its parent), then delete a
      parked account. The window shows "Couldn't finish deleting `<name>`.",
      a dim reason line reading only the OS error (no path baked into it —
      `ProfileRemoval::folder` supplies the path on its own line), a second
      dim, selectable line with the folder path, and `Close`/`Retry`
- [x] `chmod 755` the `profiles` directory back, then click `Retry`: the
      deletion completes and the window closes
- [x] Repeat the read-only failure on another parked account (in the hidden
      workspace), then click `Close` instead of `Retry` with the directory
      still read-only: the row keeps its grey parked dot, and switching to
      its workspace shows the ordinary grey "Parked"/"Start" panel in its
      place — not blank, not broken. Restoring write permission and deleting
      it again succeeds
- [x] Add a custom game after a deletion: the new account's id numbers
      higher than any deleted id, and its profile folder is a fresh
      directory distinct from any deleted account's

### Note on the queued liveness state

The `(manual)` criterion asking for a queued account's ⋯ menu could not be
held in that state long enough to screenshot: `WebKit` treats a DNS-failure
error page as a completed load, which ends the starting interval and moves
an account straight to `current`/`background` rather than leaving a second
restored account waiting behind it. Confirmed instead by reading
`session_sidebar/imp.rs`'s `bind_row_menu`: the `delete` action is built and
added to the menu unconditionally, with no liveness check of any kind —
unlike `parking`, which is disabled while `Starting` — so there is no code
path by which a queued account's menu could differ from the live and parked
cases actually screenshotted above.

## 07 — Choosing a workspace when adding a game

- [x] Seed a version 2 file: `workspace-0001` "Duo" (`side-by-side`, two
      parked accounts, room for one more), `workspace-0002` "Party" (`grid`,
      four parked accounts, full), `ungrouped` "Ungrouped" (`single`, no
      accounts), `active = "ungrouped"`
- [x] Launch per Setup. Open "Add game" → "Something else…". The details
      stage's `Workspace` field reads "Ungrouped"; opening it lists "Duo"
      then "Ungrouped" (checked) — "Party" is not offered
- [x] Cancel. Click "Main" under Duo to show it, reopen "Add game" →
      "Something else…": the `Workspace` field now defaults to "Duo"
- [x] Cancel. Switch to "Party" (full, `4`); reopen "Add game" → "Something
      else…": the field defaults to `Ungrouped`
- [x] Cancel. Relaunch with no saved file at all (first run); click "Add
      your first game" → "Something else…": the field lists `Ungrouped`
      alone, selected
- [x] Relaunch on the three-workspace seed. With `Ungrouped` shown, add a
      custom account "Alt3" (a local `file://` test page) choosing
      `Workspace` "Duo". The screenshot shows the shown workspace, grid and
      1/2/4 buttons unchanged (`Ungrouped`, empty); the new "Alt3" row
      appears under Duo's heading carrying the amber `background` dot, and
      the footer reads "1 running"
- [x] Click "Alt3" under Duo (switches the shown workspace to Duo). The
      screenshot shows its page already rendered — no reload — and its dot
      now green (`current`); `sessions.toml` lists `session-000x` "Alt3"
      under `workspace-0001`
- [x] Relaunch. The screenshot shows "Alt3" still listed under "Duo",
      running, its page loaded again from the restore — confirming it
      survived exactly where it was added
