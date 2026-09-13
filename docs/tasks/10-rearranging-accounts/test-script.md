# Test script — 10, renaming and rearranging accounts

Hand-run runbook for this item, per CLAUDE.md's planning workflow. Steps are
recorded after the fact from this item's live testing (2026-09-13); each one
reflects something actually run and observed, not a prescription for a future
run.

## Setup

- [x] Build the workspace: `cargo build --workspace` — compiles clean,
      producing `target/debug/idle-manager`.
- [x] Launch the app headless: `Xvfb :98 -screen 0 1280x800x24 -noreset &`,
      then inside `dbus-run-session -- env DISPLAY=:98 GDK_BACKEND=x11
      GSK_RENDERER=cairo XDG_CONFIG_HOME=<tmp>/config
      XDG_DATA_HOME=<tmp>/data ./target/debug/idle-manager` — a screenshot
      (`ffmpeg -f x11grab -video_size 1280x800 -i :98.0 -frames:v 1 out.png`)
      shows the window with "No games yet — add one to get started."
- [x] Drive the running window with synthetic X11 input (XTest via `ctypes`
      on `libX11.so.6`/`libXtst.so.6`) — clicks, pointer moves and typed text
      all land correctly, confirmed by screenshots after each step.
- [x] Add an account through "Add game" → "Something else…" with a `data:`
      URL address whose page carries a full-page button and a floated
      top-right `<input>` — e.g.
      `data:text/html,<input style="float:right;width:200px"><button
      style="display:block;width:1000px;height:600px;font-size:40px"
      onclick="document.body.style.background='lime';this.textContent='HIT'">CLICK</button>` —
      the account appears live in the sidebar and its page renders the
      button. (`position:fixed`/`vw`/`vh` styling renders blank in this
      headless WebKitGTK setup — plain flow layout with `float` was used
      instead.)

## Teardown

- [x] Kill the launched `idle-manager` process and its `dbus-run-session`
      wrapper by PID, then the `Xvfb` process — a fresh `Xvfb`/launch cycle
      afterward starts cleanly with an empty account list (fresh
      `XDG_DATA_HOME`) and no stale lock.

## 01 — One rule for an account name, and renaming in the book

- [x] Run `cargo test -p idle-manager-core` — 111 tests pass, including the 13
      new ones for `account_name` and `SessionBook::rename`
      (`account_name_trims_surrounding_whitespace`,
      `account_name_is_none_for_an_empty_or_whitespace_only_string`,
      `renaming_with_a_padded_name_stores_the_trimmed_name_and_returns_true`,
      `renaming_changes_only_the_display_name`,
      `renaming_with_an_empty_or_whitespace_only_name_returns_false`,
      `renaming_with_an_empty_name_leaves_the_book_unchanged`,
      `renaming_an_unknown_id_returns_false`,
      `renaming_an_unknown_id_leaves_the_book_unchanged`,
      `renaming_a_parked_account_keeps_it_parked`,
      `renaming_a_queued_account_keeps_it_queued`,
      `renaming_to_a_name_another_account_already_has_is_accepted`,
      `the_workspace_after_a_rename_carries_the_new_name`).
- [x] Run `cargo build --workspace` after the add-game dialog change
      (`crates/idle-manager-shell/src/add_game_dialog/imp.rs` now calls
      `idle_manager_core::account_name` instead of repeating the trim rule) —
      the whole workspace, including `idle-manager-shell`, compiles clean.
- [x] With the app running per Setup, click "Add game", choose "Something
      else…", type three spaces into "Name for this account" and a valid
      address into "Address" — the "Add" button stays insensitive (greyed
      out) with the address alone filled in.
- [x] Clear the name field and type "  Padded Name  " (two leading and two
      trailing spaces) — the "Add" button turns sensitive (solid blue).
- [x] Click "Add" — `XDG_CONFIG_HOME/idle-manager/sessions.toml` records
      `name = "Padded Name"` with no leading or trailing whitespace,
      confirming the add-game dialog's stored name is trimmed by the same
      rule `account_name` states.

## 04 — The grip over each place

- [x] Add two accounts ("Solo", "Twin"), each with the click-target page from
      Setup, and switch to the side-by-side (2) layout — both places show
      their pages side by side, "Solo" focused (bold in the sidebar).
- [x] Move the pointer into "Twin"'s place, over its page — a drag-handle
      icon appears in its top-right corner, inset by a small margin.
- [x] Move the pointer out of "Twin"'s place — the grip disappears.
- [x] Switch to the four-place (4) layout and repeat the hover in/out over an
      occupied place — the grip shows and hides the same way, confirming it
      is not layout-specific to `SideBySide`.
