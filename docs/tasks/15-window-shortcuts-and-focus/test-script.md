# Test script — 15 Window shortcuts, and focus that hands over the keyboard

Every keyboard step below is run with a **game page holding the keyboard**,
not the sidebar — that is the whole point of `FR.27.4`, and after this item it
is also the normal state rather than one you reach by clicking.

## Setup

- [x] `cargo build -p idle-manager` → `Finished \`dev\` profile`
- [x] A throwaway `XDG_CONFIG_HOME`/`XDG_DATA_HOME` pair holding a version 3
      `sessions.toml` with four accounts over two workspaces — `Ungrouped`
      holding `Alpha`, `Bravo`, `Charlie` in `side-by-side` (so it pages: `1/2`),
      and `Party` holding `Delta` — every account pointed at a local probe page
- [x] The probe page: a tiny local HTTP server serving one page per account
      that posts every `keydown` it receives back to the server, and its own
      `window.innerWidth` on every resize. This is what makes two claims
      measurable rather than assumed — *which* account's page holds the
      keyboard (type a letter, see which page reports it), and whether a chord
      ever reached a page at all
- [x] `Xvfb :97 -screen 0 1400x900x24 -fbdir <dir>` for the display, with
      screenshots read straight out of the `-fbdir` framebuffer through Pillow
      (no ImageMagick and no `xwd` on this box)
- [x] The app launched under `dbus-run-session`, because `GApplication`'s
      single-instance handoff otherwise hands the launch to the release build
      already running on this machine and exits
- [x] Keys and pointer injected with XTEST through `ctypes` — `libX11` for
      `XKeysymToKeycode`, `libXtst` for `XTestFakeKeyEvent`,
      `XTestFakeMotionEvent` and `XTestFakeButtonEvent`. No `xdotool` needed
- [ ] **Not available on this box:** no window manager is installed
      (`openbox`, `matchbox`, `i3`, `xfwm4`, `metacity`, `mutter`, `fluxbox`,
      `icewm`, `twm`, `marco`, `jwm` all absent). Without one, a `GtkWindow`
      never regains `is-active` after a modal dialog closes, so the one step
      that depends on re-activation is left unrun below and named in the
      item's Blockers
- [ ] **Not available on this box:** the Windows VM
      (`scripts/windows-vm/compose.yml`) was not brought up, so every Windows
      step below is unrun rather than claimed

## Teardown

- [ ] Quit the app normally; the throwaway config pair is deleted with its
      profile directories, and the probe server and `Xvfb :97` are stopped

## 01 — The sidebar and arrangement chords

- [x] `cargo nextest run -p idle-manager-shell shortcut::` → `13 tests run: 13
      passed`, pinning `Ctrl`+`B` in both letter cases, `Ctrl` with `1`/`2`/`4`
      and their keypad twins, `Ctrl`+`3` and `Ctrl`+`KP_3` mapping to nothing,
      the unmodified keys mapping to nothing, the lock-mask case, and
      `repeats_while_held` being true for `Reload` and `Zoom` alone
- [x] `make verify` → exit 0; `542 tests run: 542 passed, 1 skipped`, plus
      `fmt-check`, clippy, `cargo deny`, `arch-check: layer boundaries hold`,
      `make windows-check` and `roadmap tables are up to date`
- [x] Type a letter before any chord → only `Alpha`'s page reports it, so a
      game page holds the keyboard for every step that follows
- [x] `Ctrl`+`B` → every page's `innerWidth` goes 535 → 635 px: the sidebar
      folded and the grid took its width
- [x] The pages report no `b` for that press — only the bare `Control`
      modifier, which the window never claimed to stop
- [x] `Ctrl`+`B` again → 635 → 535 px, the sidebar back
- [x] `Ctrl`+`B` held down for two seconds → the sidebar folds once and stays
      folded (635 px), not flickering open and shut
- [x] `Ctrl`+`1` → the saved workspace's `layout` reads `single`; the page
      receives no `1`
- [x] `Ctrl`+`4` → `layout` reads `grid`; the page receives no `4`
- [x] `Ctrl`+`2` → `layout` reads `side-by-side`; the page receives no `2`
- [x] `Ctrl`+`2` again, with two games already shown → `layout` unchanged and
      nothing written
