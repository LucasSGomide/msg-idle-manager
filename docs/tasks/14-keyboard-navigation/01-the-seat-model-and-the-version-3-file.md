# 01 — The seat model: pages derived from one order, and the version 3 file

**Roadmap:** [14](../../roadmap/14-keyboard-navigation/README.md) · **Scope:** full-stack · **Depends on:** —

## Context

This application keeps several browser idle games running at once on a desktop
that stays on all day. Each game is an account; accounts are grouped into
workspaces, and the window shows one workspace at a time in an arrangement of
one, two or four places on screen, or one phone-sized place. Today every
account remembers a place of its own, and when there are more accounts than
places the extra ones sit "off-grid": still running, unseen, and swapped onto
screen by a sidebar click that pushes someone else off. Two facts are stored
for each account — its place and whether it is visible — and they can drift
apart.

This slice replaces that with pages. A workspace's accounts have exactly one
order, the order the sidebar already lists them in, and the screen shows one
page of that order: the first two accounts, then the next two, and so on, with
the page size set by the arrangement. The book that holds a workspace now
keeps three things — the order, the arrangement, and the position of the
focused account — and everything else is worked out from them: which page is
showing, which place each account is in, and which accounts are out of sight.
Nothing is stored that could disagree with anything else. Switching
arrangement keeps the focused account on screen, adding an account turns to the
page holding it, clicking an account in the sidebar turns to its page, and
dragging one account onto another swaps them within the page.

The saved workspace file has to change with it, because it no longer records a
place per account. Its version number becomes 3. A file written by the previous
version is read once and rebuilt from its list order, and a copy is kept
beside it so an older build can still open what it wrote. The mobile mode the
phone feature uses is simplified the same way: leaving it puts back the
arrangement and the focused position, and the order was never touched.

It is one slice because the domain type, the file that stores it and the three
places in the window that read it all change shape together — there is no way
to land the new model and leave the old file readable, or the old model and the
new readers, and keep the program compiling. Every slice after this one builds
on the answers this one gives.

## User experience

- **Flow** — Click a sidebar name: the page holding that account is shown and
  its slot is focused; in the one-slot layout this is exactly today's swap.
  Nothing else moves.
- **Flow** — Change layout: the page holding the focused account is shown at
  the new size.
- **Flow** — Add an account to the shown workspace: it takes the end of the
  order; the screen turns to the page holding it and focuses it.
- **Flow** — Drag an account onto another slot of the same page: the two swap,
  or the account moves into the empty slot; the sidebar's order follows. No
  drop can reach another page.
- **States** — Last page part-empty: trailing slots show nothing, and a click
  on one changes nothing — the focus stays where it was.
- **Pattern** — Sidebar rows keep their `current` / `visible` / `background`
  meaning (design rule 1); `visible` now means "on the shown page".

## Technical details

- **Architecture** — in `crates/idle-manager-core/src/session.rs`, `Session`
  loses `visibility` and `remembered_slot`; `SessionBook` keeps `sessions:
  Vec<Session>` as the one order, `layout: Layout`, and `focused: usize` (the
  position of the focused account) in place of `focused: SlotId` and the
  `remembered` map. With `k = layout.slot_count()`: `page_of(p) = p / k`,
  `slot_of(p) = SlotId::new(p % k)`, `page() = page_of(focused)`,
  `page_count() = sessions.len().div_ceil(k)` (`0` when empty),
  `placement(&SessionId) -> Option<Visibility>` answering
  `InSlot(slot_of(p))` on the shown page and `OffGrid` elsewhere,
  `focused_slot() -> SlotId`, and `focused_session() = sessions.get(focused)`
  (code standards rule 1; `FR.22.1`, `FR.22.2`).
- **Architecture** — `focused` is clamped on every mutation (`0` when empty,
  else `< len`), with a private `keep_focus_on(id_before)` so a reorder or a
  removal keeps the same account focused unless it is gone. `set_focused`
  becomes `focus_slot(SlotId) -> bool`, moving to `page() * k + slot.index()`
  only when that position holds an account. `set_layout` changes `layout`
  alone. `focus_session(id)` sets `focused` to `id`'s position and returns
  whether the page changed (`FR.22.3`). `add` / `add_from_preset` push to the
  end and focus the newcomer; `adopt` pushes and leaves `focused` alone;
  `remove` / `take` drop and clamp. `Placement`, `bring_into_focus`,
  `Arrangement`, `current_seats`, `seats_under`, `restore_arrangement`,
  `workspace_as` and `layout.rs`'s `arrange` are deleted, and `lib.rs` stops
  re-exporting `Placement` and `arrange`.
- **Architecture** — `move_to_slot(account, target: SlotId) -> MoveOutcome`
  keeps its name and outcomes on positions (`FR.22.4`): source `p` is the
  account's index, target is `page() * k + target.index()`; an occupied target
  swaps the two `Vec` entries (`Swapped`); a target past the end of the order
  removes `p` and pushes to the end (`Filled`); `Unchanged` for the same
  position, a slot outside the layout, an unknown id or a source off the shown
  page. `keep_focus_on` runs after either change. `reorder_by_placement` and
  `occupied_slots` go with the old model.
