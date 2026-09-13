# Grid during a drag

## Purpose

How the user picks an account up by its grip and drops it on another place, and
what the grid shows while they do.

## Where it sits

Renders the rearrange `**Entry**` bullet (grip in an occupied place's top-right
corner, hover only, never in the one-place arrangement), the rearrange `**Flow**`
bullets, and the "Dragging an account to another place" interaction diagram in
`## User Experience`. Covers the `**States**` bullet's parked or queued panel as
a valid drop, a drag over a game's page receiving nothing, and no grip in `Single`
or on an empty grid.

## The screen

**At rest.** The grid is unchanged: games in their places, the focused place
outlined as today. No grip is visible anywhere.

**Hover.** With the pointer anywhere inside an occupied place, including over the
game's page or a parked panel, a small drag-handle icon appears in that place's
top-right corner, inset by a small margin. It disappears when the pointer leaves
the place. Only the icon takes clicks; the rest of that corner still reaches the
game. In the one-place arrangement no grip ever appears.

**Dragging.** Pressing the grip leaves focus where it was, and the grip hides
once the drag begins. Once the pointer moves past the
drag threshold, a small rounded chip with the account's name follows it. The
place under the pointer, including the source place, is covered by a translucent
tint, and the tint follows the pointer from place to place. Empty places take the
tint too. The game pages keep running underneath and show no drop cursor.

**After the drop.** On another occupied place the two accounts trade places; on
an empty one the account moves in and its old place is left empty. The focus
outline stays with the account that had it, in whatever place that account now
sits, and the sidebar rows reorder to match. A drop on the source place, outside the grid, or Escape leaves everything
as it was.

```
Hover (Grid layout, pointer in top-left place)

┌────────────────────┬────────────────────┐
│               [⠿]  │                    │
│   Main account     │   Alt one          │
│   (live page)      │   (live page)      │
├────────────────────┼────────────────────┤
│                    │                    │
│   Alt two          │       Alt three    │
│   (live page)      │       Parked       │
│                    │       [Start]      │
└────────────────────┴────────────────────┘

Dragging "Main account" over the bottom-right place

┌────────────────────┬────────────────────┐
│               [⠿]  │                    │
│   Main account     │   Alt one          │
│                    │                    │
├────────────────────┼────────────────────┤
│                    │░░░░░░░░░░░░░░░░░░░░│
│   Alt two          │░░░░ Alt three ░░░░░│
│                    │░░░(Main account)░░░│  ← name chip at pointer
│                    │░░░░░░░░░░░░░░░░░░░░│
└────────────────────┴────────────────────┘
```

## Design rules

- Rule 4 — a parked or queued account's plain panel moves with its account and is
  not restyled; the grip sits over it as it sits over a page
- Rule 10 — the only existing shape for something drawn over live content, and
  the grip is deliberately not it: the grip takes input and shows on hover, not on
  a gesture. That gap is the item's `**New pattern**` debt
- Rule 1 — the sidebar's current-row marker (glowing dot, bold name) stays on the
  account that was current before the drag; grabbing or dropping never moves it
