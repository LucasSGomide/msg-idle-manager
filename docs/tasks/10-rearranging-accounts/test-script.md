# Test script — 10, renaming and rearranging accounts

Hand-run runbook for this item, per CLAUDE.md's planning workflow. Steps are
recorded after the fact from this item's live testing, each reflecting
something actually run and observed, not a prescription for a future run.

## Setup

- [x] Build a debug binary (`cargo build --workspace`) and launch it headless
      under Xvfb — `Xvfb :98 -screen 0 1280x800x24 -noreset`, then
      `dbus-run-session -- env DISPLAY=:98 GDK_BACKEND=x11 GSK_RENDERER=cairo
      XDG_CONFIG_HOME=<tmp>/config XDG_DATA_HOME=<tmp>/data
      ./target/debug/idle-manager` — the window opens showing the empty
      state ("No games yet — add one to get started."), confirmed by an
      `ffmpeg -f x11grab` screenshot of `:98.0`.
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
      `XDG_DATA_HOME`).

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
