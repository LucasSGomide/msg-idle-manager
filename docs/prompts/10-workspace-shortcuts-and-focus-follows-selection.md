# Goal: Add six window shortcuts — sidebar, the three arrangements, park/start one account and a whole workspace — and make focusing an account hand its game the keyboard, so the extra click inside the page goes away

**Status:** executed
**Rating:** —
**Run:** parallel with 09 — 09 writes a design brief and no code. Land this one first: 09's output will later redraw the same header bar, tooltips and shortcuts window this prompt touches.

## Context

Two changes, one item: the keys the window answers, and what "focused" actually
means.

**The keys.** The window already funnels every shortcut through one pure
function, `shortcut_for` in `crates/idle-manager-shell/src/window/shortcut.rs`,
which turns a `gdk::Key` plus modifiers into a `Shortcut` variant, with
`Window::run_shortcut` in `window/imp.rs` as the half that acts. Both engines
feed it: on Linux a GTK key controller in the capture phase, on Windows
`WebView2`'s `AcceleratorKeyPressed` through `gdk_key_for_virtual_key` in
`web_engine/virtual_key.rs`. Today it answers `F5` / `Ctrl`+`R` (reload),
`Ctrl`+`+` / `-` / `0` (zoom the focused account), `Shift`+`Tab` (next account),
`Ctrl`+`Tab` (next workspace) and `Ctrl`+`?` (the shortcuts window). Six more
are wanted, all of them the window's and none of them ever reaching a game's
page:

| Chord | What it does |
| --- | --- |
| `Ctrl`+`B` | Show or hide the sidebar — the same effect as the `▤` header-bar toggle |
| `Ctrl`+`1` | Switch the shown workspace to the one-game arrangement |
| `Ctrl`+`2` | Switch it to two games side by side |
| `Ctrl`+`4` | Switch it to four games in a grid |
| `Ctrl`+`P` | Park the focused account |
| `Ctrl`+`S` | Start the focused account |
| `Ctrl`+`Shift`+`P` | Park every account in the shown workspace — the heading menu's `Park all` |
| `Ctrl`+`Shift`+`S` | Start every parked account in the shown workspace — its `Start all` |

**The focus bug.** An account can be focused — it is the one the sidebar shows
in bold, the one whose slot carries the 2 px outline, the one the zoom and
reload keys act on — and still not have the keyboard. Pressing a key meant for
the game does nothing until the user clicks inside the page, which is an
entirely wasted click and makes the outline a lie. The reason is that focusing a
slot only moves the application's own idea of the focused position: nothing
calls the view's `grab_focus`. Both engines already have the method —
`web_engine/webkit.rs` (`self.view.grab_focus()`) and
`web_engine/webview2.rs` (`self.host.focus_view()`) — and both are still marked
`#[allow(dead_code)]` because nothing has ever called them. Wanted: whenever the
focused account changes, by any route, the focused account's view takes the
keyboard, so the page is live to typing the moment its outline appears.

This is a change to how the application behaves, so it goes through the
project's own planning flow before any code is written — requirements first,
then a roadmap item, then a task breakdown, then a branch. See constraint 1.

## Constraints

1. **Requirements first, then the roadmap, then the breakdown.** Run
   `/msg-pre-roadmap` to append the user needs and functional requirements to
   `docs/requirements.md` (the log currently ends at `UN.25` / `FR.25.2`; take the
   next free numbers, never reuse one), then `/msg-roadmap-plan-item` to open the
   item, then `/msg-roadmap-task-breakdown`. Create the session branch named with
   the item's number — `but branch new` if GitButler is set up, else
   `git checkout -b` — **before the first code edit**; a `PreToolUse` hook blocks
   a code Write without one.
2. **One shortcut table, two engines, no second key path.** Every new chord is a
   new `Shortcut` variant decided in `shortcut_for` and acted on in
   `run_shortcut`. `shortcut_for` stays pure and testable with no display: it
   reads named modifier bits with `contains`, never compares the modifier set for
   equality, so Caps Lock or Num Lock riding along can never spoil a match. Do not
   add a `GtkShortcutController` accelerator, a `GAction` accelerator or a second
   key handler for any of these.
3. **Windows parity is part of the work, not a follow-up.** Extend
   `gdk_key_for_virtual_key` with every virtual key the new chords need (`B`,
   `P`, `S`, `1`, `2`, `4`), keep marking the event handled so the page never
   sees a `keydown`, and run the action from the main loop's idle rather than
   inside the `AcceleratorKeyPressed` callback — that
   callback runs with the browser process blocked. The Windows machine may not be
   available: where a chord cannot be exercised there, say so in the item's
   blockers and in `test-script.md` rather than claiming it works.
4. **These keys belong to the window, and the page never gets them.** `Ctrl`+`S`
   and `Ctrl`+`P` are keys a game's page would otherwise take for save and print,
   and `Ctrl`+`1` / `2` / `4` are keys some pages bind themselves. Taking them in
   the capture phase is deliberate: record it as a requirement, with the
   trade-off stated, the way `FR.23.3` already records `Shift`+`Tab` costing a
   page its backward field focus.
5. **Discoverability is design rule 19, and it is not optional.** Every new chord
   gets a line in `resources/ui/help-overlay.ui`, and every control that mirrors
   one names it in its tooltip: the sidebar toggle (`Ctrl`+`B`), each arrangement
   toggle (`Ctrl`+`1` / `2` / `4`; the phone toggle keeps the tooltip it has),
   and the heading menu's `Park all` / `Start all` and a row's `Park` / `Start`
   through their menu items' accelerator text.
