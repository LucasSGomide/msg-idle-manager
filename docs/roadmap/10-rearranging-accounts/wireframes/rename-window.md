# Rename window

## Purpose

The one field where the user types an account's new name and confirms it.

## Where it sits

Renders the rename `**Flow**` bullets (open with the current name selected; type,
Enter confirms, Escape cancels; confirm closes it and the name updates) and the
"Renaming an account" interaction diagram in `## User Experience`. Covers the
`**States**` bullet's empty name (`Rename` insensitive, no error text) and same
name (accepted, nothing visible changes).

## The screen

A small modal window titled "Rename account", transient for the main window and
not resizable, the same width and margins as the add-game dialog. It holds one
text field pre-filled with the current name, with all of it selected. Below it,
at the trailing edge, `Cancel` and then `Rename`, with `Rename` as the default
button. While the trimmed text is empty, `Rename` is greyed out and nothing else
changes: no error line, no red field. On confirm the window closes, and the
sidebar row and that account's place show the new name, keeping whatever bold or
dimming the row already had.

```
┌ Rename account ──────────────────┐
│                                  │
│  [ Main account█               ] │
│                                  │
│               [Cancel] [Rename]  │
└──────────────────────────────────┘

empty field:    [Cancel] [Rename]   ← Rename insensitive
```

## Design rules

- Rule 5 — reached only from the row's ⋯ menu, never from a control on the row
- Rule 3 — the renamed row keeps its existing style: bold only for the current
  row, 55% dim when out of sight or parked
- Rule 4 — a parked or queued account's place panel shows the new name through
  its name property, with no restyle
- Rule 8 — nearest form rule: nothing blocks the working path. An empty name
  greys the button rather than raising an error
