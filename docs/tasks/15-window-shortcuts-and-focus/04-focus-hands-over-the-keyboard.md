# 04 — Focus hands over the keyboard

**Roadmap:** [15](../../roadmap/15-window-shortcuts-and-focus/README.md) · **Scope:** front-end · **Depends on:** —

## Context

One account at a time is *focused*: it is the one shown in bold in the
sidebar, the one whose slot carries a 2 px outline, and the one the zoom and
reload keys act on. Everything about it says "this is the game you are working
with".

It is not, though — not to the keyboard. Typing a key meant for the game does
nothing until the user clicks inside the page. The click accomplishes nothing
except telling the toolkit what the outline already said, and until it happens
the outline is a lie.

The cause is small. Focusing a slot moves only the application's own record of
which position is focused; nothing ever tells the page's view to take the
keyboard. Both web engines have carried a method for exactly that since the
Windows port, and both still mark it "allow dead code" because no caller ever
appeared. This slice is the caller.

The subtlety is not in grabbing — it is in knowing when not to. A focused
position may hold nothing to focus: a parked, queued or starting account, or
an empty trailing slot on a part-empty last page. In those cases the keyboard
must stay where it is rather than skip sideways to some other slot's view. And
a grab must never take the keyboard out from under something the user is
typing into — a rename dialog's field, the add-game form, the sidebar's
selection mode. The work is one method called from the one place every route
already funnels through, with those guards on it.

## User experience

- **Flow** — Focus an account by any route: a sidebar row, `Shift`+`Tab`,
  `Ctrl`+`Tab`, a pager arrow, a click on its slot, an arrangement change, a
  drag that reorders, a workspace switch, adding it, a restore on startup, or
  a phone choosing it. Its page is live to typing the moment its outline
  appears (`FR.27.1`). No click into the page.
- **States** — The focused position holds no live view — parked, queued,
  starting, or an empty trailing slot: the keyboard is left exactly where it
  was and nothing reaches for another slot's view (`FR.27.2`). When that same
  account's own view goes live, it takes the keyboard then, so starting the
  focused account hands it the keyboard at its first paint.
- **States** — A dialog is open or the sidebar is selecting: nothing is taken
  (`FR.27.3`).
- **States** — The user has deliberately clicked into the sidebar with the
  focus unchanged: a redraw for a memory reading, a liveness change or a phone
  publish does not yank the keyboard back. Only a *change* of focused account
  grabs.
- **Pattern** — The 2 px outline and the bold row stop being decoration and
  start being a claim about where typing goes (design rule 1's "one marker,
  one meaning", read forward).

## Technical details

- **Architecture** — `window/imp.rs` gains
  `last_focus_grab: RefCell<Option<SessionId>>` and one method,
  `follow_focus_with_keyboard`, called at the end of `redraw` after the
  `book` borrow is explicitly dropped (rule 12 — `redraw` holds that borrow to
  its last line today). Hooking `redraw` rather than each of the eleven routes
  is what makes `FR.27.1` hold for a route not yet written; the roadmap item's
  last Blocker records that this is structural, not enforced.
- **Architecture** — the method, in order: read the focused account's id from
  `book.active().focused_session()`; with none, or with no live view for it in
  `holders`, set `last_focus_grab` to `None` and return (`FR.27.2` — and the
  clear is what makes a later start re-grab); return when `last_focus_grab`
  already names it; return when `!self.obj().is_active()`, when
  `self.sidebar.is_selecting()`, or when `self.obj().focus()` downcasts to
  `gtk::Editable` (`FR.27.3`); otherwise call the view's `grab_focus` and
  record the id. A modal dialog is its own toplevel, so `is_active` covers the
  rename dialog and the add-game form with one check; the `Editable` test is
  for an entry inside the main window, which there is none of today and may be
  tomorrow (rule 1 — trust the check, not the absence).
- **Architecture** — `#[allow(dead_code)]` comes off `EngineView::grab_focus`
  in both `web_engine/webkit.rs` and `web_engine/webview2.rs`, and both doc
  comments stop saying no caller exists. Neither engine's implementation
  changes (rules 6, 10).
- **Code standards** — the `holders` borrow for the liveness check and the one
  for the grab are separate short borrows, not one held across `grab_focus`: a
  grab can run arbitrary toolkit code and a re-entrant `redraw` on a held
  `RefCell` is a panic, not a bug report (rule 12, rule 18).
- **Code standards** — `follow_focus_with_keyboard`'s doc comment records why
  `last_focus_grab` exists at all — that without it every redraw would grab,
  and a redraw happens for reasons that have nothing to do with focus (rule
  18).

## Acceptance criteria

- [x] `(integration)` `make verify` passes, with no `dead_code` allow left on
      either engine's `grab_focus`
- [x] `(manual)` with two live accounts on a page, click the second's sidebar
      row and type immediately: the second game receives the keystrokes, the
      first receives none, and no click inside the page was needed
- [x] `(manual)` `Shift`+`Tab` across a page of two live games, typing after
      each step: each game receives what is typed after it is focused
- [x] `(manual)` focusing a parked account leaves the keyboard exactly where
      it was and moves it to no other slot's view; starting that account, with
      it still focused, hands its new view the keyboard at its first paint
- [x] `(manual)` clicking an empty trailing slot on a part-empty last page
      changes neither the focused account nor what holds the keyboard
- [x] `(manual)` open the rename dialog from a row menu and type: the field
      keeps every character and the game behind it receives none; the same for
      the add-game form
- [x] `(manual)` in sidebar selection mode, ticking rows moves the keyboard to
      no view
- [x] `(manual)` a redraw with the focused account unchanged — another
      account's liveness changing, or the memory footer ticking — does not
      pull the keyboard back from a widget the user clicked
- [x] `(manual)` with the page holding the keyboard by grab rather than by
      click, `F5`, `Ctrl`+`+`, `Shift`+`Tab` and the eight new chords all
      still work — the item's first Blocker (`FR.27.4`)

## References

- [Roadmap item](../../roadmap/15-window-shortcuts-and-focus/README.md) —
  Front-end "Focus"; the "Focus following the focused account" diagram;
  Blockers, the first and the last
- [`docs/requirements.md`](../../requirements.md) — `FR.27.1`, `FR.27.2`,
  `FR.27.3`, `FR.27.4`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 6, 10, 12
- [`docs/code-standards.md`](../../code-standards.md) — rules 12, 18
- [`docs/design.md`](../../design.md) — rule 1
- [`docs/naming.md`](../../naming.md) — rule 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
