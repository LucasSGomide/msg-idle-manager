# 06 — Naming workspaces: New, Rename and Remove

**Roadmap:** [11](../../roadmap/11-account-workspaces/README.md) · **Scope:** front-end · **Depends on:** 05

## Context

This application runs several accounts of browser idle games in one window,
grouped into workspaces. A workspace is a named set of at most four accounts with
its own arrangement, and a built-in workspace called Ungrouped holds the rest.
The previous slice added a selection mode to the sidebar: a person ticks accounts
and moves them into a workspace that already exists. There is still no way to
make a workspace, change its name or get rid of one. This slice adds all three.

Making one starts from the ticked accounts. The Move to… menu gains a last entry,
"New workspace…", below a divider. It opens a small window asking for a name.
The Create button stays greyed out while the name is empty, or while it matches
another workspace's name with capital letters ignored, Ungrouped included. While
the name is taken, a quiet line under the field says so. Creating makes the
workspace, holding exactly the ticked accounts and unfolded, and ends selection
mode. Cancelling returns to selection mode with every tick still in place, so a
mistyped name never costs the selection. When more than four accounts are ticked,
New workspace… is greyed out, because no workspace can hold them.

Each workspace heading, except Ungrouped's, gains a "⋯" menu like the one account
rows already have. Rename… opens the same small window with the current name
filled in. Remove workspace makes the heading disappear and sends its accounts to
the bottom of Ungrouped. It asks nothing first, because only the grouping is
lost: no game stops and nothing on disk is deleted. If the removed workspace was
on screen, Ungrouped is shown instead. Heading menus hide during selection mode,
so a click there cannot change the list mid-selection.

The name window reuses the one account renaming already uses, taught to take a
title and a name check. That shared change is why creating and renaming land
together in one slice.

## User experience

