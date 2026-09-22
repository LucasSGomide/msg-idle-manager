# 01 — The sidebar and arrangement chords

**Roadmap:** [15](../../roadmap/15-window-shortcuts-and-focus/README.md) · **Scope:** front-end · **Depends on:** —

## Context

The window already answers a handful of keys of its own, all of them through
one small pure function that turns a key and its held modifiers into a named
shortcut, and one matching function that acts on what it decided. Item 14
built that pair so the two web engines — the Linux one and the Windows one —
could never disagree about what a key means.

This slice adds the first four of this item's eight chords, the ones that
work the header bar: `Ctrl`+`B` shows or hides the account list, and
`Ctrl`+`1`, `Ctrl`+`2` and `Ctrl`+`4` arrange the shown workspace for one,
two and four games. None of them is a new behaviour — each is something a
button on the header bar already does. The point is that they are reachable
without leaving the game, and that they reach the *button*, not past it: the
chord presses the toggle, the toggle's own handler does the rest, so the
button's pressed state and the thing it controls cannot drift apart.

The phone arrangement deliberately gets no key. The toggles read `1` `2` `4`
`Phone`, so the only digit left would be `Ctrl`+`3`, which would say "the
third arrangement" while the three keys beside it say "this many games" — a
key you have to memorise rather than read.

One shared piece lands here because these chords are the first to need it:
holding a chord down must run it once. Item 14 solved that for its two
navigation keys with a latch keyed to the `Tab` key specifically; this slice
generalises the latch to every shortcut except reload and zoom, which are the
two a user may legitimately want to repeat by holding.

## User experience

- **Entry** — `Ctrl`+`B`, `Ctrl`+`1`, `Ctrl`+`2`, `Ctrl`+`4`, from anywhere in
  the window including while a game page holds the keyboard.
- **Flow** — Hide the sidebar: press `Ctrl`+`B` → the sidebar folds away and
  the grid takes the full width; the `▤` button leaves its pressed state at the
  same moment. Press again to bring it back.
- **Flow** — Change the arrangement: press `Ctrl`+`2` → the shown workspace
  arranges for two games, the `2` toggle goes pressed, the page holding the
  focused account is the page shown, and every account snaps to the size it
  last chose for that arrangement. From the phone arrangement, any of the three
  leaves it first, exactly as clicking its toggle does.
- **States** — The chord for the arrangement already showing changes nothing
  and saves nothing. Both are consumed either way.
- **States** — Sidebar in selection mode: all four are consumed and inert
  (`FR.26.6`). The sidebar must not fold away under a move-to-workspace
  decision.
- **States** — Held down: each runs exactly once.
- **Pattern** — The chord drives the control, never the model behind it
  (`FR.26.1`, `FR.26.2`): `sidebar_toggle.set_active` and
  `select_layout_toggle`, whose own `toggled` handlers do the work a click
  would.

## Technical details

- **Architecture** — `window/shortcut.rs`: `Shortcut` gains `ToggleSidebar`
  and `Arrange(Layout)`, the latter carrying `idle_manager_core::Layout` so one
  variant covers three chords and `run_shortcut`'s match cannot miss one. The
  module stays pure — no widget, no `Window` — so its tests need no display
  (rule 8; code standards rule 25).
- **Architecture** — `shortcut_for` gains, after the existing reload and zoom
  checks: `ctrl` with `b`/`B` → `ToggleSidebar`; `ctrl` with `_1`/`KP_1`,
  `_2`/`KP_2`, `_4`/`KP_4` → `Arrange(Single)` / `Arrange(SideBySide)` /
  `Arrange(Grid)`. Keypad digits are accepted for the same reason
  `zoom_step_for` already accepts `KP_0`. `Ctrl`+`3` maps to nothing. Modifier
  bits keep being read with `contains`, never by comparing the set for
  equality, so a lock bit riding along cannot spoil a match.
- **Architecture** — `window/imp.rs` `run_shortcut` gains two arms, each
  returning without acting while `self.sidebar.is_selecting()`, following the
  two navigation arms already there (`FR.23.4`, `FR.26.6`): `ToggleSidebar` →
  `self.sidebar_toggle.set_active(!self.sidebar_toggle.is_active())`, with the
  revealer following through the `bind_property` set up in `constructed`;
  `Arrange(layout)` → `self.select_layout_toggle(layout)`, whose `toggled`
  handler reaches `choose_layout` — the same call a click makes, including
  leaving mobile mode first and `finish_layout_change` (rule 8).
