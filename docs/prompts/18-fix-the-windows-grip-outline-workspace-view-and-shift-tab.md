# Goal: Fix four Windows-only faults in the main window — the hidden drag grip, the focus outline that shows as a bar, the workspace switch that leaves the old pages on screen, and `Shift`+`Tab` walking the page instead of the accounts

**Status:** not executed
**Rating:** —
**Run:** sequential — depends on 17 for the data-root override and the native build steps in `docs/windows-dev.md`

## Context

Reported by the owner on 2026-09-27 on a real Windows PC, release build,
games logged in. Four things that work on Linux do not on Windows:

1. **The drag grip is missing.** The "move" icon that shows at the top right
   of a game on hover never appears, so accounts cannot be reordered by drag.
2. **The focus outline is a bar.** The ring around the focused game shows as a
   thin rectangle along the top of it only, which is very hard to spot.
3. **Switching workspaces does not switch the pages.** Clicking another
   workspace in the sidebar moves the sidebar's highlight, but the grid keeps
   showing the pages of the workspace that was selected before.
4. **`Shift`+`Tab` walks the page, not the accounts.** With a game focused it
   moves focus through the browser's own fields instead of stepping to the
   next account.

Three of them share a cause the project already knows about. On Windows each
game page is a native WebView2 child window sitting above everything GTK
draws (roadmap item 12, design rule 14 in `docs/design.md`). Item 12 moved the
grip and the zoom figure out from under the page; it did not move the focus
outline, the drop tint, or anything about off-grid pages. Pointers from a
survey of the current tree, in `crates/idle-manager-shell/src/`:

- **Grip.** `session_grid/imp.rs`: `build_grip` (~1246) makes the hidden
  `.slot-grip` image; `wire_grip` (~418–517) shows it on a
  motion controller's `enter` and hides it on `leave`, both attached to the
  slot's overlay; a claiming `GestureClick` grouped with a `gtk::DragSource`
  does the drag. On Windows `mount_grip` (~1272–1299) moves the grip into its
  own `.grip-strip` row above the page and `sync_grip_strip` hides the row
  without a live page. Likely fault: the page window takes the mouse before
  GTK sees it, so `enter` on the overlay never fires, and moving onto the
  strip fires `leave`, which hides the grip again. Drag start already
  collapses every page to 0×0 (`host/imp.rs` ~212–225) so the drop tint shows.
- **Outline.** Not CSS: `draw_slot_lines` (~913–969) paints a 3 px border
  inside the slot in `snapshot`, per design rule 26. The Windows host applies
  the widget's **full** size to the child window (`webview2/host/imp.rs`
  ~489–531), so the page covers the border on three sides; the top edge shows
  only because the grip strip sits above the page.
- **Workspace switch.** No `gtk::Stack` and no hide/show: `SessionGrid::sync`
  (~681–732) marks slots on- or off-grid, and `allocate_slots` (~756–786) puts
  off-grid slots just past the grid's bottom-right corner, clipped by GTK but
  still realized so their pages keep running. On Windows the child window
  follows its widget's position (`host/imp.rs` ~139–142, ~489–531) and the
  code trusts Windows to clip it. Judging by the symptom, it does not: the
  off-grid pages stay drawn where they were, above the new workspace's pages.
  The drag path already knows how to make pages vanish and come back.
- **`Shift`+`Tab`.** The shortcut table is `window/shortcut.rs` (~93–130);
  on Linux a capture-phase `EventControllerKey` on the window feeds it. On
  Windows the page holds focus and GTK never sees the key, so
  `webview2/ffi.rs` (~86–150) handles WebView2's `AcceleratorKeyPressed`
  — key-down only, repeats dropped through `WasKeyDown`, `Ctrl` and `Shift`
  read with `GetKeyState` — and `host/imp.rs` (~433–470) maps it through
  `virtual_key.rs`, marks it handled and runs `run_shortcut` on the next idle
  turn. Requirements `FR.23.3`/`FR.23.5` and roadmap item 14 describe exactly
  this, and item 14's Blockers admit `GetKeyState(VK_SHIFT)` was never
  exercised; item 15 says **no** chord has been tried on WebView2. WebView2 also
  has its own idea of `Tab` leaving the page (`MoveFocusRequested`), which the
  code does not handle.

