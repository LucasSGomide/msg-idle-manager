# Phone page — mobile mode off

## Purpose

What the enrolled phone shows while the desktop is in an ordinary layout:
nothing but the way to turn mobile mode on.

## Where it sits

Renders the "Mobile mode off, phone connected" `**States**` bullet and the
"Turn on mobile mode from the phone" `**Flow**` bullet in
`## User Experience`. Also covers the "Refused" and "Connecting or
reconnecting" `**States**` as variants of this screen when no game is showing.

## The screen

A full-screen page in the window background colour with one large centred
switch labelled `Mobile mode`, off, and one dim line beneath it, `Turn it on to
see a game here`. Nothing else: no list handle, no account name. Tapping the
switch sends `mobile {on: true}`; the desktop's reply state flips the switch
and the page becomes the game screen. Variants: while connecting the switch is
insensitive and the line reads `Connecting…`; after un-enrolment the switch is
gone and the only line is `This phone is no longer enrolled`.

```
┌──────────────────────┐
│                      │
│                      │
│     Mobile mode      │
│        ( ○  )        │
│  Turn it on to see   │
│    a game here       │
│                      │
│                      │
└──────────────────────┘
```

## Design rules

- Rule 4 — a plain centred stack of at most three elements on the background,
  never a busy card
- Rule 8 — the refused and connecting lines are one dim line in place, with
  no dialog
