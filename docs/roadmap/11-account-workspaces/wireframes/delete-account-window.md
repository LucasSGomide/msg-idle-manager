# Delete account window

## Purpose

The modal that confirms deleting an account, blocks while its data and folder
are removed, and reports a failure with a retry.

## Where it sits

Renders the delete `**Entry**` bullet (`Delete account…` in the row ⋯ menu), the
deleting and confirm `**Flow**` bullets, the "delete fails" `**States**` bullet,
and the "Deleting an account" diagram. It is one window with three pages. This
item lists the blocking-progress-then-error shape as a `**New pattern**`.

## The screen

A modal, non-resizable window titled "Delete account?", with three pages that
replace one another in place.

**Confirm.** The sentence "Delete Main? Its logins and saved game data will be
removed from this computer. This cannot be undone.", then `Cancel` and a red
`Delete account` button. Escape and `Cancel` close the window with nothing
changed.

**Working.** The buttons are gone. A spinner sits beside "Deleting Main…". The
window has no close button, and Escape does nothing.

**Failed.** "Couldn't finish deleting Main." then two dim lines: the reason, and
the folder path, which can be selected and copied. Then `Close` and `Retry`.
`Retry` returns to Working and runs the whole deletion again. `Close` leaves the
account in its workspace, parked and logged out: a grey dot on its row, and a
plain parked panel if it holds a place.

```
Confirm
┌ Delete account? ──────────────────────────┐
│ Delete Main? Its logins and saved game    │
│ data will be removed from this computer.  │
│ This cannot be undone.                    │
│                                           │
│            [Cancel] [ Delete account ]    │   red
└───────────────────────────────────────────┘

Working
┌ Delete account? ──────────────────────────┐
│                                           │
│        ◌  Deleting Main…                  │   no buttons, no close
│                                           │
└───────────────────────────────────────────┘

Failed
┌ Delete account? ──────────────────────────┐
│ Couldn't finish deleting Main.            │
│ Permission denied (os error 13)           │   dim
│ ~/.local/share/idle-manager/profiles/     │   dim, selectable
│   session-0004                            │
│                                           │
│                      [Close] [ Retry ]    │
└───────────────────────────────────────────┘
```

The Working page is usually on screen for well under a second; it exists so the
window can never be closed mid-way, not to show a long wait.

## Design rules

- Rule 5 — the entry point is the last item in the row's ⋯ menu, never a button
  on the row
- Rule 8 — the reason and the folder are dim lines inside the window that
  produced them, not a separate error dialog
- Rule 9 — deliberately not the message strip: the strip is for problems the
  user did not cause, and this failure is the result of an action they are
  watching
- Rule 1 — after `Close` the account keys as `parked`; no new state enters the
  dot vocabulary
- Rule 4 — a parked account's place shows the existing plain panel, unchanged
