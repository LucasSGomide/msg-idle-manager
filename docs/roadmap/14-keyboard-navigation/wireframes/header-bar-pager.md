# Header-bar pager

## Purpose

Where the owner sees which page of the shown workspace is on screen and turns
to the next or previous one with the mouse.

## Where it sits

Renders the pager `**Entry**` bullet, the "Turn a page" and "Change layout"
`**Flow**` bullets, the "One page" `**States**` bullet, and the "Turning the
page" interaction diagram.

## The screen

The existing header bar, unchanged except for one linked box directly left of
the `1` `2` `4` `Phone` layout toggles: a `‹` icon button, an `n/m` readout in
tabular figures, and a `›` icon button. The box is present only while the shown
workspace has more than one page; with one page the toggles sit where they do
today and nothing hints a pager exists. Hovering the box reads
`Next account (Shift+Tab)`; hovering an arrow reads `Previous page` /
`Next page`. Clicking an arrow changes the whole grid to the neighbouring page,
wrapping at either end, and focuses the new page's first slot.

```
┌────────────────────────────────────────────────────────────────────────┐
│ [⟳]                          [‹] 2/3 [›] [1][2][4][Phone] [☰] [▤] [Add game] │
└────────────────────────────────────────────────────────────────────────┘
   ▲ reload                     ▲ pager: hidden when m == 1
```

## Design rules

- Rule 11 — the readout is a fixed-width numeric figure so a digit changing
  never shifts its neighbour
- Rule 13 — a page turn re-keys the sidebar rows; no heading gains a mark of
  the page
- Rule 10 — the pager is an action, not an acknowledgement: no transient
  figure flashes over the grid on a turn