- **Code standards** — `tab_held: Cell<bool>` becomes `chord_held: Cell<bool>`
  and stops being about `Tab`: every shortcut but `Reload` and `Zoom` latches
  it, and `connect_key_released` clears it on any key release rather than only
  a `Tab` one. GTK 4 exposes no repeat flag; GDK enables detectable auto-repeat
  on X11 and synthesises Wayland repeat as presses alone, so a held chord
  produces no intervening release (rule 18). The doc comment says so.
- **Design** — `window.ui`: `sidebar_toggle`'s tooltip becomes
  `Show or hide the account list (Ctrl+B)`, and `layout_single`,
  `layout_side_by_side` and `layout_grid` gain `One game (Ctrl+1)`,
  `Two games (Ctrl+2)` and `Four games (Ctrl+4)`. `layout_mobile` keeps the
  tooltip it has, and `layout_toggles`' box-level tooltip stays for the gaps
  between buttons (`FR.25.4`, rule 19).
- **Design** — `resources/ui/help-overlay.ui` gains a second
  `GtkShortcutsGroup` titled `Window`, holding `<Control>b`
  "Show or hide the sidebar", `<Control>1` "One game", `<Control>2` "Two
  games" and `<Control>4` "Four games". Item 14's group keeps the navigation
  and zoom keys (`FR.25.3`, rule 19). No phone key is listed.
- **Naming** — no new file, no new identifier convention; `chord_held` follows
  rule 7's "name the thing, not its mechanism".

## Acceptance criteria

- [x] `(unit)` `shortcut_for` maps `Ctrl`+`b` and `Ctrl`+`B` to
      `ToggleSidebar`, and `Ctrl` with `1`, `2`, `4`, `KP_1`, `KP_2` and
      `KP_4` to `Arrange(Single)`, `Arrange(SideBySide)` and `Arrange(Grid)`
- [x] `(unit)` `Ctrl`+`3`, `Ctrl`+`KP_3`, and an unmodified `b`, `1`, `2` or
      `4` all map to `None`
- [x] `(unit)` `Ctrl`+`b` with `LOCK_MASK` and `MOD2_MASK` added still maps to
      `ToggleSidebar`
- [x] `(unit)` `repeats_while_held` is true for `Reload` and `Zoom` and false
      for every other variant, matched exhaustively so a variant added later
      has to answer it
- [x] `(integration)` `make verify` passes
- [x] `(manual)` with a game page holding the keyboard, `Ctrl`+`B` folds and
      unfolds the sidebar, the `▤` button's pressed state following, and the
      page's own input shows no `b`
- [x] `(manual)` `Ctrl`+`2` arranges the shown workspace for two games with
      the `2` toggle pressed and the focused account still on screen; pressing
      it again changes nothing and writes nothing
- [x] `(manual)` `Ctrl`+`1` / `2` / `4` from the phone arrangement leave it
      first and land on the chord's own arrangement
- [x] `(manual)` holding `Ctrl`+`B` for two seconds folds the sidebar exactly
      once
- [x] `(manual)` in sidebar selection mode all four chords change nothing and
      the game page does not receive them
- [x] `(manual)` hovering `▤` reads `Show or hide the account list (Ctrl+B)`
      and the three digit toggles read their own key; `Ctrl`+`?` lists all
      four under a `Window` group

## References

- [Roadmap item](../../roadmap/15-window-shortcuts-and-focus/README.md) —
  Front-end "The table", "The acts", "Auto-repeat", "Discoverability"; the
  "A window chord, on either engine" diagram
- [`docs/requirements.md`](../../requirements.md) — `FR.26.1`, `FR.26.2`,
  `FR.26.5`, `FR.26.6`, `FR.25.3`, `FR.25.4`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 12, 13
- [`docs/code-standards.md`](../../code-standards.md) — rules 18, 25
- [`docs/design.md`](../../design.md) — rule 19
- [`docs/naming.md`](../../naming.md) — rule 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
