# Phone page — the game

## Purpose

Where the phone user watches and taps one game, full screen.

## Where it sits

Renders the "Tap and scroll the game", "Leave" and "Turn on mobile mode from
the phone" `**Flow**` bullets, the "Current account parked, queued or
starting", "Active workspace holds no accounts" and "Connecting or
reconnecting" `**States**` bullets, and the "Watching and tapping a game"
interaction diagram in `## User Experience`.

## The screen

The whole screen is a canvas showing the latest frame, scaled to fit with the
frame's aspect kept and black bars where the phone's shape differs. A
translucent round handle sits in the top-left corner; tapping it opens the
account list. Touches on the canvas are taps or drags and are sent to the
desktop; nothing is drawn locally. When the current account has no live page
the canvas is replaced by a plain centred panel: the account's name, one state
word (`Parked`, `Starting`, `Queued`) and a `Start` button, insensitive while
starting or queued. When the shown workspace holds no accounts the panel is
the single line `No games in this workspace`. While reconnecting the last
frame stays, dimmed, under a one-line `Reconnecting…` label, and touches are
not sent.

```
┌──────────────────────┐   ┌──────────────────────┐
│ (≡)                  │   │ (≡)                  │
│                      │   │                      │
│   game page frame    │   │      Melvor — alt    │
│   filling the        │   │        Parked        │
│   screen             │   │       [ Start ]      │
│                      │   │                      │
│                      │   │                      │
└──────────────────────┘   └──────────────────────┘
```

## Design rules

- Rule 4 — the absent-game panel is the name, one state line and one button,
  centred on the background
- Rule 2 — the button reads `Start` and is insensitive while starting or
  queued, the same inversion rule the desktop row follows
- Rule 1 — the state word is one of the sidebar's own liveness words, nothing
  new
- New pattern — a full-screen picture with a top-left handle; the design doc
  owes a section on what carries to the served page once this ships