- **Architecture** — in `workspace_book.rs`, `MobileMode`'s snapshot becomes
  `(Layout, usize)` per workspace; `leave_mobile_mode` restores both,
  `apply_mobile_to_active` becomes `set_layout(Layout::Mobile)` after taking
  the snapshot, and `saved()` reports the snapshot's layout and position while
  the mode is on (Remote Access `FR.3.4`, `FR.3.5`). `focus_account` keeps its
  signature and calls the book's `focus_session`. `NAMED_WORKSPACE_CAPACITY`
  is untouched. In `workspace.rs`, `Account` loses `visibility` and
  `remembered_slot`, and `Workspace::focused` becomes `usize`.
- **Architecture** — in `crates/idle-manager-store/src/session_file.rs`,
  `FORMAT_VERSION` becomes `3`; `SessionEntry` drops `slot`;
  `WorkspaceRecord::focused` is the position in the account list (rule 7). A
  version 3 file is read as is. A version 2 file is read once: list order is
  the order, `focused` is the position of the account whose `slot` equals the
  record's `focused`, or `0` when none holds it. The first `save` over a
  version 2 file copies it to `sessions.v2.toml` beside `sessions.toml`,
  exactly as `V1_BACKUP_FILE` does for version 1 (`snapshot_v1_backup`
  generalises to both). A version 1 file still migrates through `parse_v1`,
  landing in version 3's shape. `SessionFileError::UnsupportedVersion`'s
  message names versions 1, 2 and 3.
- **Architecture** — the shell's three readers change only where the old
  names were: `session_grid/imp.rs` `sync` keeps reading
  `book.placement(&entry.id)` — the derived answer has the same shape, so the
  placing code, `slot_rect`, `sync_grip_strip` and the drag source need no
  change — reads `book.active().focused_slot()` in place of `focused()`, and
  the click-to-focus handler stops setting
  `self.focused` itself — it reports the slot through `on_slot_focused`, the
  window calls `book.active_mut().focus_slot(slot)` and redraws only when it
  returns `true` (rule 8); `reload_focused` and `is_focused_session` are
  unchanged. `session_sidebar/imp.rs` computes a row's `current`
  flag as `book.active().focused_session().map(Session::id) ==
  Some(session.id())`; `status_key` is unchanged. `row.rs`'s tests stop
  calling `Session::visibility()`.
- **Code standards** — every deleted seat test in `session.rs`'s `tests`
  module is replaced by one pinning the derived answer (rules 21–24); the
  store's integration tests live in
  `crates/idle-manager-store/tests/the-workspace-file-on-disk.rs` (rule 25;
  naming rule 3), with a version 2 fixture written as a literal string.
- **Naming** — `focus_slot`, `focused_slot`, `page`, `page_count` and
  `placement` are named for their result (rule 11); `sessions.v2.toml`
  follows `sessions.v1.toml`.

## Acceptance criteria

- [ ] `(unit)` four accounts in `SideBySide` with the third focused: `page()`
      is `1`, `page_count()` is `2`, `placement` answers `InSlot(0)` and
      `InSlot(1)` for the third and fourth and `OffGrid` for the first two
- [ ] `(unit)` switching from `Grid` to `Single` with the third of four focused
      answers `InSlot(0)` for that account, and switching back to `Grid`
      answers a slot for all four again
- [ ] `(unit)` `focus_slot` on a trailing slot past the end of the order
      returns `false` and leaves `focused_session()` unchanged
- [ ] `(unit)` `add` and `add_from_preset` append to the order and focus the
      newcomer; `adopt` appends and leaves `focused_session()` as it was
- [ ] `(unit)` taking the account before the focused one keeps
      `focused_session()` the same account; taking the focused last account
      clamps to the new last
- [ ] `(unit)` `move_to_slot` onto an occupied slot returns `Swapped` and trades
      exactly those two positions, onto a trailing empty slot returns `Filled`
      and moves the account to the end, and every position off the shown page
      is unchanged either way, with focus following the mover
- [ ] `(unit)` `leave_mobile_mode` restores each snapshotted workspace's layout
      and focused position, and `saved()` while the mode is on reports the
      pre-mobile layout and position
- [ ] `(integration)` a version 3 file round-trips: `focused` is written as a
      position, no account carries a `slot` key, and the list read back equals
      the one saved
- [ ] `(integration)` a version 2 file whose `focused` slot is held maps to
      that account's position, one whose `focused` slot no account holds maps
      to `0`, and a version 1 file lands in the version 3 shape
- [ ] `(integration)` the first `save` over a version 2 file writes
      `sessions.v2.toml` byte-identical to the original exactly once, and a
      second `save` leaves it untouched

## References

- [Roadmap item](../../roadmap/14-keyboard-navigation/README.md) — Back-end
  "The seat model", "Drops", "Workspaces" (the `MobileMode` and `Workspace`
  paragraphs) and "The file"; Front-end "The grid" and "The sidebar" (the
  `current` flag); the "Turning the page" diagram's `Grid::sync` and
  `Sidebar::sync` arrows
- [Wireframes](../../roadmap/14-keyboard-navigation/wireframes/) — none of
  the three screens is drawn here; this slice changes what the existing grid
  and sidebar show
- [`docs/requirements.md`](../../requirements.md) — `FR.22.1`–`FR.22.4`;
  Remote Access `FR.3.4`, `FR.3.5`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 7, 8, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 21–25
- [`docs/design.md`](../../design.md) — rule 1
- [`docs/naming.md`](../../naming.md) — rules 3, 11

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
