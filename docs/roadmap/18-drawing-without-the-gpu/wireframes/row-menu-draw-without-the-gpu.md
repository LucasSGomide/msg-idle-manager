# Row ⋯ menu with the Draw without the GPU check

## Purpose

Where the owner turns GPU drawing off or back on for one account, and the only
place that says which way that account is drawn.

## Where it sits

Renders the `**Entry**` bullet (the row's ⋯ menu, opened by its button, a
right-click or `Shift`+`F10`; Linux build only) and both interaction diagrams in
`## User Experience` — "Turning GPU drawing off for a live account" and
"Turning it on for an account with no page" — which open the same menu on a
live and on a parked row. Covers the `**States**` bullets: an unchanged value
does nothing, and the Windows build shows no item.

## The screen

The sidebar's existing account row, unchanged: the leading liveness dot, the
name, and the ⋯ button on the trailing edge, revealed on hover or selection. Its
popover menu is the one `bind_row_menu` already builds, with one more item at
the end of the first section: `Park` (or `Start`), `Keep running when hidden`,
`Muted`, then `Draw without the GPU`, each of the last three with its check.
The new item carries no accelerator on any row. Ticking it on a live row closes
the menu and the row's dot shows the reloading mark until the page's first
paint, then returns to live — the same thing changing keep-awake shows. On a
parked row it is offered the same way, reads the stored flag, and the row does
not change. The Windows build's menu ends the first section at `Muted`.

```
  ● Lux Idle — EK / ED                         ⋯
    ┌───────────────────────────────────┐
    │ Park                      Ctrl+P  │   ← chord only on the focused row
    │ ☐ Keep running when hidden        │
    │ ☐ Muted                           │
    │ ☑ Draw without the GPU            │   ← new; Linux only; no accelerator
    ├───────────────────────────────────┤
    │ Move to                         ▸ │
    │ Rename…                       F2  │
    ├───────────────────────────────────┤
    │ Delete account…                   │
    └───────────────────────────────────┘
```

## Design rules

- Rule 1 — the row's state is one leading mark; the flag adds no mark, and the
  only visible change is the existing reloading state while the page rebuilds
- Rule 2 — the first item stays the single Park/Start control; the new check
  sits beneath it with the other settings
- Rule 5 — set-and-forget settings live behind the row's one ⋯ menu button,
  action first, settings below
- Rule 6 — the trailing edge carries nothing new, so the 200 px width check is
  not reopened
- Rule 21 — one menu, reached from the ⋯ button, a right-click and
  `Shift`+`F10`; no second path to the setting
- Rule 27 — no key is bound, so no accelerator is shown on any row
