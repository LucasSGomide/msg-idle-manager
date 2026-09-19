# Phone page — the account list

## Purpose

Where the phone user picks another account, parks or starts one, and turns
mobile mode off.

## Where it sits

Renders the "Switch account", "Park or start from the phone" and "Turn mobile
mode off from either end" `**Flow**` bullets and the "Switching account from
the phone" interaction diagram in `## User Experience`.

## The screen

A panel that slides in from the left over the game and covers most of the
width. It lists every workspace as a plain heading, each followed by its
accounts: the name on the left, the liveness word in small dim text beneath
it, and on the right one button reading `Park` while the account runs and
`Start` otherwise, insensitive while starting or queued. The current account's
row is highlighted. Tapping a name sends `choose` and the panel slides away.
Headings carry no dot and no mark. At the bottom a `Mobile mode` switch, on;
turning it off sends `mobile {on: false}` and the page returns to the toggle
screen. A workspace with no accounts shows one dim line under its heading,
`No accounts`. Opening the panel does not itself count as leaving while an
account is chosen.

```
┌──────────────────┬───┐
│ Party            │   │
│  Melvor — main   │   │
│  live     [Park] │ g │
│ ▸Melvor — alt    │ a │
│  parked  [Start] │ m │
│ Ungrouped        │ e │
│  Kittens         │   │
│  starting [Start]│   │
│                  │   │
│ Mobile mode (●  )│   │
└──────────────────┴───┘
```

## Design rules

- Rule 1 — each row's state is one of the six-key vocabulary's liveness
  words; no new state is invented for the phone
- Rule 2 — one Park/Start control per row whose label and effect invert with
  liveness, insensitive while starting or queued
- Rule 3 — a parked, queued or out-of-sight account's name is dimmed; only the
  current row is emphasised
- Rule 13 — a workspace heading carries no dot, no bold and no mark of which
  workspace is shown
