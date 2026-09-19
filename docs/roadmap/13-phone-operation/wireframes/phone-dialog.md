# Phone dialog

## Purpose

Where the desktop user enrols the one phone by showing it a code, sees whether
a phone is enrolled or connected, and un-enrols it.

## Where it sits

Renders the "Enrol" and "Un-enrol" `**Flow**` bullets, the "Desktop phone
dialog, not listening" `**States**` bullet, and the "Enrolling the phone"
interaction diagram in `## User Experience`.

## The screen

A small modal window titled `Phone`. Top: one status line, one of
`No phone enrolled`, `Phone enrolled`, `Phone connected`, or, in dim text,
`Not listening: no mesh network address found`. Middle: while a code is live, a
QR picture, the same address as a selectable single line beneath it, and a
countdown `Valid for 9:41`; otherwise this block is empty. When the countdown
reaches zero the block empties and the status line reads `The code expired;
start again`. Bottom, right-aligned: `Enrol…` (insensitive while not listening
or while a code is live) and `Un-enrol` styled destructive (insensitive while
no phone is enrolled). The status line is polled once a second while the
dialog is open, so scanning flips it to `Phone enrolled` without a click.

```
┌ Phone ─────────────────────────────────┐
│ No phone enrolled                      │
│                                        │
│          ▛▀▀▀▀▀▀▀▀▀▀▀▀▜                │
│          ▌ ▄▄ ▄ ▀▄ ▄▄ ▐   QR code      │
│          ▌ █▄ ▀▄▄▀ ▄█ ▐                │
│          ▙▄▄▄▄▄▄▄▄▄▄▄▄▟                │
│  http://100.101.12.7:7466/enrol/3fa9…  │
│  Valid for 9:41                        │
│                                        │
│                  [ Enrol… ] [Un-enrol] │
└────────────────────────────────────────┘
```

## Design rules

- Rule 8 — the not-listening reason is one dim line inside the form, and the
  form stays usable; never a modal of its own
- Rule 9 (by contrast) — nothing here goes in the window's message strip: the
  user asked for this dialog, so its problems are shown inside it
- New pattern — a pairing screen showing a QR code and the same address as
  text for a fixed time; the design doc owes a rule once this ships
