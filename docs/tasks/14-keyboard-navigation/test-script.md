# Test script — 14 Keyboard navigation and paged workspaces

## Setup

- [x] `cargo build -p idle-manager` → `Finished \`dev\` profile`
- [ ] At least four accounts across two or three workspaces exist in
      `~/.config/idle-manager/sessions.toml` (or a throwaway
      `XDG_CONFIG_HOME`/`XDG_DATA_HOME` pair), one workspace left empty, so the
      paging and next-workspace steps below have something to walk

## Teardown

- [ ] Quit the app normally; a saved `sessions.toml` written during these
      steps is the version 3 shape (`version = 3`, no `slot` key on any
      account)

## 01 — The seat model: pages derived from one order, and the version 3 file

- [x] `cargo nextest run -p idle-manager-core session::` → all `SessionBook`
      unit tests pass, including the seat/page derivation pinned by task 01
- [x] `cargo nextest run -p idle-manager-store --test the-workspace-file-on-disk` →
      `19 tests run: 19 passed, 0 skipped`, covering the version 3 round trip,
      the version 1 and version 2 migrations, and the once-only backup file
- [x] `make arch-check` → `arch-check: layer boundaries hold`

## 02 — The shortcuts window and the main menu

- [x] `cargo nextest run -p idle-manager-shell help_overlay` → `1 test run: 1
      passed` (`the_help_overlay_is_readable_from_the_registered_bundle`,
      confirming `/org/idlemanager/IdleManager/gtk/help-overlay.ui` is in the
      compiled bundle and non-empty)
- [ ] With a screen: `Ctrl`+`?` while a game page has keyboard focus opens the
      shortcuts window listing reload, zoom in, zoom out, reset zoom, next
      account, next workspace and the window itself; `Esc` closes it with the
      grid unchanged
- [ ] ☰ → `Keyboard Shortcuts` opens the same window; hovering ☰ reads `Menu`

## 03 — Stepping and paging in the domain

- [x] `cargo nextest run -p idle-manager-core -E 'test(/session::tests::(.*page.*|.*focus_next.*)/)'` →
      `8 tests run: 8 passed`, including the four-in-two walk
      (`focus_next_walks_four_accounts_in_side_by_side_turning_pages_as_it_goes`)
      and both page wraps
- [x] `cargo nextest run -p idle-manager-core -E 'test(/workspace_book::tests::(.*page.*|.*focus_next.*)/)'` →
      `5 tests run: 5 passed`, including the workspace walk skipping an empty
      one, the lone-workspace `None`, and the mobile-mode landing

## 04 — One shortcut table, and the keys on the GTK controller

- [x] `cargo nextest run -p idle-manager-shell shortcut::` → `7 tests run: 7
      passed`, pinning every `shortcut_for` mapping, `ISO_Left_Tab`/`KP_Tab`,
      and the Caps Lock/Num Lock cases
- [ ] With a screen and a real keyboard, on X11 (`make dev`): four accounts in
      `2` with a game canvas focused, `Shift`+`Tab` four times walks A, B,
      page turn, C, D, page turn, A, the sidebar bold following, and the
      page's own input shows no `Tab`
- [ ] Three workspaces, one empty: `Ctrl`+`Tab` three times lands on the two
      non-empty ones and wraps, each on the page and slot it was left on; the
      empty one is never shown, is reached by expanding its heading, and is
      reached by the key once an account is moved into it
- [ ] Holding `Ctrl`+`Tab` for two seconds switches exactly once; holding
      `Shift`+`Tab` steps exactly once
- [ ] Sidebar selection mode: both keys change nothing and the game page does
      not receive them; leaving selection mode restores them
- [ ] One account in the workspace: `Shift`+`Tab` changes nothing. One
      non-empty workspace: `Ctrl`+`Tab` changes nothing
- [ ] The first Blocker: confirm `GtkWindow`'s bubble-phase `move-focus`
      binding does not pre-empt the capture-phase controller for `Shift`+`Tab`
      while a `WebKitWebView` holds focus; if it does, fall back to a
      `gtk::ShortcutController` in capture scope `Global`

## 05 — The keys on Windows

- [x] `cargo nextest run -p idle-manager-shell virtual_key` → `3 tests run: 3
      passed`, including `tab_virtual_key_maps_to_the_keyval_the_window_matches`
- [x] `make windows-check` → cross-compiles and lints
      `x86_64-pc-windows-msvc` clean
- [ ] In the Windows VM (`scripts/windows-vm/compose.yml`) with a physical
      keyboard and a game page focused: `Shift`+`Tab` walks four accounts in
      `2` exactly as on Linux, and `Ctrl`+`Tab` switches workspace, skipping
      an empty one
