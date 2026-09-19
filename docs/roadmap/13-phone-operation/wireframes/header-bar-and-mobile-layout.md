# Header bar and the Mobile layout

## Purpose

Where the desktop user switches mobile mode on and off, reaches the phone
dialog, and sees the one phone-shaped slot the mode draws.

## Where it sits

Renders the `**Entry**` bullet (the `Phone` toggle and the header menu), the
"Desktop, `Mobile` layout" `**States**` bullet, and the "Entering and leaving
mobile mode on the desktop" interaction diagram in `## User Experience`.

## The screen

The existing header bar with one change on each side of the layout group: the
linked `1` `2` `4` toggles gain a fourth member labelled `Phone`, and a
`open-menu-symbolic` menu button to their right opens a two-item menu,
`Enrol a phone…` and `Un-enrol the phone` (the second greyed while no phone is
enrolled). With `Phone` active the grid area shows a single slot the size of
the phone's viewport (412 × 915 by default), centred horizontally and
top-aligned, its outline drawn in the slot-line colour, the rest of the grid
the window background. A window shorter than the slot clips its bottom. No
grip and no grip strip appear. The sidebar and the memory footer are
unchanged.

```
┌ ⟳ ─────────────────────────── [1][2][4][Phone] ☰ [Add game] ▤ ┐
│ sidebar    │          ┌──────────────┐                        │
│ ▸ Party    │          │              │                        │
│   ● Melvor │          │  game page   │                        │
│   ● Kitten │          │  at 412×915  │                        │
│ ▾ Ungroup… │          │              │                        │
│   ● Alt    │          │              │                        │
│            │          └──────────────┘ (clipped if too short) │
│ footer     │                                                  │
└────────────┴──────────────────────────────────────────────────┘
```

## Design rules

- Rule 1 — the sidebar dots keep their six-key vocabulary; the mode adds no
  state and no marker
- Rule 6 — the sidebar's width is untouched; nothing new lands on a row's
  trailing edge
- Rule 10 — no zoom readout is ever flashed in `Mobile`, because zoom is
  locked there and no gesture changes anything
- Rule 14 — on Windows the slot outline is drawn by the grid around the native
  child window, never over it; no overlay is added
- New pattern — a fixed-size, clipped slot that does not fill its share of
  the window; the design doc owes a rule once this ships