- [x] Park an account ("Charlie") from its sidebar ⋯ menu, view its place —
      the plain placeholder panel shows (name, "Parked", "Start" button).
      Hover that place — the grip appears over the panel and the panel's
      three elements are unchanged (same text, same button, same layout).
- [x] Switch to the one-place (1) layout and move the pointer anywhere over
      the single visible place, including its top-right corner — no grip
      appears anywhere.
- [x] Quit and relaunch with a fresh, empty `XDG_DATA_HOME` — the empty state
      shows with no places at all, so there is nowhere for a grip to appear.
- [x] Hover "Solo"'s place to reveal its grip, then click at a point offset
      from the grip icon but still near the corner (on the page, not the
      icon) — the click reaches the button: it turns green
      (`background:lime`) and its text changes to "HIT".
- [x] With "Solo" focused, hover the unfocused "Twin"'s place and click
      exactly on its grip (press and release, no movement) — the sidebar's
      bold/current marker stays on "Solo"; "Twin"'s page is unaffected (no
      colour change) and no drag chip appears.
- [x] Click on "Twin"'s page (not its grip) — the sidebar's bold/current
      marker moves to "Twin", and its button reacts (turns green, "HIT"),
      confirming a page click both focuses the place and reaches the game.
- [x] Press down on "Solo"'s grip and move the pointer roughly 150px past the
      starting point (well past the drag threshold) — a small dark rounded
      chip reading "Solo" appears and follows the pointer, and "Solo"'s grip
      is no longer visible while dragging.
- [x] Release the drag over "Twin"'s floated `<input>` — the chip disappears,
      the input stays empty (no pasted text), the sidebar order and the
      focused account are unchanged from before the drag.
- [x] `cargo test -p idle-manager-shell` —
      `session_grid::tests::a_dragged_account_round_trips_through_a_value_as_a_boxed_type_not_a_string`
      passes: a `DraggedAccount` built from a `SessionId`, stored in a
      `glib::Value` via `to_value()`, reads back through `Value::get` with
      the same `SessionId`, and the value's `type_()` is not `glib::Type::STRING`.

## 03 — Moving an account between places in the book

- [x] Run `cargo test -p idle-manager-core` — 134 tests pass, including the 23
      new ones for `move_into_slot` (`layout.rs`) and `SessionBook::move_to_slot`
      (`session.rs`): `moving_onto_an_occupied_slot_swaps_exactly_those_two_and_names_the_other_account`,
      `moving_onto_an_empty_slot_fills_it_and_leaves_the_source_empty`,
      `moving_onto_the_movers_own_slot_is_unchanged`,
      `moving_onto_a_slot_the_layout_does_not_have_is_unchanged`,
      `moving_an_off_grid_account_is_unchanged`,
      `moving_an_unknown_id_is_unchanged`,
      `moving_onto_an_occupied_slot_returns_swapped_and_trades_exactly_those_two_slots`,
      `moving_onto_an_empty_slot_returns_filled_and_leaves_the_old_slot_empty`,
      `moving_onto_the_movers_own_slot_returns_unchanged_and_leaves_the_book_equal`,
      `moving_onto_a_slot_the_layout_does_not_have_returns_unchanged_and_leaves_the_book_equal`,
      `moving_an_off_grid_account_returns_unchanged_and_leaves_the_book_equal`,
      `moving_an_unknown_id_returns_unchanged_and_leaves_the_book_equal`,
      `a_swap_where_focus_was_on_the_movers_slot_moves_focus_to_the_target`,
      `a_fill_where_focus_was_on_the_movers_slot_moves_focus_to_the_target`,
      `a_swap_where_focus_was_on_the_target_moves_focus_to_the_movers_old_slot`,
      `a_swap_where_focus_was_on_neither_slot_leaves_focus_unchanged`,
      `a_swap_survives_a_layout_switch_and_back`,
      `a_real_move_reorders_in_slot_accounts_by_slot_index_with_no_off_grid_accounts_present`,
      `a_real_move_puts_in_slot_accounts_first_then_off_grid_accounts_in_their_old_relative_order`,
      `moving_a_parked_account_keeps_it_parked`,
      `a_move_changes_no_accounts_keep_awake_flag`,
      `a_move_changes_no_accounts_remembered_zoom`,
      `restoring_from_the_workspace_of_a_moved_book_reproduces_the_same_slots_and_order`.