- [ ] In the VM, a page with a focused text field receives no `keydown` for
      either chord, and `Tab` alone still moves the field focus
- [ ] In the VM, holding `Ctrl`+`Tab` for two seconds switches exactly once;
      the second Blocker (`GetKeyState(VK_SHIFT)` inside the accelerator
      callback) is confirmed
- [ ] In the VM, `F5` and `Ctrl`+`+` still reload and zoom the focused game
      after the idle hop

## 06 — The header-bar pager

- [x] `cargo nextest run -p idle-manager-shell window_template` → `1 test run:
      1 passed` (`the_window_template_is_readable_from_the_registered_bundle`,
      extended by task 06 to assert the `pager`/`page_previous`/
      `page_readout`/`page_next` ids are present)
- [ ] Two accounts in `2`: the pager is absent; adding a third makes it
      appear reading `1/2`, directly left of the layout toggles
- [ ] Clicking `›` on `1/2` shows the third account alone, its slot outlined
      and its sidebar row bold, readout `2/2`; clicking `›` again wraps to
      `1/2`; `‹` from `1/2` wraps to `2/2`
- [ ] Switching from `2` to `4` with three accounts hides the pager; switching
      to `1` shows it reading `n/3` with the focused account on screen
- [ ] Hovering the pager reads `Next account (Shift+Tab)`; the arrows read
      `Previous page` and `Next page`
- [ ] Ten accounts in `Ungrouped` in `1`: stepping from `9/10` to `10/10`
      does not move the layout toggles
- [ ] With the phone attached, clicking `›` changes the account the phone
      shows

## 07 — Park all and Start all

- [x] `cargo nextest run -p idle-manager-core -E 'test(/workspace_book::tests::(.*park.*|.*queue.*)/)'` →
      `8 tests run: 8 passed`, covering `park_all`'s returned prior liveness,
      `queue_parked`'s ordering, and both predicates' truth table including
      the empty workspace
- [x] `cargo nextest run -p idle-manager-shell -E 'test(/start_queue::/)'` →
      `4 tests run: 4 passed`, including the pure enqueue-decision tests for
      a draining and an idle queue
- [ ] Heading ⋯ shows `Park all` and `Start all` above `Rename…`;
      `Ungrouped`'s ⋯ shows only the two; each is greyed per the workspace's
      state; in selection mode no heading shows ⋯
- [ ] `Park all` on a workspace with one live, one queued and one starting
      account: the live one's slot shows the stopped panel at once, the
      queued one never starts, the starting one parks the moment its page
      paints, and `RUST_LOG=idle_manager_shell=debug` shows one view built
      for it
- [ ] `Start all` on three parked accounts brings them back one at a time in
      sidebar order: rows turn purple, then blue one at a time, then green
- [ ] `Start all` during a still-draining launch restore appends to the
      queue: at no point are two accounts `Starting` at once
- [ ] Hovering a workspace heading reads `Next workspace (Ctrl+Tab)`; the
      heading gains no dot, bold or other mark

## 08 — Windows verification, the design rules and the runbook

- [x] `make verify` → `fmt-check`, `lint`, `test`, `audit`, `arch-check`,
      `windows-check` and `roadmap-check` all pass
- [x] `make windows-package` → `windows-package: wrote
      dist/idle-manager-0.1.0-windows-x64.zip`
- [x] `docs/design.md` carries rules 18 (the header-bar pager) and 19 (the
      shortcuts window), each naming the widget and the task that built it
- [x] `docs/windows-vm.md`'s "What the VM can and cannot prove" lists the two
      keyboard chords and the header-bar pager
- [ ] In the Windows VM: the pager appears on the third account, turns and
      wraps, and stepping `9/10` → `10/10` does not move the layout toggles —
      or the `numeric` class is removed and the reason recorded (third
      Blocker, still open — not checked in this environment, which has no VM)
- [ ] In the VM: a drop onto the empty trailing slot of a part-empty last
      page moves the account there and the sidebar order follows; `Park all`
      with one starting account parks it on paint; `Ctrl`+`?` opens the
      shortcuts window
- [ ] In the VM: launching over a version 2 `sessions.toml` shows the page
      holding the previously focused account with that account focused, and
      `sessions.v2.toml` is written once and unchanged by a second save
- [ ] Every unticked step above (all X11-manual and all Windows-VM steps) is
      run and ticked before this item ships — none of them were run in this
      environment, which has no display server driving privileges were used
      for and no Windows VM
