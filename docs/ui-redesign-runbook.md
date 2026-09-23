# Runbook — the desktop UI redesign (roadmap item 11's prompt, no roadmap doc)

`test-script.md`'s shape (CLAUDE.md) applies even though this item has no
`docs/tasks/` folder to hold it in: one `## Setup`, one `## Teardown`, then
one section per screen, every line a checkbox holding one concrete action and
the result it produced. A box is ticked only once that exact step has been
run — not inferred from reading the source.

Two headless passes were run. The first found two real bugs — the focus bar
and the accelerator were keyed on the wrong signal, and the arrangement
pictures had broken geometry. Both are fixed in this branch and the fix was
re-verified with fresh screenshots; both passes' evidence is folded together
below rather than kept as two separate logs.

## Setup

- [x] `cargo build -p idle-manager` → `Finished \`dev\` profile`
- [x] `cargo nextest run --workspace --no-tests=warn` → `553 tests run: 553
      passed, 1 skipped` (552 before this item's new regression test, one
      more after it — see `## Regression coverage`)
- [x] `cargo test --workspace --doc` → `0 passed; 0 failed` in every crate
      (none of the touched crates carry doc tests)
- [x] `cargo fmt --all --check` → exit 0
- [x] `cargo clippy --workspace --all-targets -- --deny warnings` → exit 0
- [x] `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`
- [x] `./scripts/arch-check.sh` → `arch-check: layer boundaries hold`
- [x] The Windows cross-check (`cargo clippy --workspace --all-targets
      --target x86_64-pc-windows-msvc -- --deny warnings`, the body of `make
      windows-check`) → exit 0
- [x] `Xvfb :97 -screen 0 1400x900x24 -fbdir <dir>` (and, for the re-verify
      pass, `:98`) for the display, screenshots decoded from the raw
      framebuffer with Pillow
- [x] A throwaway `XDG_CONFIG_HOME` holding a version 3 `sessions.toml`: three
      accounts, `Party` (`Main account` — parked, focused, kept awake; `Alt` —
      live) and `Ungrouped` (`Farm` — parked), every address pointed at
      `https://example.com`
- [x] The app launched under `dbus-run-session`, with a private,
      freshly-created `XDG_RUNTIME_DIR`, and
      `GSK_RENDERER=cairo LIBGL_ALWAYS_SOFTWARE=1
      WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1
      WEBKIT_DISABLE_DMABUF_RENDERER=1 GTK_USE_PORTAL=0 NO_AT_BRIDGE=1` — the
      app hangs before ever mapping a window without these, stuck on
      portal/secrets D-Bus negotiation
- [x] Keys and pointer injected with XTEST through `ctypes` — `libX11` for
      `XKeysymToKeycode`, `libXtst` for `XTestFakeKeyEvent`,
      `XTestFakeMotionEvent` and `XTestFakeButtonEvent`
- [x] **Not available on this box:** no window manager is installed, so a
      `GtkWindow` never regains `is-active` after a modal dialog closes and
      nothing gated on that can be exercised. None of this item's screens
      depend on window activation, so nothing here is left unrun for that
      reason — named for the record, matching item 15's runbook.
- [x] **Not available on this box:** the Windows VM
      (`scripts/windows-vm/compose.yml`) was not brought up, so every
      Windows-only item named in the final report's risk list is unrun
      rather than claimed.

## Teardown

- [x] Both headless passes quit the app, tore down Xvfb and the private D-Bus
      session, and deleted the throwaway config/data/runtime directories

## 01 — Main window

- [x] Launch with the sidebar shown, `Party` and `Ungrouped` both holding
      accounts, four slots arranged in a grid → the window opens with no
      crash
- [x] The sidebar's own `+ Add account` button sits at the top, above the
      tree, with no "Accounts" title and no "Select" toggle anywhere
- [x] The focused slot carries a visibly heavier blue outline than the
      grid's own hairlines between slots (`test1.png`, `05a-main-menu.png`)