- [x] Run `cargo build --workspace` — the whole workspace, including
      `idle-manager-shell` and the `idle-manager` binary, compiles clean against
      the new `MoveOutcome` and `SessionBook::move_to_slot` in
      `idle-manager-core`, confirming this core-only change breaks nothing
      downstream.

## 02 — Renaming an account from the sidebar

- [x] `cargo test -p idle-manager-shell` — 35 tests pass, including
      `tests::the_rename_dialog_template_is_readable_from_the_registered_bundle`
      (looks up `/org/idlemanager/IdleManager/ui/rename-dialog.ui` from the
      registered bundle the same way the window and slot-placeholder templates
      are checked).
- [x] With the app running per Setup, add "Alpha" (`data:text/plain,Alpha
      Page`) then "Bravo" (`data:text/plain,Bravo Page`) — both live. Open
      Alpha's ⋯ menu (background, not focused) — `Rename…` is last, below
      "Keep running when hidden", sensitive.
- [x] Park Alpha from its ⋯ menu, reopen its ⋯ menu — items read "Start",
      "Keep running when hidden", "Rename…"; `Rename…` sensitive while "Start"
      is not greyed (a parked account's Start is enabled; see the starting
      case below for the greyed case).
- [x] Quit, edit `sessions.toml` by hand to point Alpha's `url` at
      `http://127.0.0.1:<port>/` where `<port>` is a local Python
      `socket.accept()`-and-hold server that never writes a response, then
      relaunch with both accounts `liveness = "running"` — Alpha's page never
      finishes loading, so it sits in `Starting` (blue dot) indefinitely, and
      Bravo sits `Queued` (purple dot) behind it in the start queue,
      confirmed with a screenshot immediately after launch. Open Alpha's
      (starting) ⋯ menu — "Start" is greyed, "Rename…" is not. Open Bravo's
      (queued) ⋯ menu — same: "Start" greyed, "Rename…" sensitive. This
      covers all four liveness states (live, parked, starting, queued) each
      listing `Rename…` last and sensitive.
- [x] Choose `Rename…` on the live, focused "Bravo" — a modal window titled
      "Rename account" (no header-bar chrome renders under the headless,
      window-manager-less Xvfb, matching the add-game dialog's own
      appearance in this harness) opens over the main window with one entry
      reading "Bravo", the whole word selected (screenshot shows it
      highlighted).
- [x] Backspace the selection to empty — `Rename` greys out, no error text or
      red field appears. Type three spaces — `Rename` stays greyed. Type a
      letter ("X", giving "X   ") — `Rename` turns sensitive (solid blue)
      again.
- [x] Press Escape — the window closes; the sidebar row still reads "Bravo"
      and `sessions.toml` on disk is unchanged (`name = "Bravo"`, confirmed
      by `cat`).
- [x] Reopen Bravo's `Rename…`, type "Renamed Bravo", press Enter — the
      window closes at once, the sidebar row reads "Renamed …" (ellipsized)
      immediately, `sessions.toml` is rewritten to `name = "Renamed Bravo"`
      within the debounce window, the memory footer still reads "1 running"
      throughout with no reload spike, and the page's own content ("Bravo
      Page") is unchanged, confirming no reload.
- [x] Switch to the side-by-side (2) layout, where Alpha's place still shows
      its loading cover/placeholder reading "Alpha" / "Starting" / a greyed
      "Start" (its page never paints, per the hung server above). Open
      Alpha's ⋯ menu, choose `Rename…`, type "Zulu", press Enter — the
      window closes, the sidebar row and the place's placeholder panel both
      switch to "Zulu" immediately, the panel still reads "Starting" with
      "Start" greyed (account untouched, not restarted or reloaded).
- [x] Park "Renamed Bravo" from its ⋯ menu — its place's placeholder panel
      shows "Renamed Bravo" / "Parked" / an enabled "Start". Choose
      `Rename…`, press Enter without changing the pre-filled text (same
      name) — the window closes and nothing visible changes: panel still
      reads "Renamed Bravo" / "Parked", "0 running" unchanged.
- [x] Rename the same parked account to "Parked Yankee" — the sidebar row
      and its place's parked panel both switch to "Parked Yankee" at once;
      the account stays `Parked` ("Start" still enabled, "0 running"
      unchanged).
- [x] Quit the app (window close button) and relaunch against the same
      `XDG_CONFIG_HOME`/`XDG_DATA_HOME` — the sidebar shows "Zulu" and
      "Parked Yankee" (the renamed names survived the relaunch), and
      `find <data-home>/idle-manager/profiles -maxdepth 1` still lists
      `session-0001` and `session-0002` — the on-disk profile folder names,
      derived from each account's hidden id, are unaffected by either
      rename.
