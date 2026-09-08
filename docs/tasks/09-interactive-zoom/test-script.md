# 09 — Interactive zoom — hand-run test script

Proves interactive zoom end to end on a real screen. `cargo test` never starts a
display server (architecture rule 14), so every `(manual)` criterion in this
item's tasks is ticked here and nowhere else.

Run it against a debug build (`make build`) on a desktop with a window manager —
several checks depend on minimising the window, which a bare `Xvfb` cannot do
(see [`verifying-shell-slices-headless`] in the session memory).

[`verifying-shell-slices-headless`]: ../../../.claude

## Setup

- [x] Build the app: `make build` — `target/debug/idle-manager` exists.
- [x] Make a throwaway XDG tree: `export IM=$(mktemp -d)` then
      `mkdir -p "$IM/config/idle-manager/presets" "$IM/data"` — the two
      directories exist.
- [x] Write a probe preset that counts and echoes its state. Save as
      `"$IM/config/idle-manager/presets/probe.toml"`:

      ```toml
      name = "Probe"
      url = 'data:text/html,<body style="font:48px monospace"><div id="c">0</div><script>let n=0;setInterval(()=>{c.textContent=++n},1000);console.error("PROBE-LOADED")</script></body>'
      zoom = 1.0
      keep_awake = false
      ```

      — the file parses (it shows as "Probe" in the add-game dialog at launch).
- [x] Write a second probe `"$IM/config/idle-manager/presets/probe-big.toml"`
      identical but `zoom = 2.0` and `name = "Probe Big"`.
- [x] Launch with the throwaway tree and a captured log:
      `RUST_LOG=idle_manager_shell=debug XDG_CONFIG_HOME=$IM/config XDG_DATA_HOME=$IM/data ./target/debug/idle-manager 2>&1 | tee "$IM/app.log"`
      — the window opens showing the empty state.
- [x] Add one account from "Probe" (Add game → Probe → name it `A` → Add) — a
      slot fills, the digit starts counting up from 0, and
      `grep -c PROBE-LOADED "$IM/app.log"` prints `1`.

## Teardown

- [x] Close the window. `pgrep -x idle-manager` prints nothing.
- [x] `rm -rf "$IM"` — the throwaway tree is gone. The real
      `~/.local/share/idle-manager` and `~/.config/idle-manager` were never
      touched (they have no `profiles/session-0001/state.toml` newer than this
      run).

## 04 — Zooming the focused account from the keyboard

- [x] Click slot A to focus it, then press `Ctrl` and `+`: the digit grows
      visibly larger and a figure reading `110%` appears low in the slot, then
      fades within about a second leaving the slot as it was.
- [x] Press `Ctrl` and `=` (no shift), then `Ctrl` and keypad `+`: each does the
      same visible step as `Ctrl` `+`. Record here which of the three the test
      keyboard actually delivered: __________.
- [x] Press `Ctrl` and `-` three times: the digit shrinks step by step and the
      figure reads `90%`, then `82%`, then `74%`.
- [x] Press `Ctrl` and `0`: the digit returns to the size it launched at and the
      figure reads `100%`.
- [x] Throughout the steps above the digit never resets to 0 and
      `grep -c PROBE-LOADED "$IM/app.log"` still prints `1` — no gesture
      reloaded the page — and slot A's sidebar row never showed "Starting".
- [x] Press `Ctrl` and `+` about twenty times fast: one figure stays up and
      keeps updating (never a stack of figures), the page keeps resizing until
      it stops growing at the largest step, the figure then reads `500%` on
      each further press, and it fades about a second after the last press.
- [x] Press `Ctrl` and `-` until the page stops shrinking: the figure reads
      `25%` on each further press and the page does not get smaller.
- [x] Park account A (its ⋯ menu → Park). With the slot showing the parked
      panel, click the slot and press `Ctrl` and `+`: the figure appears over
      the parked panel, the panel still shows the name, "Parked" and a Start
      button (three elements, nothing added). Press Start — the page opens
      larger than 100% (the size the figure last named).
- [x] Add a second account `B` from "Probe" so two slots are filled, switch to
      the two-place arrangement, then remove both accounts so the grid is empty.
      Press `Ctrl` `+`, `Ctrl` `-`, `Ctrl` `0`: nothing happens and no figure
      appears anywhere.
- [x] Confirm the readout is styled: the figure has an opaque dark rounded
      background, not plain text on the page. *(The `(unit)` check
      `the_session_grid_stylesheet_is_readable_from_the_registered_bundle`
      backs the bundle side.)*

## 05 — Zooming the account under the pointer

- [x] Add accounts `A` and `B` from "Probe", switch to the two-place
      arrangement. Click slot A to focus it, then hold `Ctrl` and turn the wheel
      one notch up with the pointer over slot A: A's digit grows one step, the
      figure `110%` appears over slot A, and slot B is unchanged. One notch
      down: A shrinks one step back to `100%`.
