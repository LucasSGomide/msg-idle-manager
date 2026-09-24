# Main menu: version and Check for updates

## Purpose

Where the user reads which version is running and starts an update check by
hand.

## Where it sits

Renders the `**Entry**` bullet's manual route and the "Checking by hand from
the main menu" interaction diagram in `## User Experience`, plus the
`**Pattern**` bullet on the insensitive version item.

## The screen

The header bar's existing `☰` main menu gains one section, placed after the
`Phone…` section and before `Keyboard Shortcuts`. Two items: `Idle Manager
0.2.0`, drawn insensitive because it has no action, naming the version actually
installed; and `Check for updates`, which closes the menu and opens the update
notice in its `Checking for updates…` state. No accelerator on either item, and
neither appears in the shortcuts window.

```
┌ ☰ ──────────────────────────┐
│ Melvor · Main               │
│   Zoom in           Ctrl++  │
│   Zoom out          Ctrl+-  │
│   Reset zoom        Ctrl+0  │
├─────────────────────────────┤
│ Phone…                      │
├─────────────────────────────┤
│ Idle Manager 0.2.0          │  (insensitive)
│ Check for updates           │
├─────────────────────────────┤
│ Keyboard Shortcuts   Ctrl+? │
└─────────────────────────────┘
```

## Design rules

- Rule 23 — an item with no bound action is drawn insensitive by the popover
  menu itself, the device used for the row's own workspace in `Move to ▸`
- Rule 19 (by contrast) — no accelerator text, because no key answers this
  item and it is not listed in the shortcuts window
- Rule 28 — the version line grows with its text; nothing fixed in width
