# 03 — The Muted menu item, the window handler and restore

**Roadmap:** [17](../../roadmap/17-account-sound-control/README.md) · **Scope:** front-end · **Depends on:** 01, 02

## Context

The two earlier slices gave the application a per-account "muted" flag that is
saved to disk, and an engine call on each platform that silences a page in
place. Neither is reachable yet: nothing on screen sets the flag, and nothing
carries a saved flag to the engine. This slice closes that loop, and it is the
first moment the owner can actually mute a game.

The owner reaches it from the three-dot menu on an account's row in the
sidebar, the same menu that holds Park or Start and "Keep running when hidden".
The menu's first section gains a check item labelled "Muted", with no keyboard
shortcut. Ticking it sends the account's identity and the requested value to the
window, and nothing else: the item never ticks itself. The window asks the
domain to record the flag. If nothing changed, that is the end of it. If it
did, the window redraws the sidebar, so the menu shows the check ticked the next
time it opens, schedules a save of the workspace file, and tells the account's
holder, which silences the live page or simply remembers the flag for an
account that has no page right now. The account's liveness never moves, so the
row's coloured dot stays as it was and no reloading mark appears.

The same slice makes a saved flag survive a restart. Where the window creates an
account's holder, from a restored workspace, from a single account added later,
or from a brand-new account, it now reads the account's muted flag from the
domain and hands it to the holder, replacing the placeholder the engine slice
left there.

Finally this slice writes the hand-run test script for the whole feature: mute
and unmute a live game, mute a parked one and start it, quit and reopen, and the
same on Windows where a machine is available.

## User experience

- **Entry** — the ⋯ menu on an account's sidebar row, opened by its button, a
  right-click or `Shift`+`F10` (design rule 21). The first section reads
  `Park`/`Start`, `Keep running when hidden`, `Muted`.
- **Flow** — Mute a live account: tick `Muted` → the menu closes, the game goes
  silent at once, the row's dot and name do not change. Open the menu again →
  `Muted` is ticked.
- **Flow** — Mute a parked, queued or starting account: tick `Muted` → the check
  is stored and shows ticked; the page is built muted when it starts.
- **Flow** — Park a muted account and start it again → it comes back muted.
  Quit and reopen → every muted account is still muted.
- **States** — Ticking `Muted` on an already-muted account, or unticking an
  unmuted one, changes nothing: nothing saved, nothing applied.
- **Pattern** — a stateful check item whose handler reports the intent and never
  sets its own state; the row redraws from the domain's answer (design rule 5,
  the keep-awake action at `session_sidebar/imp.rs:932`). No mark on the row
  (design rules 1 and 6); no accelerator shown (design rule 27).

## Technical details

- **Architecture** — `docs/architecture.md` rule 8: intent → transition → state
  → render; the menu item sends `(id, requested)` and the row is rebuilt from
  `Session::is_muted` on the next `SessionSidebar::sync`.
- **Architecture** — `session_sidebar/row/imp.rs`: `#[property(get, set)]
  is_muted: Cell<bool>` mirrored from `Session::is_muted`, set in `Row::new`
  (`row.rs`) beside `is-kept-awake`.
- **Architecture** — `session_sidebar/imp.rs`: `const MUTED_ACTION: &str =
  "muted"`; in `build_row_action_group` a `gio::SimpleAction::new_stateful`
  seeded from `data.is_muted()` whose `change-state` handler calls a new
  `on_muted_toggled` callback cell and never `set_state`; in
  `build_row_menu_model` append `Muted` to `actions_section` right after `Keep
  running when hidden`, no accelerator. `session_sidebar.rs`:
  `connect_muted_toggled(handler: impl Fn(SessionId, bool))`.
- **Architecture** — `window/imp.rs`: connect it beside
  `connect_keep_awake_toggled`; `fn toggle_muted(&self, id, value)` →
  `book.set_muted`; return if unchanged; `redraw()`; `request_save()`;
  `holders.get_mut(id).set_muted(value)`. No `connect_painted`, no
  `finish_starting`.
- **Architecture** — `window/imp.rs`: at the restore loop, the single dormant
  add and the new-account path, read `is_muted` off the session and pass it to
  `SessionView::dormant` / `SessionView::new` in place of task 02's `false`.
- **Design** — `docs/design.md` rules 1, 5, 6, 21, 27 as the wireframe lists
  them. The shortcuts window and `RemoteAccount` are unchanged.
- **Code standards** — rule 21–24 for the `row.rs` test; rule 17 `///` on the
  new public items; rule 29 `make verify` before acceptance.

## Acceptance criteria

- [x] `(unit)` `status_key` is the same whether or not the account is muted, pinned beside the keep-awake test in `row.rs`
- [x] `(integration)` `cargo test --workspace`, `make clippy`, `make fmt-check`, `make arch-check` and `make roadmap-check` pass
- [x] `(manual)` Linux: tick `Muted` on a live account that is playing sound → it goes silent at once, the page does not reload, the row's dot is unchanged, reopening the menu shows the check ticked
- [x] `(manual)` Linux: untick `Muted` → the game sounds again with no reload
- [x] `(manual)` Linux: tick `Muted` on a parked account, start it → it starts silent; its entry in `sessions.toml` carries `muted = true`; quit and reopen → still muted, every other account still sounds and their entries carry no `muted` key
- [x] `(manual)` Windows: the live mute and unmute above, on a current WebView2 runtime (left unticked until a Windows machine runs it, as `FR.28.7` and the roadmap item's blocker record)

## References

- [Roadmap item](../../roadmap/17-account-sound-control/README.md) — the full picture
- [Wireframes](../../roadmap/17-account-sound-control/wireframes/) — the row menu this slice renders
- `docs/architecture.md` · `docs/design.md` · `docs/code-standards.md` · `docs/naming.md`

## Implement with

A fresh implementation session with `Scope: front-end`; `make verify` before acceptance, then this task's section of `test-script.md`.
