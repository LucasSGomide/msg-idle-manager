# 02 — The session sidebar: hand-run test script

Proves the shell end to end, the coverage architecture rule 14 keeps out of
`cargo test`. Every box is one concrete action and the result it must produce.

The automated half (`(unit)` criteria) is not repeated here; run it with
`make verify`.

## Setup

- [x] `make verify` exits 0 — format, clippy, tests, audit, layer boundaries and
      the roadmap check all pass.
- [x] `cargo build --workspace` produces `target/debug/idle-manager`.
- [x] A display server is available. The verification runs below used a headless
      X server (`Xvfb :99 -screen 0 1280x800x24`) with
      `GDK_BACKEND=x11 GSK_RENDERER=cairo` and synthetic input via `XTest`; a
      normal desktop needs none of that.
- [x] `export XDG_DATA_HOME="$(mktemp -d)"` so the run's profiles land in a
      throwaway directory. Every account below is added with the address
      `about:blank`, which paints at once — the sidebar and grid behaviour under
      test does not depend on a real page.

## Teardown

- [x] Close the window (title-bar close, or `WM_DELETE_WINDOW`). The process
      ends. `rm -rf "$XDG_DATA_HOME"` removes the profile directories the run
      created.

## 01 — Bringing a session into the focused slot

- [x] `cargo nextest run -p idle-manager-core` — 22 tests pass. The six that pin
      this slice's `## Acceptance criteria`:
      `bringing_an_off_grid_session_into_an_occupied_focused_slot_swaps_the_two`,
      `a_swap_leaves_every_other_sessions_visibility_untouched`,
      `bringing_an_off_grid_session_into_an_empty_focused_slot_moves_no_other_session`,
      `activating_a_visible_session_moves_focus_to_its_slot`,
      `activating_a_visible_session_changes_no_visibility`,
      `the_placement_names_every_session_and_an_outcome_for_each_case`; plus the
      two `SessionBook::focus_session` wrapper tests
      `focusing_a_visible_session_moves_the_books_focus_to_its_slot` and
      `focusing_an_off_grid_session_swaps_it_into_the_focused_slot_in_the_book`.

## 02 — The sidebar and its fold

- [x] Launch the app. The header bar shows, left of "Add game", a toggle button
      with the `sidebar-show-symbolic` icon, pressed (active) at startup, and the
      fixed-width sidebar column is drawn down the leading edge.
- [x] With no accounts, the sidebar area holds one centred line
      "No accounts yet — add one to get started." and no rows. (The window's own
      "No games yet …" empty state shows to its right, driven by the same
      emptiness.)
- [x] Press "Add game" and add five accounts in turn (`Aaa`, `Bbb`, `Ccc`,
      `Ddd`, `Eee`). The sidebar shows five rows in that add order, each with the
      account name on the leading edge.
- [x] In the 1-slot arrangement: `Eee` (the account in the slot) shows `Current`
      with a glowing green dot on its row's trailing edge and its name is drawn
      bold. `Aaa`, `Bbb`, `Ccc`, `Ddd` each show `Background` with a glowing
      amber dot and the name in the dimmed (≈55% alpha) style. (design rule 1)
- [x] Press "4". `Aaa` (focused slot) shows `Current` + glowing green dot, bold;
      `Bbb` / `Ccc` / `Ddd` show `Visible` + a plain green dot (no glow); `Eee`
      shows `Background` + glowing amber dot, dimmed.
- [x] Press the header toggle. The sidebar column is not drawn at all and the
      2×2 grid widens to the full window width; the focus rectangle stays on
      slot 0, every web view keeps its slot, no page reloads. Press the toggle
      again: the sidebar returns with the same five rows, the same trailing
      markers and the same bold current row.
- [x] The `footer` `GtkBox` (declared `id="footer"`, `visible=false`, no
      children, after the scrolled list in `session-sidebar.ui`) is bound: the
      sidebar's redraw calls `set_visible(false)` on that template child on every
      sync and the app runs with no `Gtk-CRITICAL` and no panic, which a missing
      child id would raise on the first `TemplateChild::get()`. `GTK_DEBUG=interactive`
      shows the box under the list.

## 03 — Clicking a row to swap or focus

- [x] Five accounts, "4" arrangement, slot 0 focused (`Aaa` current). Click the
      `Eee` row (`Background`). `Eee` swaps into slot 0: its row switches to
      `Current` + glowing green dot and its name goes bold; the `Aaa` row
      switches to `Background` + glowing amber dot in the dimmed style.
      `Bbb` / `Ccc` / `Ddd` keep `Visible` and their grid slots.
- [x] In that swap the two moved views keep their pages — the grid re-places
      through the same `size_allocate`-only path a layout toggle uses (item 01's
      re-place path, unchanged here), so neither view is re-parented and neither
      `about:blank` slot shows a reload or placeholder.
- [x] Every account other than the two involved keeps its row marker and grid
      position — `Bbb` / `Ccc` / `Ddd` above are unchanged by the swap.
- [x] Click the `Ccc` row (in slot 2, `Visible`). The focus rectangle moves to
      slot 2; `Ccc` switches to `Current` + glowing green dot and bold, `Eee`
      drops back to `Visible`; no view moves and no other row changes.
- [x] Clicking an out-of-sight account with an empty focused slot: not reachable
      from the UI — an account is out of sight only when every slot is filled, so
      the focused slot is never empty while a row exists to click. The transition
      (`Outcome::Filled`, nothing displaced) is pinned by the core unit test
      `bringing_an_off_grid_session_into_an_empty_focused_slot_moves_no_other_session`.
