# Add-game workspace field

## Purpose

The one change to the add-game window: choosing which workspace a new account
goes into.

## Where it sits

Renders the add-game `**Entry**` bullet ("the add-game window's details form
gains a `Workspace` field") and the adding `**Flow**` bullet.

## The screen

The existing second stage of the add-game window, for either a catalogue game or
the "Something else…" path, gains a `Workspace` drop-down below the fields it
already has and above its action buttons. The list holds every named workspace
with room for one more account in sidebar order, then `Ungrouped`. It defaults to
the shown workspace when that workspace has room, and to `Ungrouped` otherwise.
Confirming into a workspace that is not shown creates the account there, running
out of sight; the window does not switch.

```
┌ Add game ─────────────────────────────┐
│ Account name                          │
│ [ Alt 3                             ] │
│                                       │
│ Workspace                             │
│ [ Party A                         ▾ ] │   rooms-left list + Ungrouped
│                                       │
│                    [Back] [  Add  ]   │
└───────────────────────────────────────┘
```

## Design rules

- Rule 7 — the first stage's "Something else…" escape hatch is unchanged; the
  field is added to both paths' details form alike
- Rule 8 — the form's partial-failure line and its working path are untouched;
  a full workspace is left out of the list rather than shown with an error
