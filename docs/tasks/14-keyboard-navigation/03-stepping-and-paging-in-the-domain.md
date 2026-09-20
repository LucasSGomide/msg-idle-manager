# 03 — Stepping and paging in the domain

**Roadmap:** [14](../../roadmap/14-keyboard-navigation/README.md) · **Scope:** back-end · **Depends on:** 01

## Context

This application keeps several browser idle games running at once. Each game
is an account, accounts are grouped into workspaces, and the previous slice
made a workspace's accounts read as pages: one order, a page size set by the
arrangement, and a focused position from which the shown page is worked out.

This slice adds the four movements everything visible in this item is built
on, in the pure domain crate — the part of the program that knows the rules
but touches no screen. Two move within a workspace. "Next account" steps the
focused position forward by one and wraps from the last account to the first;
because pages are just runs of that order, stepping past the end of a page
turns to the next page on its own, and a workspace of four accounts shown two
at a time walks A, B, turn, C, D, turn, A. "Next page" and "previous page"
jump a whole page and land on the first place of the new page, wrapping in
either direction. Each answers whether it moved, so a workspace of one account
or one page is a no-op the caller can tell apart.

The third moves between workspaces. "Next workspace" walks the sidebar's order
forward from the shown workspace, wrapping, to the next one holding at least
one account, and makes it the shown one. Empty workspaces are never landed on,
the built-in `Ungrouped` workspace is part of the cycle, and the workspace
landed on shows the page and place it was left on because its own focused
position is untouched. With no other non-empty workspace it answers nothing
and changes nothing. If the phone's mobile mode is on, landing applies the
phone-sized arrangement to the new workspace the way the existing mode code
already does for a switch.

The workspace book also gains one-line pass-throughs for the two in-workspace
movements, so the window never reaches into the shown workspace's own book for
a navigation. Everything here is a unit test on plain values.

It is its own slice, separate from the model change before it, because that
change is large and these movements are the first thing built on it that
nothing else needs to compile — and because the keyboard, the pager and the
Windows path all call these and only these.

## Technical details

- **Architecture** — in `crates/idle-manager-core/src/session.rs`,
  `SessionBook` gains `focus_next() -> bool` (`focused = (focused + 1) % len`,
  `false` for a book of one or none; `FR.23.1`), `next_page() -> bool` and
  `previous_page() -> bool` (`focused = ((page() ± 1) mod page_count()) * k`,
  slot 1 of the new page; `false` when `page_count() <= 1`; `FR.22.5`). All
  three keep the clamp invariant of task 01.
- **Architecture** — in `workspace_book.rs`, `WorkspaceBook` gains
  `focus_next_workspace() -> Option<Switch>`: from the active entry, walk
  `entries` forward with wraparound to the first whose book has at least one
  session, skipping the active one; `None` when no other qualifies
  (`FR.23.2`). Landing sets `active` and, in mobile mode, runs the existing
  `apply_mobile_to_active`; the landed book's `focused` is untouched
  (`FR.18.3`). `Switch { from, to }` is the type `focus_account` already
  returns.
- **Architecture** — `WorkspaceBook` gains `next_page()`, `previous_page()`
  and `focus_next_account()`, each one line delegating to the active book and
  returning its `bool`, so the shell never calls `active_mut()` for a
  navigation (rule 8).
- **Code standards** — one test per behaviour in each file's `tests` module
  (rules 21–24): the four-in-two walk, the wrap, the one-account no-op, the
  part-empty last page, both page wraps, the single-page no-op, the workspace
  walk skipping an empty one, the lone-workspace `None`, the untouched landed
  position, and the mobile-mode landing. Every test is a `WorkspaceBook` or
  `SessionBook` built in memory (rule 25).
- **Naming** — `focus_next`, `next_page`, `previous_page`,
  `focus_next_workspace`, `focus_next_account` are named for their result
  (rule 11); the `bool` they return answers "did it move".

## Acceptance criteria

- [ ] `(unit)` four accounts in `SideBySide` starting on the first:
      `focus_next` four times visits the second, third, fourth and first, with
      `page()` reading `0, 1, 1, 0`
- [ ] `(unit)` `focus_next` on a book of one account and on an empty book
      returns `false` and changes nothing
- [ ] `(unit)` `next_page` lands on the first position of the next page and
      wraps from the last page to the first; `previous_page` wraps from the
      first page to the last
- [ ] `(unit)` `next_page` and `previous_page` return `false` on a one-page
      book, and on three accounts in `SideBySide` `next_page` from page `0`
      focuses the third account alone on page `1`
- [ ] `(unit)` `focus_next_workspace` from `Ungrouped` over `[Ungrouped: A]`,
      `[W1: —]`, `[W2: B]` lands on `W2`, and from `W2` wraps to `Ungrouped`
- [ ] `(unit)` `focus_next_workspace` with a single non-empty workspace returns
      `None` and leaves `active_id()` unchanged
- [ ] `(unit)` after `move_accounts` puts one account into a workspace the walk
      skipped, `focus_next_workspace` lands on it
- [ ] `(unit)` landing on a workspace whose focused position is `3` shows that
      position — its `focused_session()` is the same account as before the
      switch away
- [ ] `(unit)` in mobile mode, landing applies `Layout::Mobile` to the landed
      workspace and `leave_mobile_mode` afterwards restores its layout

## References

- [Roadmap item](../../roadmap/14-keyboard-navigation/README.md) — Back-end
  "Paging and stepping" and "Workspaces" (the `focus_next_workspace` and
  delegation sentences); the "Turning the page" and "Next workspace"
  diagrams' `Book` arrows
- [Wireframes](../../roadmap/14-keyboard-navigation/wireframes/) — no screen;
  the pager in `header-bar-pager.md` calls these
- [`docs/requirements.md`](../../requirements.md) — `FR.22.5`, `FR.23.1`,
  `FR.23.2`, `FR.18.3`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 8, 9
- [`docs/code-standards.md`](../../code-standards.md) — rules 21–25
- [`docs/naming.md`](../../naming.md) — rule 11

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