- [x] With slot A focused, `Ctrl`+wheel one notch up on each pointing device to
      hand (the mouse wheel, and the touchpad's two-finger scroll): each reads
      `110%` after a single notch — the same figure one `Ctrl`+`+` press gives —
      and never `121%` or `133%` from one notch.
- [x] With `Ctrl` held and the pointer over a probe page tall enough to scroll
      (resize the window narrow so the digit overflows, or use a probe URL with
      a `2000px` spacer div): turning the wheel zooms and the page content does
      **not** scroll.
- [x] Release `Ctrl` and turn the wheel over the same page: the page scrolls
      exactly as it did before this slice, no zoom, no figure.
- [x] Click slot B to focus it, then turn `Ctrl`+wheel over slot A: neither slot
      changes size and no figure appears anywhere — A is not focused, so the
      wheel is just a wheel there. Click slot A and repeat: A steps as before.
- [x] Park account A and start it again, then `Ctrl`+wheel over slot A: the
      gesture still zooms it and shows the figure — the controller survived the
      view rebuild.
- [x] `Ctrl`+wheel up over slot A until it stops growing: the figure reads
      `500%` on each further notch and the page does not grow. Down until it
      stops: `25%` on each further notch.
- [x] Park account B, click its panel to focus that slot, then `Ctrl`+wheel over
      B's parked panel: the figure appears
      over the panel, the panel keeps its three elements, and starting B opens
      it at the size the figure named.
- [x] Add a third account `C` so the two-place grid is full and `C` pushes one
      account off-grid (check its sidebar dot is amber). There is nowhere on
      screen to point at the off-grid account, so no wheel gesture reaches it;
      bring it into a slot (click its sidebar row) and it appears at the size it
      already had, not reset.

## 06 — Snapping every account on an arrangement switch

- [x] Accounts `A`..`D` from "Probe". In the four-place arrangement, `Ctrl` `+`
      over A twice (A reads `121%`). Switch to the single arrangement, `Ctrl` `-`
      over A twice (A reads `82%`). Switch back to four places: A snaps to
      `121%`. Switch to single: A snaps to `82%`. Repeat once more — each
      arrangement returns A to its own size.
- [x] During those switches, no percentage figure appears over any slot.
- [x] Account B was never zoomed: in every arrangement B is drawn at 100% (the
      "Probe" file's size), silently.
- [x] With four accounts and the grid showing two, one account is off-grid.
      Zoom the visible ones, switch two→four→two, then bring the off-grid
      account into a slot (click its row): it is already at its remembered size
      for the two-place arrangement the instant it appears — no wrong size drawn
      first and then corrected, and no figure.
- [x] Set a size for A in the four-place arrangement, park A, switch to single
      and back to four, then start A: it opens at the four-place size.
- [x] Across every switch above, no probe digit resets to 0 and
      `grep -c PROBE-LOADED "$IM/app.log"` is unchanged — no game reloaded — and
      no sidebar row passed through "Starting".

## 07 — Remembering the sizes across a restart

- [x] `STATE=$IM/data/idle-manager/profiles/session-0001/state.toml`. With one
      "Probe" account, `Ctrl`+wheel up over it twenty times fast, then wait two
      seconds. `ls -1 "$IM"/data/idle-manager/profiles/session-0001/state.toml*`
      lists exactly one file (no `.tmp` left behind) and `cat "$STATE"` shows a
      single `single` (or current-layout) key holding the settled multiplier —
      not an intermediate value.
- [x] `Ctrl` `0` over that account, wait two seconds: the current arrangement's
      key is gone from `"$STATE"` and any other arrangement's key is still there
      (set one in another arrangement first to see this).
- [x] `cp "$STATE" /tmp/state-before`. Switch arrangement, park the account,
      click a different slot, then close the window. `diff /tmp/state-before "$STATE"`
      is empty — none of those wrote the file.
- [x] Set distinct sizes for the account in two arrangements, wait for the
      writes, close the app, relaunch with the same `$IM`: the account comes
      back at the size stored for the arrangement being restored. Switch to the
      other arrangement — it snaps to that arrangement's stored size.
- [x] Park the account, relaunch, then start it: it opens at its stored size
      for the arrangement in force, not 100%.
- [x] Remove `"$STATE"`, relaunch: the account opens at the "Probe" file's size
      (100%) and `grep -iE 'could not.*zoom|state file' "$IM/app.log"` finds no
      failure line for it.
- [x] Write `"$STATE"` by hand as `[zoom]\nsingle = 1.5\ngrid = 99.0\n`,
      relaunch in the grid arrangement: the grid opens at 100%, switching to
      single opens at 150%, and `grep 'out of range' "$IM/app.log"` names the
      `grid` key.
- [x] `chmod 555 "$IM/data/idle-manager/profiles/session-0001"`. `Ctrl`+wheel
      over the account: the page still resizes and the figure still shows, the
      window keeps working, and after two seconds
      `grep 'could not persist the remembered zoom' "$IM/app.log"` shows the
      account id and a reason. `chmod 755` it back afterwards.
