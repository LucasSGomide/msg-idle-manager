# 09 — Interactive zoom — hand-run test script

Proves interactive zoom end to end on a real screen. `cargo test` never starts a
display server (architecture rule 14), so every `(manual)` criterion in this
item's tasks is ticked here and nowhere else.

Run it against a debug build (`make build`) on a desktop with a window manager —
several checks depend on minimising the window, which a bare `Xvfb` cannot do
(see [`verifying-shell-slices-headless`] in the session memory).

[`verifying-shell-slices-headless`]: ../../../.claude

## Setup

- [ ] Build the app: `make build` — `target/debug/idle-manager` exists.
- [ ] Make a throwaway XDG tree: `export IM=$(mktemp -d)` then
      `mkdir -p "$IM/config/idle-manager/presets" "$IM/data"` — the two
      directories exist.
- [ ] Write a probe preset that counts and echoes its state. Save as
      `"$IM/config/idle-manager/presets/probe.toml"`:

      ```toml
      name = "Probe"
      url = 'data:text/html,<body style="font:48px monospace"><div id="c">0</div><script>let n=0;setInterval(()=>{c.textContent=++n},1000);console.error("PROBE-LOADED")</script></body>'
      zoom = 1.0
      keep_awake = false
      ```

      — the file parses (it shows as "Probe" in the add-game dialog at launch).
- [ ] Write a second probe `"$IM/config/idle-manager/presets/probe-big.toml"`
      identical but `zoom = 2.0` and `name = "Probe Big"`.
- [ ] Launch with the throwaway tree and a captured log:
      `RUST_LOG=idle_manager_shell=debug XDG_CONFIG_HOME=$IM/config XDG_DATA_HOME=$IM/data ./target/debug/idle-manager 2>&1 | tee "$IM/app.log"`
      — the window opens showing the empty state.
- [ ] Add one account from "Probe" (Add game → Probe → name it `A` → Add) — a
      slot fills, the digit starts counting up from 0, and
      `grep -c PROBE-LOADED "$IM/app.log"` prints `1`.

## Teardown

- [ ] Close the window. `pgrep -x idle-manager` prints nothing.
- [ ] `rm -rf "$IM"` — the throwaway tree is gone. The real
      `~/.local/share/idle-manager` and `~/.config/idle-manager` were never
      touched (they have no `profiles/session-0001/state.toml` newer than this
      run).

## 04 — Zooming the focused account from the keyboard

- [ ] Click slot A to focus it, then press `Ctrl` and `+`: the digit grows
      visibly larger and a figure reading `110%` appears low in the slot, then
      fades within about a second leaving the slot as it was.
- [ ] Press `Ctrl` and `=` (no shift), then `Ctrl` and keypad `+`: each does the
      same visible step as `Ctrl` `+`. Record here which of the three the test
      keyboard actually delivered: __________.
- [ ] Press `Ctrl` and `-` three times: the digit shrinks step by step and the
      figure reads `90%`, then `82%`, then `74%`.
- [ ] Press `Ctrl` and `0`: the digit returns to the size it launched at and the
      figure reads `100%`.
- [ ] Throughout the steps above the digit never resets to 0 and
      `grep -c PROBE-LOADED "$IM/app.log"` still prints `1` — no gesture
      reloaded the page — and slot A's sidebar row never showed "Starting".
- [ ] Press `Ctrl` and `+` about twenty times fast: one figure stays up and
      keeps updating (never a stack of figures), the page keeps resizing until
      it stops growing at the largest step, the figure then reads `500%` on
      each further press, and it fades about a second after the last press.
- [ ] Press `Ctrl` and `-` until the page stops shrinking: the figure reads
      `25%` on each further press and the page does not get smaller.
- [ ] Park account A (its ⋯ menu → Park). With the slot showing the parked
      panel, click the slot and press `Ctrl` and `+`: the figure appears over
      the parked panel, the panel still shows the name, "Parked" and a Start
      button (three elements, nothing added). Press Start — the page opens
      larger than 100% (the size the figure last named).
- [ ] Add a second account `B` from "Probe" so two slots are filled, switch to
      the two-place arrangement, then remove both accounts so the grid is empty.
      Press `Ctrl` `+`, `Ctrl` `-`, `Ctrl` `0`: nothing happens and no figure
      appears anywhere.
- [ ] `grep -c 'session-grid.css' <(strings target/debug/idle-manager)` is not
      needed — instead confirm the readout is styled: the figure has an opaque
      dark rounded background, not plain text on the page. *(The `(unit)` check
      `the_session_grid_stylesheet_is_readable_from_the_registered_bundle`
      backs the bundle side.)*