- **Entry** — a workspace heading's ⋯ menu offers `Rename…` and `Remove
  workspace`. It is absent on `Ungrouped` and hidden while selection mode is on.
  The `Move to…` menu gains a separator and `New workspace…` at its end.
- **Flow** — choose `New workspace…`. A small window titled "New workspace" asks
  for a name. `Create` is insensitive while the trimmed name is empty or matches
  another workspace's, ignoring case, `Ungrouped` included. Confirming creates the
  workspace holding exactly the ticked accounts, expanded, and ends selection
  mode. The shown workspace does not change. Cancelling returns to selection mode
  with the ticks kept.
- **Flow** — renaming a workspace: heading ⋯ → `Rename…` opens the same small
  window titled "Rename workspace", pre-filled and fully selected, with the same
  uniqueness rule.
- **Flow** — removing a workspace: heading ⋯ → `Remove workspace`. The workspace
  disappears and its accounts join the bottom of `Ungrouped`. If it was shown,
  `Ungrouped` is shown. Nothing on disk changes and no confirmation is asked,
  because nothing is lost but the grouping.
- **States** — **name taken or empty** (new or rename): the confirm button is
  insensitive, with no error text. A second dim line under the field reads
  "Another workspace has this name." only while the name is taken. **Ticked set
  larger than four**: `New workspace…` is insensitive.
- **Pattern** — the heading's ⋯ menu is the account row's ⋯ menu shape, design
  rule 5, built like `bind_row_menu` at
  `crates/idle-manager-shell/src/session_sidebar/imp.rs:347`. The name window is
  `RenameDialog` (`crates/idle-manager-shell/src/rename_dialog.rs`) with a title
  and a name check passed in, reusing its default-button and select-all
  behaviour. `New workspace…` sits under a separator, the escape-hatch shape of
  design rule 7. The dim line is design rule 8.

## Technical details

- **Front-end** — `RenameDialog` gains a constructor taking a title, a confirm
  label, the starting text and a check closure returning `Ok`, `Empty` or
  `Taken`. The account rename keeps calling it with `account_name`, which never
  answers `Taken`. The workspace dialog passes a closure over `WorkspaceBook`'s
  name rule that ignores the workspace being renamed. `Taken` shows the dim line.
  The dialog decides nothing else.
- **Front-end** — a new `bind_heading_menu`, the same shape as `bind_row_menu`
  at `session_sidebar/imp.rs:347`, adds `rename` and `remove` actions under a
  `heading` action group, their names constants. `Ungrouped` gets no menu button.
  The heading menu button is hidden while `is_selecting` is set. The actions
  report `connect_workspace_rename_requested(WorkspaceId)` and
  `connect_workspace_remove_requested(WorkspaceId)`.
- **Front-end** — the move menu appends a menu section break, then `New
  workspace…`, insensitive when `Destinations::can_create` is false (design rule
  7). It emits `connect_move_requested` with a new `MoveTarget::New` variant.
- **Front-end** — a window intent `present_workspace_name_dialog(purpose)` covers
  both create and rename, transient for the window. For create, confirming calls
  `WorkspaceBook::create_workspace(name, ids)`, and only on `Ok` does it call
  `end_selection`, `redraw` and `request_save`. A cancelled window calls nothing,
  so the mode and every tick survive (`FR.17.7`). For rename, confirming calls
  `rename_workspace`, then redraws and saves.
- **Front-end** — a window intent `remove_workspace(id)` calls
  `WorkspaceBook::remove_workspace`. When the active workspace changed, it runs
  what a switch runs: `select_layout_toggle` for `Ungrouped`'s layout and
  `snap_zoom_for_active`. Then it redraws and saves. It never touches a
  `SessionView` holder or the disk (`FR.15.4`, `FR.15.11`, `FR.21.8`).
- **Design** — rules 5, 7 and 8. Account renaming keeps its "Rename account"
  title, `Rename` label and empty-name behaviour unchanged.
- **Testing** — `(manual)` against the headless harness, seeded with a version 2
  `sessions.toml`, in this task's section of `test-script.md` (architecture rule
  14).

## Acceptance criteria

- [x] `(manual)` `Ungrouped`'s heading has no ⋯ button, and a named heading's ⋯
      opens a menu holding `Rename…` then `Remove workspace`
- [x] `(manual)` in selection mode every heading's ⋯ button is hidden, and each
      returns after `Done`
- [x] `(manual)` `Move to…` ends with a separator and `New workspace…`, which is
      insensitive while five accounts are ticked
- [x] `(manual)` in the "New workspace" window, `Create` is insensitive for an
      empty or all-spaces name. Typing "ungrouped", or an existing workspace's
      name in another case, keeps it insensitive and shows the dim "Another
      workspace has this name." line, which goes once the name is unique
- [x] `(manual)` confirming a unique name adds its heading just above
      `Ungrouped`, expanded, holding exactly the ticked accounts. Selection mode
      ends, the shown workspace is unchanged, and `sessions.toml` lists the new
      workspace
- [x] `(manual)` cancelling or pressing Escape in the "New workspace" window
      returns to selection mode with every tick and the count intact
- [x] `(manual)` `Rename…` opens "Rename workspace" with the current name fully
      selected and `Rename` sensitive. A taken name greys it with the dim line,
      and a confirmed new name shows on the heading and survives a relaunch
- [x] `(manual)` `Remove workspace` on a hidden workspace removes its heading
      without asking. Its accounts appear at the bottom of `Ungrouped` in order,
      still running, and their profile folders remain on disk
- [x] `(manual)` `Remove workspace` on the shown workspace shows `Ungrouped`, and
      the 1/2/4 buttons take `Ungrouped`'s layout
- [x] `(manual)` an account row's `Rename…` still opens "Rename account", and an
      empty name still greys `Rename` with no dim line

## References

- [Roadmap item](../../roadmap/11-account-workspaces/README.md) — the full
  picture, including the `New workspace…` branch of the "Grouping accounts in
  selection mode" diagram and the Naming state of "Sidebar modes"
- [Wireframes](../../roadmap/11-account-workspaces/wireframes/) — this slice
  draws `workspace-name-window.md`, the heading menu in `sidebar-tree.md`, and
  the `New workspace…` entry in `selection-mode.md`
- [`docs/requirements.md`](../../requirements.md) — `FR.15.4`, `FR.15.8`,
  `FR.15.11`, `FR.17.2`, `FR.17.6`, `FR.17.7`
- [`docs/design.md`](../../design.md) — rules 5, 7, 8
- [`docs/architecture.md`](../../architecture.md) — rules 8, 12, 13, 14
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
