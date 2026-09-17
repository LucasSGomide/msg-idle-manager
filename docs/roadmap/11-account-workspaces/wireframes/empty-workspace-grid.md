# Empty workspace grid

## Purpose

What the grid area shows when the workspace on screen holds no accounts, kept
distinct from the application's first run.

## Where it sits

Renders the "shown workspace empty" and "no accounts anywhere" `**States**`
bullets, and the `**Pattern**` bullet reusing the window's centred empty box.
It is reached by the grouping `**Flow**` (moving the last account out of the
shown workspace) and the removing and deleting flows.

## The screen

The grid area to the right of the sidebar is replaced by one centred, dim line
reading "No games in this workspace", with no button. The header's 1/2/4 buttons
and `Add game` stay where they are, and the sidebar is unchanged. When no
workspace holds any account at all, the existing first-run state shows instead:
"No games yet — add one to get started." and its `Add your first game` button,
unchanged.

```
┌─────────┬──────────────────────────────────────────┐
│ Sidebar │                                          │
│         │                                          │
│         │        No games in this workspace        │   dim, centred
│         │                                          │
│         │                                          │
└─────────┴──────────────────────────────────────────┘
```

## Design rules

- Rule 4 — an absence is a plain centred element on the window's background,
  never a busy card
- Rule 9 — this is not the message strip; an empty workspace is a state the
  user made, not a problem to dismiss