- [x] `Ctrl`+`4` pressed from the phone arrangement leaves it first and lands
      on `grid`, exactly as clicking the `4` toggle from `Phone` does
- [x] Hovering `▤` reads `Show or hide the account list (Ctrl+B)`
      (`shot-tooltip-sidebar.png`)
- [x] Hovering the `2` toggle reads `Two games (Ctrl+2)`
      (`shot-tooltip-two.png`)
- [x] `Ctrl`+`?` opens the shortcuts window with a second group, `Window`,
      listing `Ctrl+B` Show or hide the sidebar, `Ctrl+1` One game, `Ctrl+2`
      Two games, `Ctrl+4` Four games — and no phone key anywhere
      (`shot-shortcuts.png`)

## 02 — The parking chords

- [x] `cargo nextest run -p idle-manager-shell shortcut::` covers the four new
      arms: `Ctrl`+`p`/`P` → `ParkFocused`, `Ctrl`+`s`/`S` → `StartFocused`,
      the same two letters under `Ctrl`+`Shift` → `ParkWorkspace` /
      `StartWorkspace`, and that lock bits never promote `Ctrl`+`P` to the
      workspace chord
- [x] `make verify` → exit 0 (the same run recorded under 01)
- [x] `Ctrl`+`P` with `Alpha` focused and live → `Alpha` reads `parked`,
      `Bravo` and `Charlie` still `running`; the page receives no `p`
- [x] A second `Ctrl`+`P` → `Alpha` still `parked`, nothing started
- [x] `Ctrl`+`S` → `Alpha` back to `running`
- [x] A second `Ctrl`+`S` on the now-live account → nothing changes
- [x] `Ctrl`+`Shift`+`P` → all three `Ungrouped` accounts read `parked`, and
      `Party`'s `Delta` is untouched at `running`
- [x] `Ctrl`+`Shift`+`P` again, with nothing left running → nothing changes
- [x] `Ctrl`+`Shift`+`S` → all three back to `running`
- [x] `Ctrl`+`Shift`+`P` held for two seconds → the workspace parks exactly
      once
- [x] A live account's ⋯ menu reads `Park` with `Ctrl+P` at its trailing edge
      (`shot-row-menu-live.png`)
- [x] The `Ungrouped` heading's ⋯ menu reads `Park all  Shift+Ctrl+P` and
      `Start all  Shift+Ctrl+S`, the second greyed because nothing is parked
      (`shot-heading-menu.png`)
- [x] `Ctrl`+`?` lists all eight of this item's chords under `Window`,
      including the four parking ones (`shot-shortcuts.png`)
- [x] `Ctrl`+`S` pressed twice during a `Ctrl`+`Shift`+`S` queue drain, on an
      account that is queued and then starting, changes nothing and starts no
      second view: all three still come back one at a time and the run ends
      with every account `running`, no panic and no error in the log
- [ ] With a game page focused, `Ctrl`+`S` opens no save dialog and `Ctrl`+`P`
      no print dialog — not run: the probe page is not a real game, and a page
      that binds either key is what this step is about. Named in the item's
      Blockers

## 03 — The new keys on Windows

- [x] `cargo nextest run -p idle-manager-shell virtual_key` → `6 tests run: 6
      passed`, including `0x42`/`0x50`/`0x53`/`0x31`/`0x32`/`0x34` mapping to
      `b`, `p`, `s`, `_1`, `_2`, `_4`, and `0x33` (`VK_3`) mapping to nothing
- [x] `the_mapped_keyvals_decide_the_same_shortcuts_the_linux_keyvals_do`
      passes: every Windows virtual key, mapped and run through
      `shortcut_for`, decides exactly the shortcut the Linux keyval decides —
      including the two chords where Win32 reports the unshifted letter and
      GDK the shifted one
- [x] `make verify` passes, `make windows-check` included
- [ ] Every Windows-machine step is unrun — see 05

## 04 — Focus hands over the keyboard

- [x] At launch, with no click anywhere, typing a letter reaches `Alpha`'s
      page — the focused account's page has the keyboard (`FR.27.1`)
- [x] Clicking the `Bravo` row in the sidebar focuses it and typing then
      reaches `Bravo`'s page, with no click inside the page
- [x] `Shift`+`Tab` → focused position 1; typing now reaches `Bravo`'s page
      and no other
- [x] `Shift`+`Tab` again → focused position 2, the page turns, and typing
      reaches `Charlie`'s page
