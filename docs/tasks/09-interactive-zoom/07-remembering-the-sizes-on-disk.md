# 07 — Remembering the sizes across a restart

**Roadmap:** [09](../../roadmap/09-interactive-zoom/README.md) · **Scope:** full-stack · **Depends on:** 03, 06

## Context

Everything the feature needs now exists, in two halves that have never met. The
gestures work and the sizes they choose survive an arrangement switch, but only
while the window is open — close the application and the evening's work is gone.
Separately, there is a file format and the code to read and write it, wired to
nothing.

This slice joins them. The place in the program that decides what the real
implementations are builds the file-backed one and hands it to the window, which
never learns a file is involved at all.

Writing is deliberately lazy. Turning a wheel produces a burst of steps — twenty
notches in two seconds is normal — and writing the file twenty times would be
absurd. So the page and the figure update instantly and the file waits: each
change restarts a short timer for that account, and only when the changes stop
does the settled size get written, once. Nothing else writes. Switching
arrangement, pausing an account, clicking a different place, closing the window —
none of them touch the file, because none of them is somebody choosing a size.

Reading happens whenever a page is built for an account, whether that is a brand
new account, one being started again after a pause, or one coming back after the
application was closed. Its chosen sizes are read, handed to the rules layer, and
the size for the arrangement in force is what the page opens at — so a game never
appears at the wrong size and then jumps.

Failure is handled by shrugging. A file that cannot be written puts a line in the
log with the account and the reason, and the session carries on; the size is
still right on screen, it just will not survive a restart. A file with one
nonsensical value loses that value and keeps the rest. Nothing here can stop the
application from starting or interrupt anybody, because losing a remembered size
is a small annoyance and refusing to run is not.

## User experience

- **Entry** — no new control and no new gesture. The gestures from the two
  previous slices are the only thing that causes anything to be written.
- **Flow** — choose sizes, close the application, open it again: every account
  comes back at the size chosen for the arrangement being restored, and a game
  started after a pause opens at its chosen size too.
- **States** — **a fresh account**: nothing has ever been chosen for it, so it
  opens at the size its game file asks for and no file is written for it until a
  gesture happens.
- **States** — **a file that cannot be read or written**: the application starts
  and runs exactly as normal and nothing appears on screen. The failure goes to
  the log only — deliberately not the message strip, which design rule 9 reserves
  for something that went wrong before the user did anything, whereas this is the
  consequence of something they just did and can see working.

## Technical details

- **Architecture** — the composition root at
  `crates/idle-manager/src/main.rs:37` builds `TomlZoomMemory` beside the locator
  and the catalogue and hands it to the window as a third `Rc<dyn ...>` port
  (rule 3). The shell must not learn that remembered sizes are a TOML file, and
  `scripts/arch-check.sh` already forbids it depending on the store crate at all.
  The binary returns `anyhow::Result` and the port's error is a `thiserror` enum
  (rule 11).
- **Front-end** — the window holds one settle timer per account id: a zoom change
  cancels that account's pending timer and arms a new one for a short interval,
  and the timer's callback reads the account's remembered map off the book and
  calls the port's `write` (`FR.12.5`). The interval is a named constant in
  milliseconds (code standards rule 5). `save_on_change.rs`'s `Saver` is the
  shape to follow, not the object to reuse: it debounces one whole-workspace
  write, this debounces per account.
- **Front-end** — a failed write is logged with the account id and a reason in
  `tracing` fields and never surfaced (code standards rules 14, 15); it is never
  discarded with `let _ =`.
- **Front-end** — `realise_account` at
  `crates/idle-manager-shell/src/window/imp.rs:264` asks the port for the
  account's remembered sizes, installs them on the book with `restore_zoom`, and
  then passes `zoom_for` the current layout — not the baseline — into
  `SessionView::new` at `crates/idle-manager-shell/src/window/imp.rs:294`, which
  is what makes a page open at the chosen size (`FR.12.7`).
- **Front-end** — `restore_workspace`'s dormant holders take the same resolved
  size, so an account restored on launch is right before it is ever drawn and the
  start queue does not have to correct it.
- **Front-end** — nothing else writes (`FR.12.4`): a layout switch, a park, a
  focus change and a clean shutdown all leave `state.toml` alone. The close
  handler flushes the workspace saver only, and gains no zoom flush — a size
  chosen a moment before quitting is allowed to be lost.
- **Testing** — `(manual)` against the headless harness (architecture rule 14,
  code standards rule 25), appended to `test-script.md` as this task's own
  section. Point `XDG_DATA_HOME` at a temporary tree so the real profile root is
  never touched, and write `state.toml` by hand to set the malformed and
  unwritable cases up.

## Acceptance criteria

- [ ] `(manual)` a run of twenty wheel notches over one account leaves exactly
      one `state.toml` for it, holding the settled size and no intermediate value
- [ ] `(manual)` control and zero removes that arrangement's entry from the file
      and leaves the other arrangements' entries in place
- [ ] `(manual)` switching arrangement, parking an account, clicking a different
      place and closing the window all leave `state.toml` byte-for-byte unchanged
- [ ] `(manual)` closing and reopening the application brings every account back
      at the size chosen for the arrangement that is restored
- [ ] `(manual)` a parked account started again after a relaunch opens at its
      chosen size for the arrangement in force
- [ ] `(manual)` an account with no `state.toml` opens at the size its game file
      asks for, and the launch logs no failure
- [ ] `(manual)` a `state.toml` holding one out-of-range value opens that
      arrangement at the game file's size, keeps the other arrangements' values,
      and logs a warning naming the dropped key
- [ ] `(manual)` with the account's profile root made read-only, the gesture
      still resizes the page and shows the figure, the window carries on, and the
      failed write appears in the log with the account id and a reason

## References

- [Roadmap item](../../roadmap/09-interactive-zoom/README.md) — the full picture,
  including the "Where an account's size comes from" diagram this slice closes
  and the first blocker, identifier reuse across a relaunch
- [The zoom readout wireframe](../../roadmap/09-interactive-zoom/wireframes/zoom-readout.md)
  — the whole item's screen; this slice adds no new drawing to it
- [`docs/requirements.md`](../../requirements.md) — `FR.12.3`, `FR.12.4`,
  `FR.12.5`, `FR.12.6`, `FR.12.7`, `FR.12.8`
- [`docs/design.md`](../../design.md) — rules 4 and 9, and the
  transient-readout rule the item's pattern owes
- [`docs/architecture.md`](../../architecture.md) — rules 3, 8, 10, 11, 12, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 7, 12, 13, 14,
  15, 17, 25
- [`docs/naming.md`](../../naming.md) — rules 2, 8, 10, 11

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
