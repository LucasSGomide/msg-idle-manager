# A failed account

## Purpose

What an account looks like while it is being brought back automatically, and
what it looks like once the application has given up.

## Where it sits

Renders the `**Flow**` and `**States**` bullets in `## User Experience` and the
"A game crashes and comes back" interaction diagram. The second diagram in that
section is the policy, not a screen. Covers the healthy, reconnecting, failed
and recovered cases.

## The screen

Two surfaces, both already built.

**The slot** uses item 03's placeholder panel with different content. While
reconnecting it shows the account's name, a line giving the attempt number and
that another attempt is coming, and no button — pressing something during a
one-second wait helps nobody. Once the application has stopped, the same panel
shows the account's name, a line saying the game stopped responding and how many
attempts were made, and a "Try again" button.

**The sidebar row** shows the same two states on its trailing edge, and the
failed state outranks everything else the row could say. An account that is
parked, out of sight and failed reads as failed; the other facts are still true
and are no longer the useful ones. Recovery removes the marker with no
acknowledgement — nothing to dismiss, nothing to notice in the morning except a
game that is running.

An account that is out of sight when it fails changes only its row. Nothing
appears over the games the user is actually looking at.

```
+------------------+-----------------------------------+
| Main account   1 |                                   |
| Farm 1    failed |            Main account           |
| Farm 2  retry 2… |                                   |
|                  |===================================|
|                  |            Farm 1                 |
|                  |   Stopped responding after 4 tries|
|                  |          [ Try again ]            |
+------------------+-----------------------------------+
```

## Design rules

- **Reuses** item 03's slot placeholder rather than introducing a panel. That
  widget was built with its text and its action as properties for exactly this,
  which is recorded in item 03's `wireframes/parked-account.md`.
- `docs/design.md` has no numbered rules yet. The debt this screen adds is
  precedence: a row can now be parked, out of sight, kept awake and failed at
  once, and the rule for which of those it says is invented here.
- `docs/architecture.md` rule 8 — "Try again" emits an intent; the domain resets
  the attempt count and the shell then rebuilds the view.
- `docs/architecture.md` rule 9 — the delays are the domain's, taking elapsed
  time as an argument, so the sequence this screen displays is the same one the
  unit tests assert.
- `docs/code-standards.md` rule 5 — the delay sequence is a named constant with
  its unit, `RESTART_DELAYS_SECS`, and the attempt count shown here is read from
  it rather than written twice.