## Constraints

1. **Same ground rules as prompt 17.** The live release app and its
   `%APPDATA%\idle-manager` data are untouchable; every build runs as a test
   copy under the data-root override; the native build follows
   `docs/windows-dev.md`. Read 17's constraints 1, 3, 6, 7 and 9 as if they
   were written here.
2. **Reproduce each of the four on the PC before changing code**, with two
   workspaces of at least two accounts each, and add the observed behaviour to
   the checklist in `docs/windows-dev.md`. Re-run every step after the fix.
3. **The grip is reachable and the drag completes.** On Windows the grip
   shows when the pointer is over the game's slot or its strip, stays while the
   pointer is on the strip, and a drag from it reorders accounts with the drop
   tint visible, exactly the item 10 flow item 12's sequence diagram promises.
   If the cleanest fix is to keep the grip always visible on Windows rather
   than hover-shown, that is acceptable — but then design rule 14 is amended to
   say so and why, in the same commit.
4. **The outline is a ring on all four sides.** The focused game's border is
   visible left, right, top and bottom on Windows, at the width and colour
   design rule 26 sets. Insetting the child window's bounds by the border
   width is the obvious route; whatever route is taken, the drop tint and the
   slot hairlines rule 26 also describes get the same treatment or an explicit
   note in the rule saying why not.
5. **Only the shown workspace's pages are on screen.** After a switch by
   sidebar click, `Ctrl`+`Tab`, or a phone choosing an account, the previous
   workspace's pages are not visible anywhere in the window, the new ones are,
   and the hidden ones keep running (no reload, no `Park`). Reuse the collapse
   and restore the drag already performs rather than inventing a second
   mechanism; check the Linux path is unchanged — it must keep clipping, not
   collapsing.
6. **`Shift`+`Tab` and `Ctrl`+`Tab` do what `FR.23.1`/`FR.23.2` say, with the
   page focused.** The page never sees the `keydown`; the switch runs from the
   idle turn; a held key runs once; selection mode consumes both. Fix it inside
   the existing `AcceleratorKeyPressed` path — no `GtkShortcutController`, no
   `GAction` accelerator, no second key handler (`FR.23.3`, `FR.26.5`). While
   there, run the whole shortcut table on Windows (`F5`, `Ctrl`+`R`, zoom,
   `Ctrl`+`?`, and item 15's `Ctrl`+`B`/`1`/`2`/`4`/`P`/`S` and the two
   `Shift` chords) and record which work; fix a chord only if the cause is the
   one already being fixed, otherwise write it into item 15's Blockers.
7. **Focus follows selection still holds.** After every route to a focused
   account (item 15, `follow_focus_with_keyboard`) the focused page has the
   keyboard on Windows, and nothing steals focus out of a dialog or entry.
   Prove it with the page focused, not the sidebar.
8. **Linux is unchanged.** Every change is behind `cfg(windows)` or in a
   place the survey shows Windows-only; the Linux test suite, `make verify`
   and the headless checks pass as before.
9. **Write the findings down.** Each fault's cause and fix goes into the
   `## As built` or `## Blockers` of the item it belongs to — 10
   (rearranging), 12 (Windows), 14 (keyboard navigation), 15 (window
   shortcuts and focus) — and design rules 14 and 26 are amended where the
   shape changed. Branch `fix/18-windows-grip-outline-workspace-and-shift-tab`;
   `make roadmap-sync` after doc changes on the Linux side.

## Output

Working code on the branch, the four faults reproduced and re-run in the
`docs/windows-dev.md` checklist, the design-rule and roadmap amendments of
constraints 3, 4 and 9, and the record of which shortcuts work on Windows. In
chat, a short summary per fault and the one thing left for the owner: install
the new build over the live app when the games can be paused and confirm the
four behaviours with the real accounts.
