# Workspace heading menu

## Purpose

Where the owner parks or starts a whole workspace in one act, and where a
heading names the key that reaches the next workspace.

## Where it sits

Renders the `Park all` / `Start all` `**Entry**` bullet, the "Park all" and
"Learn the keys" `**Flow**` bullets, the "Heading menu" and "Sidebar in
selection mode" `**States**` bullets, and the "Park all and Start all"
interaction diagram.

## The screen

The sidebar tree as it is today. A named workspace heading's ⋯ menu gains a
first section holding `Park all` and `Start all`, above a divider and the
existing `Rename…` and `Remove workspace`. The `Ungrouped` heading, which had
no ⋯ button, gains one whose menu holds the first section only. Each item is
greyed when it would touch no account: `Park all` when nothing in the workspace
is live, queued or starting; `Start all` when nothing is parked; both in an
empty workspace. In selection mode the ⋯ button is hidden, as today. Hovering a
heading's name reads `Next workspace (Ctrl+Tab)`. Choosing `Park all` turns
every row's dot grey and every shown slot to the stopped panel at once, with no
confirmation; `Start all` turns the parked rows purple, then blue and green one
at a time as the queue brings them back.

```
▾ Party A                                [⋯]
    ● Main         ┌──────────────────┐
    ● Alt 1        │ Park all         │
    ○ Alt 2        │ Start all        │
                   ├──────────────────┤
▾ Ungrouped   [⋯]  │ Rename…          │
    ● Solo         │ Remove workspace │
                   └──────────────────┘
                     Ungrouped's menu: the first section only
```

## Design rules

- Rule 2 — a park/start control is shown but insensitive when its direction
  does not apply; here the two items grey out rather than vanish
- Rule 5 — the deliberate action sits first in the ⋯ menu, above the
  set-and-forget items; `Park all` / `Start all` lead, `Rename…` follows
- Rule 4 — a parked slot shows the plain centred panel; `Park all` produces it
  in every slot at once
- Rule 13 — the heading stays undecorated: no dot, no bold, no mark; the
  tooltip is the only thing it gains
- Rule 1 — the rows' dots keep the six-key vocabulary; `Start all` walks
  parked → queued → starting → live through those keys and no other
