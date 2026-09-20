# 04 — One shortcut table, and the keys on the GTK controller

**Roadmap:** [14](../../roadmap/14-keyboard-navigation/README.md) · **Scope:** front-end · **Depends on:** 03

## Context

This application keeps several browser idle games running at once in one
window, and a game's page owns the keyboard while it has focus. Moving to
another account or another workspace today always means reaching for the
mouse. The domain now knows how to step to the next account and the next
workspace; this slice puts those two movements on keys, on Linux, and makes
the window's key handling a single table both engines can share.

Two keys are added. `Shift`+`Tab` is "next account": the focus moves to the
next account on the page, turns the page when it reaches the end, and wraps
from the last account to the first. `Ctrl`+`Tab` is "next workspace": the
next workspace in sidebar order that holds an account is shown, on the page
and place it was left on. Both are caught by the window before any game page
can see them, through the same early "capture" listener the reload and zoom
keys already use, so a page cannot keep them. Holding a key down does not
repeat: the toolkit gives no "this is a repeat" flag, so the window remembers
that the key is down until it is released and ignores presses in between.
While the sidebar is in its multi-select mode, where the owner is choosing
accounts to move, both keys are swallowed and change nothing, so the screen
never moves under that decision.

The handling itself is split in two. A small pure module turns a key and its
modifiers into one of four named shortcuts — reload, zoom, next account, next
workspace — or nothing, masking out Caps Lock and Num Lock so they cannot
spoil a match, and accepting the three key names the toolkit uses for Tab on
different platforms. A second function runs whichever shortcut was found. The
Linux listener calls the two in sequence; the Windows path, built next,
calls them with a hop in between. Keeping the decision pure means it is unit
tested without a display, and keeping the act separate means the two engines
can never disagree about what a key does.

It is its own slice because it is the first thing a person can feel from this
item, it is complete on Linux by itself, and everything the Windows slice
adds is a caller of what is built here.

## User experience

- **Entry** — `Shift`+`Tab` and `Ctrl`+`Tab` from anywhere in the window,
  including while a game page has the keyboard.
- **Flow** — Next account: press `Shift`+`Tab` → the focus moves to the next
  occupied slot on the page; from the page's last account the page turns and
  the first slot of the next page is focused; from the last account of the
  last page the first page's first slot is focused. Holding the key does not
  repeat.
- **Flow** — Next workspace: press `Ctrl`+`Tab` → the sidebar's next workspace
  that holds an account is shown, on the page and slot it was left on, its
  layout toggle follows → the sidebar rows re-key to it. Empty workspaces are
  skipped; `Ungrouped` is in the cycle. Holding the key does not repeat.
- **States** — One account in the workspace: `Shift`+`Tab` changes nothing.
  One non-empty workspace: `Ctrl`+`Tab` changes nothing. Sidebar in selection
  mode: both keys change nothing. In every case the keys are still consumed,
  so the page never sees them.
- **States** — A workspace holding no account cannot be reached by
  `Ctrl`+`Tab`; it is still reachable by expanding its heading and, once an
  account is moved into it, by the key.
- **Pattern** — The keys go through the one capture-phase controller the
  reload and zoom keys use (`window/imp.rs`, the `key_controller` in
  `constructed`); no second controller is added.

## Technical details

- **Architecture** — add `crates/idle-manager-shell/src/window/shortcut.rs`
  (declared in `window.rs`; code standards rule 9, naming rule 2) holding
  `pub(crate) enum Shortcut { Reload, Zoom(ZoomStep), NextAccount,
  NextWorkspace }` and `pub(crate) fn shortcut_for(key: gdk::Key, modifiers:
  gdk::ModifierType) -> Option<Shortcut>`: mask with
  `gtk::accelerator_get_default_mod_mask()`, then `F5` or `Ctrl`+`r` →
  `Reload`; `Ctrl` plus a zoom key → `Zoom` (moving `zoom_step_for` from
  `window/imp.rs` here); `Shift` plus `Tab`, `ISO_Left_Tab` or `KP_Tab` →
  `NextAccount`; `Ctrl` plus the same three → `NextWorkspace` (`FR.23.3`).
- **Architecture** — `Window::handle_shortcut_key(key, modifiers) -> bool`
  becomes `shortcut_for` followed by `run_shortcut`; add `pub(crate) fn
  run_shortcut(&self, shortcut: Shortcut)`: `Reload` and `Zoom` exactly as
  today; `NextAccount` → `book.focus_next_account()` then `sync_watched`,
  `redraw`, `request_save`; `NextWorkspace` → `book.focus_next_workspace()`
  and on `Some(switch)` `select_layout_toggle(layout)`,
  `snap_zoom_for_active`, `sync_watched`, `redraw`, then `request_save`
  (the body `focus_session` runs for a switch today — factor it into
  `after_workspace_switch(&Switch)` so both call one function). Both
  navigation arms return before acting when `self.sidebar.is_selecting()`
  (`FR.23.4`); the key is consumed either way (rules 8, 10). The shown
  workspace being empty makes both a no-op by the book's own answer.
  `handle_shortcut_key` stays `pub(crate)`.
