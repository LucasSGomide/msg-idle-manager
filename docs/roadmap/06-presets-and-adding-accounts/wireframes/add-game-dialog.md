# Add-game dialog, with the catalogue

## Purpose

The same dialog item 01 built, reshaped so the common case is choosing a known
game and the uncommon case is still possible.

## Where it sits

Renders the `**Entry**`, `**Flow**` and `**States**` bullets in
`## User Experience` and the "Adding an account from the catalogue" interaction
diagram. Replaces item 01's `wireframes/add-game-dialog.md`; that file stays
where it is as the record of what this screen was.

## The screen

The same modal over the main window, now in two stages inside one dialog rather
than one form.

**Stage one** is a list of games, one row each, showing the game's display name
and nothing else — the address, the browser identity and the zoom are the
application's business, not something to preview. The list is sorted by display
name. The final row is separated from the rest by a hairline and reads
"Something else", so it is visibly not a game.

If a file in the folder could not be read, a single line sits under the list
naming that file and what was wrong with it. The list still shows everything
that did parse. If nothing parsed and there are no files at all, the list area
holds one line saying no games are configured and names the folder to put them
in — with "Something else" still present, so the dialog is never a dead end.

**Stage two** depends on the choice. For a game, it is one field: a name for
this account, so two accounts on the same game can be told apart. For "Something
else", it is item 01's two fields unchanged, name and address.

The action row is constant across both stages: cancel, and a suggested action
that reads "Add". It is insensitive until the stage's fields are filled.

```
+------------------------------------+
|  Add a game                        |
|  +------------------------------+  |
|  | Melvor Idle                  |  |
|  | Cookie Clicker               |  |
|  | Universal Paperclips         |  |
|  |------------------------------|  |
|  | Something else…              |  |
|  +------------------------------+  |
|  ! kittens-game.toml: missing url  |
|                                    |
|  Name for this account             |
|  [ Alt                          ]  |
|                                    |
|                  [ Cancel ] [ Add ]|
+------------------------------------+
```

## Design rules

- `docs/design.md` has no numbered rules yet. This screen owes two: how an
  escape-hatch option is separated from real options inside a chooser, and how a
  partial failure is reported inside a form without blocking it.
- Item 01's `wireframes/add-game-dialog.md` already recorded the keyboard and
  suggested-action debts; they are unchanged and inherited here.
- `docs/architecture.md` rule 3 — the dialog receives the catalogue through a
  trait and never learns that the entries are files on disk.
- `docs/architecture.md` rules 12 and 13, `docs/naming.md` rules 1 and 4 — the
  reshaped `add-game-dialog.ui` beside `add_game_dialog.rs`; the preset files
  themselves are kebab-case, `presets/melvor-idle.toml`.
