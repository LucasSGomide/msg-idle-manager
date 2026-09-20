# Shortcuts window

## Purpose

Where the owner learns every key the window understands without reading
documentation.

## Where it sits

Renders the `Keyboard Shortcuts` `**Entry**` bullet, the "Learn the keys"
`**Flow**` bullet, and the "The shortcuts window" interaction diagram.

## The screen

GTK's own shortcuts window, opened over the main window by `Ctrl`+`?` or by the
`Keyboard Shortcuts` item now at the end of the ☰ menu (whose tooltip becomes
`Menu`). One section, one group titled `Idle Manager`, one row per key: Reload
the focused game, Zoom in, Zoom out, Reset zoom, Next account, Next workspace,
Keyboard shortcuts. Each row shows the key caps on the left and the action on
the right. `Esc` or the close button hides it; nothing in the workspace
changes.

```
┌ Shortcuts ──────────────────────────────── × ┐
│  Idle Manager                                │
│  [F5] / [Ctrl]+[R]     Reload the focused game│
│  [Ctrl]+[+]            Zoom in               │
│  [Ctrl]+[−]            Zoom out              │
│  [Ctrl]+[0]            Reset zoom            │
│  [Shift]+[Tab]         Next account          │
│  [Ctrl]+[Tab]          Next workspace        │
│  [Ctrl]+[?]            Keyboard shortcuts    │
└──────────────────────────────────────────────┘
```

## Design rules

- Rule 14 — on Windows this is its own native surface, so it draws above a
  `WebView2` child window like the zoom popover does; no overlay layer is used
- Rule 10 — the window is an action the user opened, not an acknowledgement;
  it takes focus and closes on `Esc`, unlike a transient readout
