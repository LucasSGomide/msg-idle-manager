# The zoom readout over a place

## Purpose

What the user sees when a zoom gesture lands: the game redrawn at its new size,
and a percentage figure over that place which fades on its own. Also what they
see when the size changes without a gesture — an arrangement switch — where the
figure deliberately does not appear.

## Where it sits

Renders the "Zooming the account in the active place" and "Switching arrangement
snaps every account to its remembered size" interaction diagrams in
`## User Experience`, and the `**Entry**` bullet's two gestures. Covers all four
cases from the `**States**` bullet — nothing on screen, at the limit, a parked
account in the active place, and an account out of sight — plus the
`**Pattern**` bullet placing the figure on the same per-place stack that already
carries the name cover and the parked panel.

## The screen

The main window, unchanged in every structural respect: the sidebar, the header
bar and the arrangement buttons are untouched, and no new control appears
anywhere. The only new thing drawn is a small percentage figure over the place
whose game just changed size — a short label on its own opaque ground, centred
horizontally and sitting low in the place rather than over the middle of the
game, so it never covers what the reader is looking at while they are adjusting
it. It appears the instant the gesture lands and fades out after about a second,
leaving the place exactly as it was. A run of gestures shows one figure that
keeps updating, never a queue of them.

The figure is drawn over whatever the place is showing, because the place is
what the gesture affected. Over a live game that is the page itself. Over a
parked account it is design rule 4's plain centred panel, which is not restyled
and not moved — the figure sits above it and the panel keeps its three stacked
elements. Both are the same layer in the same per-place stack; there is no
second widget for the parked case.

Three variants of the same screen, and one non-appearance:

- **A live game.** The page resizes in place and the figure reads the new
  percentage. Nothing reloads and nothing else on screen moves.
- **At the limit.** The page does not change size, because the step was clamped,
  but the figure still appears reading the unchanged percentage. That is the
  whole point of drawing it here: a gesture that changes nothing must still look
  received rather than lost.
- **A parked account in the active place.** There is no page to resize, so only
  the figure changes — over the parked panel, which keeps saying the account is
  parked. The size it names is what that account will open at when it is next
  started.
- **An arrangement switch.** Every game on screen is redrawn at its own
  remembered size for the new arrangement, and no figure appears over any of
  them. The absence is the design: nobody asked for a size change, so nothing
  acknowledges one.

An empty grid draws none of this — with no place filled there is nothing for a
gesture to act on and nothing to draw a figure over.

```
+----------------+---------------------+---------------------+
| Main account   |                     |                     |
| Farm 1         |     (game page)     |     (game page)     |
| Farm 2         |                     |                     |
|                |        [110%]       |                     |
|                +---------------------+---------------------+
|                |                     |       Farm 2        |
|                |     (game page)     |       Parked        |
|                |                     |      [ Start ]      |
|                |                     |        [90%]        |
+----------------+---------------------+---------------------+
```

Two places carry a figure here only to show both variants at once; in use one
gesture affects one place.

## Design rules

- Rule 4 — the panel standing in for an absent game stays a plain centred panel
  of exactly three stacked elements on the window's background. The figure is
  drawn over it, never folded into it, so the panel gains no fourth element and
  item 08's failure panel inherits nothing new.
- Rule 6 — a fact drawn over a place never takes width from anything: the figure
  is an overlay, so it costs no layout and the sidebar's derived width is
  untouched by this item.
- No numbered rule covers a transient acknowledgement drawn over live content.
  This screen is the item's `**New pattern**` bullet, and `docs/design.md` owes a
  rule once it ships — where the figure sits, how long it lasts, and why an
  arrangement switch shows none.