6. **Splitting park and start into two chords departs from design rule 2 —
   record the departure and why it is cost-free.** The rule asks for one control
   whose label and effect invert with the account's state, for two reasons: so it
   can never be pressed in a direction that does not apply, and because a pair of
   buttons would always have one half disabled on a scarce trailing edge (rule
   6). Two chords keep the rule's *shape* broken, but neither reason survives
   contact with a keyboard. The inapplicable direction is a silent no-op
   (constraint 7), so there is no wrong state to reach and nothing to undo; and a
   key occupies no trailing edge, so the second reason never applied. What is
   left is that a key carries no label to read the direction off — and the row's
   dot already answers that, naming which of the two keys is live before either
   is pressed. A single flip key would obey the letter and read worse: nothing
   would say which direction the press is about to take, and liveness moves on
   its own (queued → starting → live) between deciding and pressing. So: two
   chords, each idempotent — `Ctrl`+`S` always means running, `Ctrl`+`P` always
   means parked. Write that reasoning into the roadmap item's Design bullet and
   either narrow rule 2 (its one-control reasoning governs a control with a
   visible label; a shortcut takes one key per direction, inert where it does not
   apply) or add a rule beside it. Do not leave the contradiction unrecorded.
7. **A chord that does not apply does nothing, quietly.** `Ctrl`+`P` on an
   already-parked account, `Ctrl`+`S` on one that is live or starting,
   `Ctrl`+`Shift`+`P` on a workspace with nothing running, `Ctrl`+`Shift`+`S` on
   one with nothing parked: consumed, no-op, no message strip and no sound —
   matching the greyed menu item that already means "this would touch nothing".
   While the sidebar is in selection mode every new chord is consumed and inert,
   exactly as `Shift`+`Tab` and `Ctrl`+`Tab` are under `FR.23.4`. Holding a chord
   must not repeat.
8. **Reuse the paths the mouse already uses; do not reimplement them.**
   `Ctrl`+`1` / `2` / `4` go through whatever clicking the arrangement toggle
   goes through, so the toggle's own state, the page the focused account sits on
   and `snap_zoom_for_active` all follow. `Ctrl`+`B` drives the `sidebar_toggle`
   button's active state so the button and the revealer can never disagree.
   `Ctrl`+`P` / `Ctrl`+`S` go through the row's existing parking handler and
   `Ctrl`+`Shift`+`P` / `Ctrl`+`Shift`+`S` through the heading's `park_all` /
   `queue_parked` and the start queue, all reading the domain for the direction —
   the shell decides nothing (architecture rule 8).
9. **The phone arrangement gets no key.** The toggles read `1` `2` `4` `Phone`,
   so the only digit left is `Ctrl`+`3`, which would say "the third
   arrangement" while the three keys beside it say "this many games" — a key
   that has to be memorised rather than read is worse than no key. Decided:
   leave it on its toggle for now. Do not invent one, and do not list a phone
   key in the shortcuts window.
10. **The focus fix covers every route to a focused account, not just the
    sidebar.** A sidebar row click, `Shift`+`Tab`, `Ctrl`+`Tab`, a pager arrow, a
    click on a slot, an arrangement change, a drag that reorders, a workspace
    switch, an account added, a restore on startup, and a phone choosing an
    account: each one that lands on a live view hands that view the keyboard.
    Drop the `#[allow(dead_code)]` from both engines' `grab_focus` once it is
    called.
11. **What the focus fix must not break.** The window's capture-phase key
    controller must keep firing with a page holding focus — that is what makes
    `Shift`+`Tab` and the new chords work at all, so prove it with the page
    focused, not just with the sidebar focused. A focused slot holding no live
    view (parked, queued, starting, or an empty trailing slot on a part-empty
    page) has nothing to focus: leave the keyboard where it is rather than
    reaching for another slot's view. And nothing may steal focus out from under
    a dialog or a text entry — a rename dialog's entry, the add-game form, the
    sidebar's own selection mode — so check what holds focus before grabbing it.
12. **Acceptance is the project's own two acts.** Tick every box under the task's
    `## Acceptance criteria` only once a passing automated test backs it —
    `shortcut_for`'s new arms are plain unit tests with no display, the domain
    side (park-all, arrangement change, focus movement) is unit tested in
    `idle-manager-core`, and anything needing a widget is an integration test —
    and write the task's own section into `docs/tasks/<item>/test-script.md`, one
    checkbox per concrete action and the result it must produce. The keyboard
    checks are hand-run: the headless route on this machine is Xvfb plus the
    dlopen-XTest helper already used for earlier items' `(manual)` checks. Never
    tick a box for work that does not exist.
13. **`make verify` passes before the work is called done** — formatting, clippy,
    tests, dependency audit, layer boundaries and the roadmap check — and
    `make roadmap-sync` is run after any checkbox or status changes.

## Output

Working code in the repository, reached through the planning flow of constraint
1: the appended requirements, the roadmap item with its Design and Architecture
bullets, the task breakdown under `docs/tasks/<item>/`, the implementation on a
branch named for the item, the new `help-overlay.ui` lines and tooltips, the
tests behind every ticked acceptance box, and the item's `test-script.md`
section for the hand-run keyboard checks.
