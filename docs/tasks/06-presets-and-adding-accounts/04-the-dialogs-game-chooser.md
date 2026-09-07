# 04 — The add-game dialog's game chooser

**Roadmap:** [06](../../roadmap/06-presets-and-adding-accounts/README.md) · **Scope:** front-end · **Depends on:** 02

## Context

Pressing "Add game" today opens a small window asking for two things: a name for
the account and the web address the game lives at. That asks the user to know
something they should not have to — the right address, typed correctly — and it
says nothing about the three other answers each game needs.

This slice reshapes that window into two stages without adding a second window.
The first stage is a list of the games the program knows about, one row each,
showing the game's name and nothing else; the address and the rest are the
program's business, not something worth previewing. The last row is separated
from the games by a hairline and reads "Something else", so it plainly is not a
game. Choosing a game moves to the second stage, which asks for one more thing:
a name for this account, so two accounts on the same game can be told apart.
Choosing "Something else" brings back the two fields exactly as they are today,
for a game the program has never heard of. Either way the buttons along the
bottom stay put — cancel, and an "Add" that stays unpressable until the stage's
fields are filled.

The list is read fresh every time the button is pressed rather than once when
the program starts, so a file somebody drops into the folder by hand shows up on
the next press without restarting anything.

Two things can go wrong and neither one closes the window. A file that cannot be
read is simply left out of the list, with one line underneath naming that file
and what was wrong with it — the window loses a row, not its usefulness. And a
folder with nothing readable in it puts a single line where the list would be,
saying no games are configured and naming the folder to put files into, with
"Something else" still sitting there, so the window is never a dead end.

## User experience

- **Entry** — the same "Add game" button from item 01. The dialog behind it
  changes shape; nothing else moves.
- **Flow** — the dialog opens with a list of games read from the configuration
  folder, each showing its display name, plus a "Something else" entry at the
  bottom of the list.
- **Flow** — choose a game and the dialog asks for one more thing: a name for
  this account, so two accounts on the same game can be told apart. Confirming
  creates it and loads the game.
- **Flow** — choose "Something else" and the dialog reveals the name and address
  fields from item 01, unchanged. Confirming creates an account with default
  values for everything the file would have supplied.
- **States** — **catalogue present**: the list shows one row per file, sorted by
  display name. **Catalogue empty**: the list area says no games are configured
  and names the folder to put files in, with "Something else" still available so
  the dialog is never a dead end. **A file that will not parse**: the list omits
  it and shows one line naming the file and the problem, rather than failing to
  open the dialog.
- **New pattern** — a chooser whose last entry reveals a different form. None of
  design rules 1 to 6 covers it: every one of them is about a sidebar row or the
  panel standing in for an absent game. The design doc owes a rule for an
  escape-hatch option inside a chooser, and for how a partial failure like an
  unreadable file is reported inside a form.

## Technical details

- **Front-end** — reshape `add-game-dialog.ui`, `add_game_dialog.rs` and
  `add_game_dialog/imp.rs` into two stages inside one dialog (architecture rules
  12, 13; naming rules 1, 2, 4, 7): the wrapper gains the catalogue and the
  confirm handler grows the preset arm, while the `imp` module holds the
  template children — a `gtk::ListView` over the catalogue entries plus the
  escape-hatch row,
  then a second page holding the account-name field, with the address field
  revealed only on the escape-hatch path. The action row is constant across both
  stages — cancel, and a suggested "Add" left insensitive until the current
  stage's fields are filled, extending the sensitivity wiring the dialog already
  has.
- **Front-end** — the failure line is a `gtk::Label` under the list, populated
  from the failed entries the port returns, so a bad file costs the dialog one
  row rather than its ability to open. When nothing at all is readable the list
  is replaced by a `gtk::Label` naming the presets directory the catalogue
  reported; the escape-hatch row survives both states.
- **Front-end** — the item's wording is that the dialog receives a
  `&dyn PresetCatalogue` and never learns that the entries are files; in this
  codebase that is an `Rc<dyn PresetCatalogue>`, mirroring how the window already
  receives the profile locator, because the dialog outlives the call that opens
  it. The trait object is the point either way: `scripts/arch-check.sh` forbids
  the shell from depending on the store crate (architecture rule 3).
- **Front-end** — the catalogue is listed when the dialog opens, not held from
  start-up, which is what makes a hand-added file appear on the next press.
- **Front-end** — `window/imp.rs`: `present_add_game_dialog` hands the dialog the
  catalogue, and the confirm handler gains a second arm calling
  `add_from_preset` and then building the holder exactly as `create_account`
  does today. The escape-hatch arm keeps calling `add` and is otherwise
  untouched (architecture rule 8 — the dialog reports what the user chose, the
  book decides what it means).
- **Front-end** — `main.rs` constructs `TomlPresetCatalogue` and hands it to
  `Window::new` beside the profile locator; a failure to construct it is
  reported with `anyhow` context like the locator's (architecture rules 3, 11).
- **Design** — write the two rules this screen owes into `docs/design.md`, as
  items 02 to 04 did for the patterns they introduced: how an escape-hatch
  option is separated from real options inside a chooser, and how a partial
  failure is reported inside a form without blocking it.
- **Testing** — architecture rule 14 and code standards rule 25 keep GTK out of
  `cargo test`, so this slice's evidence is the item's `test-script.md`. The
  shipped game files land in task 03, so verify against files written into the
  presets folder by hand — which is also the path a user adding their own game
  takes.

## Acceptance criteria

- [x] `(manual)` pressing "Add game" opens a dialog whose first stage lists the
      games in the presets folder, one row each showing only the display name,
      sorted by display name
- [x] `(manual)` the last row reads "Something else" and is separated from the
      games by a hairline
- [x] `(manual)` choosing a game shows one field, a name for this account, with
      "Add" insensitive until it is non-empty
- [x] `(manual)` choosing "Something else" reveals item 01's name and address
      fields unchanged, with "Add" insensitive until both are filled
- [x] `(manual)` confirming with a game creates an account under the typed name
      that loads that game's address, and its sidebar row carries the typed name
- [x] `(manual)` confirming through "Something else" creates an account exactly
      as item 01 did
- [x] `(manual)` an account created from a game whose file sets the
      background-speed default shows "keep running when hidden" already on in its
      row menu
- [x] `(manual)` a file that will not parse is left out of the list and named,
      with its problem, on one line under the list, while every other file still
      lists and the dialog still opens
- [x] `(manual)` with nothing readable in the folder, the list is replaced by one
      line saying no games are configured and naming the folder, and "Something
      else" is still choosable
- [x] `(manual)` a file added to the folder by hand appears the next time "Add
      game" is pressed, with no restart

## References

- [Roadmap item](../../roadmap/06-presets-and-adding-accounts/README.md) — the
  full picture, including the "Adding an account from the catalogue" interaction
  diagram that fixes the order the dialog, the catalogue and the book are called
  in
- [Wireframes](../../roadmap/06-presets-and-adding-accounts/wireframes/) — the
  two stages, the hairline above "Something else", where the failure line sits
  and what the empty list area says
- [`docs/requirements.md`](../../requirements.md) — `FR.10.2`, `FR.10.3`,
  `FR.6.1`
- [`docs/architecture.md`](../../architecture.md) — rules 3, 8, 10, 11, 12, 13,
  14
- [`docs/code-standards.md`](../../code-standards.md) — rules 6, 8, 11, 13, 14,
  15, 17, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7, 12
- [`docs/design.md`](../../design.md) — rules 1 to 6, which this dialog must not
  contradict; the two rules it owes are the ones it adds

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