- **Architecture** — `SessionSidebar` gains `pub(crate) fn is_selecting(&self)
  -> bool` in `session_sidebar.rs`, reading the existing `is_selecting` cell
  in its `imp`.
- **Architecture** — the capture-phase `gtk::EventControllerKey` in
  `window/imp.rs` `constructed` gains a `key-released` handler and the window
  a `tab_held: Cell<bool>`: a press mapping to `NextAccount` or
  `NextWorkspace` while `tab_held` is set is consumed and ignored; a press
  that runs one sets it; a release of `Tab`, `ISO_Left_Tab` or `KP_Tab`
  clears it. The release propagates — WebKitGTK re-queues an unhandled press
  but never sees the one stopped in capture, so the page receives a release
  with no press and ignores it (`FR.23.3`).
- **Code standards** — the latch carries a comment naming the constraint: GTK
  4 exposes no auto-repeat flag on a key event (rule 18). `shortcut_for` is
  unit tested in `shortcut.rs`'s `tests` module with `gdk::Key` constants and
  `gdk::ModifierType` values, which need no display (rules 21–25): each
  mapping, `ISO_Left_Tab` and `KP_Tab`, `Tab` alone → `None`, and Caps Lock /
  Num Lock added to a matching chord still matching.
- **Code standards** — the first Blocker is settled here: if
  `GtkWindow`'s bubble-phase `move-focus` binding is found to pre-empt the
  capture controller for `Shift`+`Tab` while a `WebKitWebView` holds focus,
  the fallback is a `gtk::ShortcutController` in capture phase with scope
  `Global` on the same window, carrying the constraint as a comment (rule 18).
  The measurement is a runbook step.
- **Design** — nothing new is drawn; the existing focus outline and sidebar
  bold move (design rule 1).

## Acceptance criteria

- [x] `(unit)` `shortcut_for` maps `F5` and `Ctrl`+`r` to `Reload`, `Ctrl`
      plus each zoom key to the matching `Zoom(step)`, and every other key
      without a modifier to `None`
- [x] `(unit)` `shortcut_for` maps `Shift` plus `Tab`, `ISO_Left_Tab` and
      `KP_Tab` to `NextAccount` and `Ctrl` plus the same three to
      `NextWorkspace`; `Tab` alone is `None`
- [x] `(unit)` `shortcut_for` with `LOCK_MASK` and `MOD2_MASK` added to a
      matching chord still returns the same shortcut
- [x] `(integration)` `make verify` passes
- [ ] `(manual)` on X11 with four accounts in `2` and a game canvas focused,
      `Shift`+`Tab` four times walks A, B, page turn, C, D, page turn, A, the
      sidebar bold following, and the page's own input shows no `Tab`
- [ ] `(manual)` with three workspaces, one empty, `Ctrl`+`Tab` three times
      lands on the two non-empty ones and wraps, each on the page and slot it
      was left on, the layout toggle following; the empty one is never shown,
      is still reached by expanding its heading, and is reached by the key
      once an account is moved into it
- [ ] `(manual)` holding `Ctrl`+`Tab` for two seconds switches exactly once;
      holding `Shift`+`Tab` steps exactly once
- [ ] `(manual)` in sidebar selection mode both keys change nothing and the
      game page does not receive them; leaving selection mode restores them
- [ ] `(manual)` with one account in the workspace `Shift`+`Tab` changes
      nothing, and with one non-empty workspace `Ctrl`+`Tab` changes nothing

## References

- [Roadmap item](../../roadmap/14-keyboard-navigation/README.md) — Front-end
  "One shortcut table" and "Auto-repeat"; the "Turning the page" and "Next
  workspace" diagrams; Technical References, the second, third and sixth;
  Blockers, the first
- [Wireframes](../../roadmap/14-keyboard-navigation/wireframes/) — no screen;
  `header-bar-pager.md` names the `Shift`+`Tab` tooltip task 06 adds
- [`docs/requirements.md`](../../requirements.md) — `FR.23.1`–`FR.23.4`,
  `FR.11.1`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12
- [`docs/code-standards.md`](../../code-standards.md) — rules 9, 18, 21–25
- [`docs/design.md`](../../design.md) — rule 1
- [`docs/naming.md`](../../naming.md) — rules 2, 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The `(manual)` steps run
under X11 (`make dev`) with a real keyboard.
