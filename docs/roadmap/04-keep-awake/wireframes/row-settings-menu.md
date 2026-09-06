# The row settings menu

## Purpose

Where a per-account setting lives that is decided once and then forgotten,
without spending room on the row itself.

## Where it sits

Renders the `**Entry**` and `**Flow**` bullets in `## User Experience` and the
"Turning keep-awake on for an account" interaction diagram. Covers the off, on
and reloading cases from the `**States**` bullet. The second diagram in that
section is behaviour, not a screen, and is not drawn here.

## The screen

The sidebar row gains a menu button on its trailing edge, after item 03's action
button, drawn as the conventional three-dot affordance and not labelled. It
opens a small menu with one item in this release: a checkable "Keep running when
hidden".

Choosing it closes the menu, and the row's place marker changes to say the
account is reloading until its page paints again — the same treatment item 03
gives a starting account, because it is the same situation from the user's point
of view. There is no confirmation before the reload and no dialog explaining it;
the reloading state is the explanation, and the setting is reversible.

Once on, the row's trailing edge carries a small persistent indication so the
setting is readable without opening the menu. That edge now holds three things
at once — the place, the state, and this — which is the point at which the row's
trailing area needs a rule rather than an arrangement.

```
| Main account      1 · running     [Park] [⋮] |
| Farm 1       away · awake         [Park] [⋮] |
                                          |
                        +----------------------------+
                        | [x] Keep running when hidden|
                        +----------------------------+
```

## Design rules

- `docs/design.md` has no numbered rules yet. This screen creates the debt the
  item's `**New pattern**` bullets name: where a row's settings live as against
  its actions, and how three separate facts share one trailing edge without the
  row becoming unreadable at the sidebar's fixed width.
- `docs/architecture.md` rule 8 — the menu item emits an intent; the reload is
  the shell's response to what the domain returned, not the menu's doing.
- `docs/architecture.md` rules 12 and 13 — the menu is a `gio::Menu` described
  in the sidebar's existing template.
- `docs/naming.md` rule 12 — the underlying flag is named as a question,
  `is_kept_awake`; the menu's wording is the user's, not the field's.
