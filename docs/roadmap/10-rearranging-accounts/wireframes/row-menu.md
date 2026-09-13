# Row menu

## Purpose

Where the user reaches `Rename…` for one account, beside that account's existing
Park/Start action and keep-awake setting.

## Where it sits

Renders the rename `**Entry**` bullet ("a new `Rename…` item in an account row's
existing ⋯ menu") and the first step of the "Renaming an account" interaction
diagram in `## User Experience`. Covers the rename `**States**` bullet's
parked, starting and queued accounts, where the item is still offered.

## The screen

The sidebar row is unchanged: name, status dot, keep-awake mark, ⋯ button. The
⋯ button opens the same popover menu it opens today. Park/Start comes first,
then "Keep running when hidden" with its tick, then `Rename…` last. `Rename…` is
never greyed out: a starting account's Park/Start item is insensitive, but its
`Rename…` item is not. The row's trailing edge gains nothing.

```
Main account  ●  ☾  [⋯]
                  ┌──────────────────────────┐
                  │ Park                     │
                  │ ✓ Keep running when hidden│
                  │ Rename…                  │
                  └──────────────────────────┘
```

## Design rules

- Rule 5 — settings and the Park/Start action live behind the one ⋯ menu button,
  action first and settings below it, so `Rename…` goes below
- Rule 2 — Park/Start stays the single invert-with-state item and is the only one
  greyed out while starting
- Rule 6 — the trailing edge gains no widget, so the sidebar width needs no
  re-derivation
