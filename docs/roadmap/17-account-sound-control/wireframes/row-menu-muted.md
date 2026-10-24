# Row ⋯ menu with the Muted check

## Purpose

Where the owner mutes or unmutes one account, and the only place that says
whether it is muted.

## Where it sits

Renders the `**Entry**` bullet (the row's ⋯ menu, opened by its button, a
right-click or `Shift`+`F10`) and both interaction diagrams in
`## User Experience` — "Muting a live account from its row menu" and "Muting an
account with no page, and starting it later" — which open the same menu on a
live and on a parked row. Covers the `**States**` bullets: an unchanged value
does nothing, an engine refusal shows nothing, and the row's mark never moves.

## The screen

The sidebar's existing account row, unchanged: the leading liveness dot, the
name, and the ⋯ button on the trailing edge, revealed on hover or selection. Its
popover menu is the one `bind_row_menu` already builds, with one more item in
the first section. The section reads `Park` (or `Start`, inverting with
liveness), then `Keep running when hidden` with its check, then `Muted` with its
check. `Muted` carries no accelerator on any row, focused or not. The second
section (`Move to ▸`, `Rename…`) and the third (`Delete account…`) are as they
are today. Ticking `Muted` closes the menu; the row behind it does not change in
any way — no new mark, no reloading state, the dot keeps its colour. Reopening
the menu shows the check ticked. On a parked row the item is offered exactly the
same way and reads the stored flag. Nothing on the row or in the window says
"muted" apart from this check.

```
  ● Melvor                                    ⋯
    ┌───────────────────────────────────┐
    │ Park                      Ctrl+P  │   ← chord only on the focused row
    │ ☐ Keep running when hidden        │
    │ ☑ Muted                           │   ← new; no accelerator
    ├───────────────────────────────────┤
    │ Move to                         ▸ │
    │ Rename…                       F2  │
    ├───────────────────────────────────┤
    │ Delete account…                   │
    └───────────────────────────────────┘
```

## Design rules

- Rule 1 — the row's state is one leading mark and nothing else visible; mute
  adds no second mark and never changes the mark's status key
- Rule 2 — the first item stays the single Park/Start control that inverts with
  liveness; `Muted` sits beneath it, not beside it
- Rule 5 — set-and-forget settings live behind the row's one ⋯ menu button, the
  action first and the settings below it
- Rule 6 — the trailing edge carries nothing new, so the sidebar's 200 px width
  check is not reopened
- Rule 21 — the same menu opens from the ⋯ button, a right-click and
  `Shift`+`F10`; `Muted` is in that one menu, never a second path
- Rule 27 — an accelerator is shown only where the key would act; `Muted` has no
  key, so it shows none on any row