- [x] The focused row carries the 3 px leading bar in the same blue as the
      slot's outline — including while the focused account is *parked*
      (`test1.png`; this is the fix, see `## Regression coverage`)

## 02 — Header bar

- [x] `[▤]` (sidebar toggle) sits before `[R]` (reload) at the header's
      start edge, both above the sidebar column (every screenshot below)
- [x] With an account focused, the title reads `{account} · {workspace}` —
      `Main account · Party` throughout, `Alt · Party` after switching focus
      (`05b-main-menu-after-focus-change.png`)
- [ ] With nothing focused, the title is empty and the reload button is
      greyed with tooltip `Nothing to reload` — not run: the fixture never
      leaves every account parked with none focused, and forcing that state
      was not worth a third headless pass. Backed by
      `cargo nextest run -p idle-manager-shell window::imp::tests`
      indirectly only; **left as a real gap**, not claimed
- [ ] Hovering reload with an account focused reads `Reload {account} (F5)`
      — not screenshotted; the tooltip text is a one-line `format!` in
      `update_header` and was read from source, not from a hover
- [x] The four arrangement toggles read as pictures — one bordered
      rectangle, two side by side, a 2×2 grid, `phone-symbolic` — sized
      consistently with each other (`header-toggles-zoom.png`; this is the
      fix, see `## Regression coverage`)
