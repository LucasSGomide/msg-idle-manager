# 07 — Choosing a workspace when adding a game

**Roadmap:** [11](../../roadmap/11-account-workspaces/README.md) · **Scope:** front-end · **Depends on:** 06

## Context

This application runs several accounts of browser idle games in one window,
grouped into workspaces. A workspace is a named set of at most four accounts with
its own arrangement, and a built-in workspace called Ungrouped holds the rest. A
new account is made in an add-game window: the person picks a game, types a name
for the account, and confirms. Until now every new account lands in whatever
workspace is on screen, which breaks as soon as that workspace is full. This slice
lets the person choose.

The add-game window's details form gains a Workspace field. It lists every named
workspace that still has room for one more account, then Ungrouped, which always
has room. A full workspace is simply not offered. The field starts on the
workspace currently on screen when that has room, and on Ungrouped otherwise, so
the common case needs no extra click.

Adding into a workspace that is not on screen does not switch to it. The new
account starts running inside its workspace, out of sight, and its row appears
under that workspace's heading in the sidebar. When the person later switches to
that workspace, the game is already loaded and nothing reloads. The account's
zoom is also worked out for the arrangement of the workspace it joined, not the
one on screen, so it appears at the right size the first time it is shown.

This is a small slice of its own because it touches the add-game window and the
account-creating code, which no other slice of this item changes. It depends on
the naming slice only because both edit the main window's code, so they cannot
run at the same time.

## User experience

- **Entry** — the add-game window's details form gains a `Workspace` field.
- **Flow** — the field lists named workspaces with room for one more, then
  `Ungrouped`. It defaults to the shown workspace when that has room, otherwise
  `Ungrouped`.
- **Flow** — adding into a workspace that is not shown creates the account
  running there, out of sight. The shown workspace, the grid and the 1/2/4
  buttons do not change, and the new row appears under its workspace's heading.
- **States** — **shown workspace full**: it is not listed, and the field
  defaults to `Ungrouped`. **Only `Ungrouped` exists**: the field lists
  `Ungrouped` alone, selected.
- **States** — **new account in a hidden workspace**: its row keys as
  `background` (or its liveness key while starting), never `current` or
  `visible` (design rules 1 and 3).

## Technical details

- **Front-end** — the details stage in `add_game_dialog/imp.rs` gains a
  `Workspace` `gtk::DropDown` over the `Destinations` for one account
  (`WorkspaceBook::destinations(1)`), in list order with `Ungrouped` last.
- **Front-end** — the drop-down defaults to the shown workspace when it appears
  in the destinations, and to `Ungrouped` otherwise (`FR.17.5`).
- **Front-end** — `AddGameDialog::new` at `add_game_dialog.rs:54` takes the
  destinations and the default. Both `Confirmed` variants at `:28` gain
  `workspace: WorkspaceId`. The dialog decides nothing about room.
- **Front-end** — `present_add_game_dialog` in `window/imp.rs` asks the book for
  `destinations(1)` and the default before opening the dialog.
- **Front-end** — `create_account` and `create_account_from_preset` at
  `window/imp.rs:590` pass the chosen workspace to `WorkspaceBook::add` and
  `add_from_preset`. They never switch (`FR.17.9`). A refusal cannot arise from
  the offered list, and one that does creates nothing.
- **Front-end** — `realise_account` at `window/imp.rs:607` resolves the zoom
  against the chosen workspace's layout, not the shown one. The grid places the
  new view through `WorkspaceBook::placement`, so it is off-grid when its
  workspace is hidden.
- **Design** — rules 1 and 3 for the new row. The field reuses the details
  stage's existing label-and-control spacing.
- **Testing** — `(manual)` against the headless harness, seeded with a version 2
  `sessions.toml`, in this task's section of `test-script.md` (architecture rule
  14).

## Acceptance criteria

- [x] `(manual)` with `Ungrouped` shown and "Duo" holding two accounts, the
      `Workspace` field lists "Duo" then `Ungrouped`, with `Ungrouped` selected
- [x] `(manual)` with "Duo" shown, the field defaults to "Duo", and a "Party"
      holding four accounts is not listed
- [x] `(manual)` with the full "Party" shown, the field defaults to `Ungrouped`
- [x] `(manual)` with only `Ungrouped` in the file, the field lists `Ungrouped`
      alone, selected
- [x] `(manual)` adding a game into "Duo" while `Ungrouped` is shown puts the new
      row under Duo's heading with the background key. The shown workspace, grid
      and 1/2/4 buttons are unchanged
- [x] `(manual)` switching to "Duo" after that shows the new account's page
      already loaded, with no reload, at the zoom its preset gives for Duo's layout
- [x] `(manual)` after a relaunch the new account is still listed under "Duo"

## References

- [Roadmap item](../../roadmap/11-account-workspaces/README.md) — the full
  picture
- [Wireframes](../../roadmap/11-account-workspaces/wireframes/) — this slice
  draws `add-game-workspace-field.md`
- [`docs/requirements.md`](../../requirements.md) — `FR.17.5`, `FR.17.9`,
  `FR.15.6`
- [`docs/design.md`](../../design.md) — rules 1, 3
- [`docs/architecture.md`](../../architecture.md) — rules 8, 14
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