- [x] `Ctrl`+`Tab` → the shown workspace is `Party` and typing reaches
      `Delta`'s page
- [x] `Ctrl`+`Tab` back → `Ungrouped` on the page and account it was left on
      (focused 2), and typing reaches `Charlie`'s page
- [x] `Ctrl`+`P` on the focused account → it reads `parked`, and typing now
      reaches **no** page at all: the keyboard is left where it was rather
      than handed to another slot's view (`FR.27.2`)
- [x] `Ctrl`+`S` on that same still-focused account → it is live again and
      typing reaches its page, so the grab happens at the new view's first
      paint
- [x] `Ctrl`+`4` → the arrangement changes and typing still reaches the
      focused account's page
- [x] In `grid` with three accounts, clicking the empty fourth slot leaves the
      focused position at 0 and leaves `Alpha`'s page holding the keyboard —
      an empty slot reaches for nothing (`FR.27.2`)
- [x] With the rename dialog open from a row's ⋯ menu, typing `quill` reaches
      no page at all (`shot-rename-typed.png`, `FR.27.3`)
- [x] With the add-game form open on `A game the application does not know`,
      typing `zephyr` lands in `Name for this account` and **no** page
      receives any of it (`shot-form-typed.png`, `FR.27.3`)
- [x] With the sidebar in selection mode, no chord moves the keyboard to any
      view and no page receives one (recorded under 05's selection-mode run)
- [x] Clicking the sidebar row of the account that is **already** focused
      takes the keyboard to that row — the focused account has not changed —
      and twelve seconds of redraws (the memory footer ticks on its own) never
      pull it back to the page: the hand-over follows a focus *change*, not
      every redraw
- [x] Every chord in 01 and 02 was pressed with a game page holding the
      keyboard and was acted on, so the capture-phase controller keeps firing
      against a grab-focused view — the item's first Blocker, answered on this
      engine (`FR.27.4`)
- [ ] **Known boundary, by design rather than by failure:** focus is handed
      over when the *focused account changes*, so clicking a header-bar
      control — a layout toggle, the `Phone` toggle — leaves the keyboard on
      that control until the focused account next changes. Measured: after
      clicking `Phone` and pressing `Ctrl`+`4`, no page held the keyboard
      until a focus change. `FR.27.1` promises only the focus-change case and
      this matches it; the chords mean a keyboard user never clicks those
      controls at all. Left unticked because it is worth a decision rather
      than a silent assumption
- [ ] Closing a dialog gives the keyboard back to the focused page without a
      click — **not run**: this box has no window manager, so the window never
      regains `is-active` after the dialog's toplevel goes, and the guard that
      protects the dialog's own field (`FR.27.3`) is what blocks the re-grab.
      Measured: with the dialog closed, a click into the page restores typing
      while a focus change does not, and the debug line
      `the focused account's view was not handed the keyboard … active=false`
      names the guard. `follow_window_activation` is the handler written for
      this and it cannot be exercised here. Named in the item's Blockers

## 05 — The design rules, the Windows check and the runbook

- [x] `make verify` → exit 0
- [x] `make windows-package` → `wrote dist/idle-manager-0.1.0-windows-x64.zip
      (39M)`
- [x] `docs/design.md` rule 2 carries the narrowing sentence, rule 19 names
      accelerator text as the menu-item equivalent of a tooltip, and rule 20
      states the one-key-per-direction case with its reasoning
- [x] Sidebar selection mode: with `Select` pressed, `Ctrl`+`4` changes no
      arrangement, `Ctrl`+`Shift`+`P` and `Ctrl`+`P` park nothing, `Ctrl`+`B`
      does not fold the sidebar, and no game page receives any of the four
      (`FR.26.6`, `shot-selection-mode.png`)
- [ ] In the Windows VM with a physical keyboard and a game page holding
      focus, each of the eight chords does what it does on Linux and the page
      receives no `keydown` — **not run**: the VM was not brought up
- [ ] In the VM, `Ctrl`+`S` on a page with a focused text field opens no save
      dialog and types no `s` into the field — **not run**
- [ ] In the VM, holding `Ctrl`+`Shift`+`P` for two seconds parks the
      workspace exactly once — **not run**
- [ ] In the VM, focusing an account hands its page the keyboard with no click
      inside it — **not run**
