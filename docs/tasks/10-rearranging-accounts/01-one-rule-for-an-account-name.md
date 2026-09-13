# 01 — One rule for an account name, and renaming in the book

**Roadmap:** [10](../../roadmap/10-rearranging-accounts/README.md) · **Scope:** back-end · **Depends on:** —

## Context

This application runs several accounts of browser idle games in one window. Each
account has a name the user typed when they created it, and today that name can
never change. The only way to fix a typo is to delete the account and add it
again, which throws away the game's saved login.

This slice teaches the application's rules layer how to rename an account. It
draws nothing on screen. A later slice adds the menu item and the small window a
person actually uses. Here the work is the rule itself and the one operation that
applies it.

The rule already exists, but only inside the window for adding a game. A name
has the spaces around it trimmed, and a name that is empty after trimming is
refused. This slice moves that rule into one small function in the rules layer.
The add-game window then calls it instead of repeating the rule, so the add
window and the future rename window can never disagree. A person adding a game
sees no difference.

The object that holds every account then gains a rename operation. Given an
account and some text, it applies the rule and stores the result as that
account's name. It reports whether it stored anything: an unknown account or an
empty name stores nothing. It changes the name and only the name. The hidden
identifier that names the account's folder on disk stays the same, so the login
and saved sizes in that folder are untouched. A running game keeps running, a
paused one stays paused, and the account keeps its place on screen. Two accounts
may share a name, because creating one already allows that.

The saved list of accounts already records each name, so a rename reaches the
file on disk with no change to how the file is written.

## Technical details

- **Back-end** — `crates/idle-manager-core/src/session.rs` gains a pure
  `account_name(raw: &str) -> Option<String>`: the trimmed text, or `None` when it
  is empty. It is the single statement of the rule `FR.13.3` says both dialogs
  share.
- **Back-end** — the add-game dialog stops spelling the rule inline at
  `crates/idle-manager-shell/src/add_game_dialog/imp.rs:252` (button
  sensitivity) and `:263` (the stored name) and calls `account_name` at both, so
  the two dialogs cannot drift. No visible change.
- **Back-end** — `SessionBook::rename(&mut self, account: &SessionId, name: &str)
  -> bool` runs `account_name`, writes the result to the session's
  `display_name`, and returns whether it stored anything. It returns `false` for an
  unknown id or an empty name.
- **Back-end** — `rename` touches no other field: not `id`, not liveness, not
  visibility, not keep-awake, not `focused`, not the order of `sessions`
  (`FR.13.2`, `FR.13.5`). A duplicate name is accepted.
- **Back-end** — `display_name` is already carried by `SessionBook::workspace`
  at `crates/idle-manager-core/src/session.rs:342`, so a rename reaches the saved
  file with no store change (`FR.13.4`).
- **Architecture** — core stays pure domain (architecture rules 1, 7, 9). The
  function and method follow `docs/code-standards.md` rules 1, 3, 17, 21 and 23
  and `docs/naming.md` rules 6, 9, 11 and 12.
- **Testing** — unit tests at the foot of `session.rs` (code standards rule 24).
  The add-game dialog check is `(manual)`, because shell behaviour is covered by
  `test-script.md` (architecture rule 14). This task's section there holds the
  `cargo test` run and the add-game check. If this is the item's first accepted
  slice, it also writes `## Setup` and `## Teardown`.

## Acceptance criteria

- [ ] `(unit)` `account_name("  Main account  ")` returns `Some("Main account")`
- [ ] `(unit)` `account_name` returns `None` for an empty string and for a
      whitespace-only string
- [ ] `(unit)` `rename` with a padded name stores the trimmed name and returns
      `true`, and the session's id, liveness, visibility and keep-awake flag, the
      book's focused slot and the order of its sessions are all equal to before
- [ ] `(unit)` `rename` with an empty or whitespace-only name returns `false` and
      leaves the book equal to before
- [ ] `(unit)` `rename` of an unknown id returns `false` and leaves the book equal
      to before
- [ ] `(unit)` renaming a parked account and a queued account keeps each in the
      same liveness it had
- [ ] `(unit)` renaming an account to the name another account already has is
      accepted and both sessions keep their own ids
- [ ] `(unit)` `SessionBook::workspace` after a rename carries the new name for
      that account
- [ ] `(manual)` in the add-game dialog, a whitespace-only name keeps the confirm
      button insensitive, and a padded name is added with its spaces trimmed

## References

- [Roadmap item](../../roadmap/10-rearranging-accounts/README.md) — the full
  picture, including the "Renaming an account" diagram whose session-book step
  this slice implements
- [`docs/requirements.md`](../../requirements.md) — `FR.13.2`, `FR.13.3`,
  `FR.13.4`, `FR.13.5`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 7, 9, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 3, 17, 21, 23,
  24
- [`docs/naming.md`](../../naming.md) — rules 6, 9, 11, 12

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
