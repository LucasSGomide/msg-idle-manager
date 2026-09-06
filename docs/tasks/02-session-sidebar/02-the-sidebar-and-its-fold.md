# 02 — The sidebar and its fold

**Roadmap:** [02](../../roadmap/02-session-sidebar/README.md) · **Scope:** front-end · **Depends on:** —

## Context

After the first roadmap item, the window can hold up to four game accounts on
screen at once, but nothing stops a user opening more, and an account past the
fourth has no representation anywhere — it runs out of sight with no way to see
that it exists. This slice adds the surface that closes that gap: a fixed-width
column down the left edge of the window that lists every account the application
holds, in the order they were added, whether or not each one currently has a
place on screen.

Each row shows the account's display name and, on its trailing edge, where that
account stands right now: the number of the slot it fills, or a short word
meaning it is running out of sight. An out-of-sight account's name is drawn in a
dimmed style, and the row for the account in the focused slot is marked as the
current one. The row is built with room for more state than this — later items
add whether an account is awake and whether it has stopped responding — so the
piece that turns an account's state into the trailing text is written as one
small function those items extend, not a chain of branches in the row-building
code.

When there are no accounts at all, the column shows a single line of centred
text instead of an empty list, so a new user sees an explanation rather than a
blank strip. Under the list sits a footer box, left empty and hidden in this
slice; a later item fills it with a memory readout, and leaving the slot here
avoids reopening this layout then.

A toggle button in the header bar, on by default, folds the whole column away.
Folded, the grid takes the full window width and every game keeps its slot, its
focus and its page — folding is a view of the state, never a change to it.
Unfolding brings the column back exactly as it was.

This slice renders the list and folds it; it does not yet make a row do anything
when clicked. That is the next slice, which needs both this widget and the domain
rule built alongside it.

## User experience

- **Entry** — a toggle button in the header bar, left of the "Add game" button.
  On by default, so the list is the first thing visible after item 01.
- **Flow** — the list sits to the left of the grid, full window height, one row
  per account in the order the accounts were added.
- **Flow** — each row shows the account's name and, on the trailing edge, its
  place: the number of the slot it occupies, or a marker meaning it is running
  out of sight.
- **Flow** — press the toggle to fold the list away. The grid takes the full
  window width and every game keeps its slot, its focus and its page.
- **States** — **empty**: no accounts, so the list area holds one line of text
  instead of rows. **In a slot**: the row's trailing edge names the slot and the
  row of the focused slot is marked as current. **Out of sight**: the row's
  trailing edge says so and the row's name is drawn in the dimmed style.
  **Folded**: the list is not rendered at all and the toggle reads as off.
- **New pattern** — a persistent index list beside a content area with a per-row
  state marker on the trailing edge, and a fold-away side panel driven from a
  header-bar toggle. The row state marker became `docs/design.md` rule 1 (a word
  plus a coloured dot, one status key driving both); the dimmed style is folded
  into that rule. A rule for which side panels fold is still owed.

## Technical details

- **Front-end** — add `session_sidebar.rs` with its `imp` module and a
  `session-sidebar.ui` template (naming rules 1, 2, 4, 7; architecture rules
  10, 12, 13).
- **Front-end** — the list is a `gtk::ListView` over a `gtk::SingleSelection`
  wrapping a `gio::ListStore` (a `gtk::ListView` recycles rows; `gtk::ListBox` is
  the GTK 3 pattern and does not) of a small `glib::Object` subclass carrying an
  account's identifier, display name and state as properties; the row type is
  `sidebar::Row`, not `SidebarRow` (naming rule 6).
- **Front-end** — a `SignalListItemFactory` binds each row to a `gtk::Box` with a
  leading name label and a trailing state label; the trailing string is derived
  from the domain's `Visibility` in one function so items 03 and 08 extend that
  derivation, not the factory.
- **Front-end** — the empty state is a `gtk::Label` shown in place of the
  `ListView` whenever the store is empty, bound to the same emptiness the
  window's own empty state already watches so the two cannot disagree.
- **Front-end** — restructure `window.rs`/`window.ui` so the content area is a
  horizontal `gtk::Box` whose first child is a `gtk::Revealer` holding the
  sidebar and whose second is the session grid from item 01; bind the header-bar
  toggle to the revealer's `reveal-child`. This slice only renders from
  `SessionBook` state and emits no intent — row activation is task 03
  (architecture rule 8).
- **Code standards** — rule 18: comment the revealer choice with the constraint
  that forced the opposite choice in `session_grid.rs` — the sidebar holds
  labels, so unrealising it when folded costs nothing, unlike a web view.
- **Front-end** — leave an empty, hidden footer `gtk::Box` under the list for
  item 05.
- **Testing** — architecture rule 14 and code standards rule 25: no test in this
  repository may require a display server, so this front-end slice's evidence is
  the item's `test-script.md` and its acceptance criteria are `(manual)`.

## Acceptance criteria

- [x] `(manual)` with several accounts added, the sidebar shows one row per
      account in add order, each with the name on the leading edge
- [x] `(manual)` an account in a slot shows a status marker on its row's
      trailing edge — `Current` with a glowing green dot for the focused slot,
      `Visible` with a plain green dot otherwise — and the focused-slot row's
      name is bold (design rule 1)
- [x] `(manual)` an out-of-sight account's row shows `Background` with a glowing
      amber dot and draws its name in the dimmed style (design rule 1)
- [x] `(manual)` with no accounts, the sidebar area shows a single line of text
      and no rows
- [x] `(manual)` the header-bar toggle is on at startup and the sidebar is
      visible
- [x] `(manual)` pressing the toggle folds the sidebar away, the grid widens to
      the full window, and every session keeps its slot, focus and page; pressing
      it again restores the sidebar unchanged
- [x] `(manual)` an empty, non-visible footer box is present under the list
      (`GTK_DEBUG=interactive` shows it in the widget tree)

## References

- [Roadmap item](../../roadmap/02-session-sidebar/README.md) — the full picture, including the "Folding the list away" diagram
- [Wireframes](../../roadmap/02-session-sidebar/wireframes/session-sidebar.md) — the column, the row layout, the empty and folded cases
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 18, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 6, 7
- [`docs/design.md`](../../design.md) — no numbered rules yet; the index list, the row state marker, the dimmed style and the fold-away panel are patterns it owes a rule for

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