- [x] The main menu (`open-menu-symbolic`, header's end edge) opens with
      `Phone…` and `Keyboard Shortcuts`, and no `Enrol a phone…` /
      `Un-enrol the phone` pair (`05a-main-menu.png`)

## 03 — Sidebar rows, headings and the add button

- [x] `cargo nextest run -p idle-manager-shell session_sidebar::row::` →
      `13 tests run: 13 passed` (12 existing plus the new regression test) —
      the status-key ordering, the action label and accelerator inversion,
      the dimming rule, that a kept-awake account keys the same as one that
      is not, and that a focused parked account is `is-current` even though
      its `status` is `"parked"`, all hold
- [x] A row's `⋯` is not visible at rest; hovering the row reveals it with
      nothing else in the row moving (`01a-before-hover-zoom.png` vs
      `01c-truly-away-zoom.png`)
- [x] Giving the row (a heading, in the run) keyboard focus also reveals its
      `⋯` (`01d-focus-within-zoom.png`)
- [x] Right-click on a row opens the same menu the `⋯` button opens
      (`rightclick-focused-row.png`, `rightclick-alt-row.png`,
      `02a-rightclick.png`)
- [x] `Shift`+`F10` / `Menu` on a focused row opens the same menu
      (`06a-menu-key.png`, on a heading)
- [x] The row that holds the focused slot shows its Park/Start item with
      `Ctrl+S` (it is parked, so `Start` is offered) and `Rename…` with
      `F2`; the non-focused `Alt` row's menu shows `Park` and `Rename…` with
      no chord printed at all (`rightclick-focused-row.png` vs
      `rightclick-alt-row.png`; this is the fix, see `## Regression coverage`)
- [x] `Move to ▸` lists the row's own workspace first, insensitive, labelled
      `(here)`, then every other workspace with room, then `New workspace…`
      in its own section (`04b-move-to-expanded.png`: `Party (here)`
      greyed, `Ungrouped` enabled, `New workspace…` below a hairline)
- [x] A parked account's row shows the pause icon (`‖`-shaped), not a dot —
      `Main account` and `Farm`, both parked, in every screenshot; a live,
      not-focused account shows the filled green dot — `Alt`
      (`test1.png`). Starting/queued were not exercised — the fixture's
      accounts settle immediately, and forcing a mid-restore window was not
      worth a third headless pass; backed instead by
      `cargo nextest run -p idle-manager-shell session_sidebar::row::` and
      `session_grid::`, which cover the shape/text mapping directly
- [x] A heading's `⋯` is hover/focus-revealed exactly like a row's; the
      shown workspace's (`Party`) heading menu shows `Park all
      Shift+Ctrl+P` / `Start all Shift+Ctrl+S` (`06a-menu-key.png`) — a
      second, non-shown heading's menu was not screenshotted this run, but
      `bind_heading_menu`'s `shown` gate is unit-exercised only indirectly
      (no dedicated `imp` test for this one branch); **left as a real gap**

## 04 — Sidebar memory readout

- [x] `cargo nextest run -p idle-manager-shell memory_footer::` → `9 tests
      run: 9 passed` — unchanged by this item, still reads three lines with
      tabular figures; visible at the sidebar's foot in every screenshot

## 05 — Account menu and workspace menu

- [x] Covered under 03 — the same menu is the row's `⋯` menu

## 06 — Main menu

- [x] With `Main account` focused, the main menu's first section reads
      `Zoom — Main account · 90%` and holds `Zoom in`/`Zoom out`/`Reset
      zoom` with `Ctrl++`/`Ctrl+-`/`Ctrl+0` (`05a-main-menu.png`)
- [x] Focusing `Alt` and reopening the main menu shows the section label
      updated to `Zoom — Alt · 100%`, with the title and the grid's own
      focused-slot outline moved to match
      (`05b-main-menu-after-focus-change.png`)
- [ ] With nothing focused, the zoom section is absent from the menu
      entirely — not run, same gap as 02's "nothing focused" step
- [x] `cargo nextest run -p idle-manager-shell window::shortcut::` → `21
      tests run: 21 passed`, including the zoom-key mapping this section's
      accelerators name

## 07 — A slot in each state

- [x] `cargo nextest run -p idle-manager-shell session_grid::` → `12 tests
      run: 12 passed`, including the `Starting…` ellipsis, the spinner
      flag, and that only a focused parked slot's Start button carries the
      `Start (Ctrl+S)` tooltip
- [ ] A parked slot's Start button shows the `Ctrl+S` tooltip only when
      that slot also holds the window's focus — not screenshotted (a
      tooltip needs a hover-and-wait XTEST sequence this run did not add);
      backed by the unit test above, which exercises `placeholder_panel`
      directly
- [ ] A starting slot's panel reads `Starting…` with a spinner beside it —
      not exercised live, same reason as 03's starting/queued gap; backed
      by the unit test above
- [ ] A queued slot's panel reads `Queued` with no button — not exercised
      live; unchanged by this item beyond the sidebar's own mark, and
      already covered by pre-existing unit tests
- [x] The focused slot's outline is visibly heavier and in the theme's blue
      selection colour, not the old plain-foreground tint (every
      screenshot with a focused slot, e.g. `test1.png`)

## 08 — Add account dialog

- [ ] Stage one's title reads `Add account`; stage two's `‹ Back` button
      (click and `Alt+Left`) returns to stage one — not screenshotted this
      run (the fixture never opened this dialog); read from source
      (`add-game-dialog.ui`, `add_game_dialog/imp.rs`) and covered by
      `cargo check`/`clippy` only, not a live run. **Left as a real gap.**

## 09 — Rename

- [ ] Unchanged by this item; already matched the brief before this branch
      (title names what is renamed, field pre-selected, dim clash line,
      `Rename` insensitive on a clash) — not re-run; nothing in this diff
      touches `rename_dialog.rs`/`.ui`

## 10 — Delete account

- [ ] `Enter` in the confirm stage cancels rather than deletes, now that
      the dialog's default widget is `Cancel` — not screenshotted; read
      from `delete-account-dialog.ui`'s `default-widget` property. **Left
      as a real gap** — this is a one-line change with an obvious effect,
      but "obvious" is not "run".

## 11 — Phone

- [x] `cargo nextest run -p idle-manager-shell phone_dialog::` → `13 tests
      run: 13 passed`, including the new `Not connected.` / `Waiting for
      the phone…` / `Connected.` vocabulary and that `NotListening` still
      wins over a live code
- [ ] The dialog's buttons read `Connect phone…` and `Disconnect`; the
      explanatory sentence shows only in the plain not-connected, no-code
      state — not screenshotted; the fixture's link port was never
      attached, so `Phone…` in the main menu (visible, greyed, in
      `05a-main-menu.png`) could not be opened this run. **Left as a real
      gap.**

## 12 — Keyboard shortcuts window

- [ ] Opens with three groups — `Games`, `Accounts`, `Window` — listing the
      four new chords among the existing ones — not screenshotted; read
      from `help-overlay.ui`, which `lib.rs`'s
      `the_help_overlay_is_readable_from_the_registered_bundle` test loads
      and parses as valid resource data but does not render. **Left as a
      real gap.**

## 13 — Empty and warning states

- [ ] With no accounts at all, the sidebar still shows its `+ Add account`
      button and the grid shows the first-run message — not run this pass
      (the fixture always seeds three accounts); unchanged by this item
      beyond the button's new position, already confirmed under 01/03

## Keyboard — the four new chords

- [x] `cargo nextest run -p idle-manager-shell window::shortcut::` covers
      all four: `Ctrl+N` → `AddAccount`, `F2` → `RenameFocused`,
      `Ctrl+Page Down` → `NextPage`, `Ctrl+Page Up` → `PreviousPage`, and
      that none of the four repeats while held
- [ ] `Ctrl+N` opens the add-account dialog; `F2` opens the rename dialog;
      `Ctrl+Page Down`/`Ctrl+Page Up` turn the page — none pressed live
      this run (the fixture's one page per workspace made the pager chords
      moot, and the other two were not reached before time ran out).
      **Left as a real gap**, distinct from `Shift+F10`/`Menu` above, which
      *was* pressed live.

## Regression coverage — the two bugs the first headless pass found

1. **Focus bar and row-menu accelerator gated on the wrong signal.**
   `bind_account` checked `row.status() == "current"`, but `status_key`
   lets liveness outrank visibility — a parked, focused account reads
   `status() == "parked"`, never `"current"`. `Main account` (parked,
   focused, in the fixture) showed neither its focus bar nor `Ctrl+S`/`F2`
   in its own menu on the first pass. Fixed by adding a `Row::is-current`
   property, set independently of `status` from the same `current` bool
   `SessionSidebar::sync` already computes, and reading it instead of
   `status` in both places. Locked in by a new unit test,
   `a_focused_parked_account_is_current_even_though_its_status_is_parked`
   (`session_sidebar/row.rs`), and re-verified live: `test1.png` shows the
   bar, `rightclick-focused-row.png` shows both accelerators.
2. **Arrangement toggle pictures had broken geometry.** `.arrangement-cell`
   carried a border but no `min-width`/`min-height`, so the `1` and `2`
   pictures collapsed to a bare line; the `4` picture's cells had
   `hexpand`/`vexpand` with nothing capping the `GtkGrid`'s own size, so it
   grew well past its neighbours. Fixed with explicit per-picture cell
   sizes in `window.css`, `halign`/`valign` `center` on each picture
   container, and `column-homogeneous`/`row-homogeneous` on the grid
   instead of expand flags on its cells. Re-verified live:
   `header-toggles-zoom.png` shows all four toggles at a consistent size,
   the active one still visibly checked.

No regression test backs the CSS fix directly — CSS geometry is exactly
what design rule 11 (constraint 11 here) calls "looks", not "behaviour",
and its own vocabulary tests (`the_window_stylesheet_is_readable_from_the_registered_bundle`)
only prove the file loads, not that it lays out correctly. The screenshot
above is this fix's only evidence, and needs re-checking by eye if
`window.css` or the arrangement toggles' markup ever change again.
