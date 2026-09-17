# Game place on Windows

## Purpose

What one place in the grid looks like on Windows, where the game view is a
native window that nothing GTK draws can sit on top of.

## Where it sits

Renders the grip-strip, dragging and zoom `**Flow**` bullets, the Loading and
stopped-process `**States**` bullets, the per-place grip strip
`**New pattern**`, and the "Rearranging a place on Windows" and "Zoom
acknowledgement on Windows" interaction diagrams in `## User Experience`.

## The screen

Each place holding a live game is a thin strip with the drag grip at its right
end, and the game view fills everything below the strip. The strip is on the
window's background, no taller than the grip plus its existing margins, and
shows nothing else. The grip follows the visibility rule it already has on
Linux. When a zoom gesture lands, the percentage appears low and centred over
the game in a small popup on its own opaque ground, then fades.

Three variants of the same place:

- **Loading.** Until the page first paints, the game view is kept at zero size
  and the cover shows the account's name centred where the game will be.
- **Dragging.** From the moment a grip is picked up until it is dropped or the
  drag is cancelled, every live game view in the grid is shrunk to nothing.
  The drop highlight and the slot lines show as on Linux.
- **Stopped, parked or queued.** There is no game view, so the place shows the
  existing absent-game panel exactly as on Linux, with no strip.

```
Live                                  Dragging (views collapsed)
┌──────────────────────────── ⠿ ┐      ┌──────────────────────────────┐
│┌────────────────────────────┐ │      │╔════════════════════════════╗│
││                            │ │      │║                            ║│
││        game page           │ │      │║     drop highlight         ║│
││                            │ │      │║                            ║│
││          ┌──────┐          │ │      │╚════════════════════════════╝│
││          │ 125% │ ← popup  │ │      └──────────────────────────────┘
│└──────────┴──────┴──────────┘ │
└───────────────────────────────┘
```

## Design rules

- Rule 4 — a place with no live game shows the plain centred panel with name,
  one state line and one button; the Windows build reuses it unchanged
- Rule 10 (its overlay clause) — Linux draws per-place controls as overlays
  that take no layout. The Windows strip is the one exception, because nothing
  can sit on top of the game view there, and the design doc owes a rule for it
- Rule 10 — the zoom figure is transient, low, centred and on its own opaque
  ground, a single figure that is re-armed rather than queued, and never shown
  for a change the user did not make; only its host (a popup) differs
