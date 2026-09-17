# Selection mode

## Purpose

Where the user ticks accounts and moves them into a workspace, or creates a new
workspace from them.

## Where it sits

Renders the selection-mode `**Entry**` bullet, the grouping `**Flow**` bullets
(from `Select` through choosing a destination, `New workspace…` and `Done`), the
"`Move to…` with nothing ticked", "ticked set larger than four" and "collapsed
heading" `**States**`, the "Grouping accounts in selection mode" diagram, and the
Selecting state of the "Sidebar modes" diagram.

## The screen

The same sidebar with the toggle reading `Done`. Every account row gains a
leading tick box inside its indentation; headings lose their ⋯ buttons, and
account rows keep theirs. A click anywhere on an account row ticks or unticks it
and never switches or focuses. A bar appears between the list and the memory
footer: a count on the left ("2 ticked") and a `Move to…` menu button on the
right, insensitive at zero. The menu lists every named workspace with room for
the whole ticked set, then `Ungrouped`, then a separator and `New workspace…`,
which is insensitive when more than four accounts are ticked. A workspace
without room is not listed. Collapsing a heading hides its rows but keeps their
ticks, and the count does not change.

```
┌────────────────────────────────┐
│ Accounts                [Done] │
│ ▾ Party A                      │
│   ☑ Main         ●  ◆        ⋯ │
│   ☐ Alt 1        ●           ⋯ │
│ ▸ Party B                      │   collapsed, ticks kept
│ ▾ Ungrouped                    │
│   ☑ Farm         ●           ⋯ │
├────────────────────────────────┤
│ 2 ticked          [Move to… ▾] │
├────────────────────────────────┤
│ Shell                  142 MiB │
└────────────────────────────────┘

Move to… menu (Party B is full, so it is not listed)
┌──────────────────┐
│ Party A          │
│ Ungrouped        │
├──────────────────┤
│ New workspace…   │
└──────────────────┘
```

Choosing a workspace moves the ticked accounts to the bottom of its list, ends
the mode and clears every tick; the shown workspace does not change. Choosing
`New workspace…` opens the workspace name window; cancelling it returns here
with the ticks intact. `Done` ends the mode and clears the ticks.

## Design rules

- Rule 1 — the status dots are unchanged in this mode; a tick is not a state and
  never takes a dot's place
- Rule 5 — account rows keep their one ⋯ menu on the trailing edge while
  selecting
- Rule 6 — the tick box is paid for by the 200 px width, so the name still gives
  way only in length
- Rule 7 — `New workspace…` is the escape hatch below the real destinations, set
  apart by a separator rather than a second control
