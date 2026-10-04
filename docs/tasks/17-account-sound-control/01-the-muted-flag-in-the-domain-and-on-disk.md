# 01 — The muted flag in the domain and on disk

**Roadmap:** [17](../../roadmap/17-account-sound-control/README.md) · **Scope:** back-end · **Depends on:** —

## Context

This application keeps several browser idle games running at once, each as an
account with its own isolated storage. The owner wants to silence one game
without silencing the computer. Before any menu item or engine call exists, the
application has to be able to remember, per account, that the owner wants it
silent, and it has to remember that across a restart.

This slice adds that memory and nothing else. In the pure domain crate, the
part of the code that knows what an account is and how it changes state, each
account gains one more yes-or-no fact beside the existing "keep running when
hidden" flag: whether it is muted. A new transition sets it and reports whether
anything actually changed, so a caller can skip work when the owner ticks a box
that was already ticked. Unlike the keep-awake transition, which has to mark the
account as starting because the page is about to reload, this one leaves the
account's liveness exactly as it found it: a mute is applied in place and no
page reloads.

In the storage crate, the account's entry in the workspace file the application
rewrites after every change gains one optional key. The key is written only when
the account is muted and read as "not muted" when absent, so a file with no
muted account is written byte for byte as it is today. The file's format version
does not change, and the readers for the two older formats are untouched because
no old file ever carried the key.

It is its own slice because it is the one part of the feature that can be
proven entirely by fast tests with no display: the domain tests run in
milliseconds and the storage tests write to a temporary folder. Everything the
later slices do is checked against what this one stores.

## Technical details

- **Architecture** — `docs/architecture.md` rules 1, 7 and 8: the core names no
  engine type, the store owns the on-disk key and maps it to the domain, the
  transition is intent → state with no widget involved.
- **Architecture** — in `crates/idle-manager-core/src/session.rs`: a private
  `is_muted: bool` on `Session`, a `#[must_use] pub fn is_muted(&self) -> bool`
  reader beside `is_kept_awake()`, and
  `SessionBook::set_muted(&mut self, session: &SessionId, value: bool) -> bool`
  beside `set_keep_awake`. Returns `false` for an unknown id or an unchanged
  value; never touches `liveness`. Default `false` in `SessionBook::add` and
  the preset path; map it both ways in the `Account`↔`Session` conversions.
- **Architecture** — in `crates/idle-manager-core/src/workspace.rs`: `pub
  is_muted: bool` on `Account`. In `workspace_book.rs`: `WorkspaceBook::set_muted`
  beside `set_keep_awake`, delegating to the holding book. `remote.rs`'s
  `RemoteAccount` is not changed.
- **Architecture** — in `crates/idle-manager-store/src/session_file.rs`: `muted:
  bool` on `SessionEntry` with
  `#[serde(default, skip_serializing_if = "std::ops::Not::not")]`, mapped in
  `entry_to_account` and `account_to_entry`. `FORMAT_VERSION` stays 3;
  `LegacySessionEntry` unchanged. The existing workspace-file snapshot must pass
  as it is.
- **Code standards** — `docs/code-standards.md` rules 21–24 for the tests at the
  foot of `session.rs`; rule 17 `///` on every new public item; rule 13 no
  `unwrap` outside tests.
- **Naming** — `docs/naming.md` rule 12: the boolean reads as a question,
  `is_muted`; the on-disk key is the plain `muted`, like `keep_awake`.

## Acceptance criteria

- [x] `(unit)` an account added to the book starts with `is_muted() == false`
- [x] `(unit)` `set_muted` to the value already held returns `false` and leaves the book untouched
- [x] `(unit)` muting a live account returns `true` and leaves its liveness `Live`; muting a parked account leaves it `Parked`
- [x] `(unit)` the flag survives park, unpark, a layout change that moves the account between pages, and a move between workspaces
- [x] `(unit)` `set_muted` on an id no workspace holds returns `false`
- [x] `(unit)` a `workspace()` → `restore()` round trip preserves `is_muted` on every account
- [x] `(integration)` a workspace list with one muted account writes and reads back with that account muted and the rest not
- [x] `(integration)` an entry for an unmuted account carries no `muted` key, and `the_written_file_matches_its_snapshot_with_the_version_key` passes unchanged
- [x] `(integration)` a version 3 file with `muted = true` on one entry reads back with only that account muted

## References

- [Roadmap item](../../roadmap/17-account-sound-control/README.md) — the full picture
- [Wireframes](../../roadmap/17-account-sound-control/wireframes/) — the menu this flag will be read from
- `docs/architecture.md` · `docs/code-standards.md` · `docs/naming.md`

## Implement with

A fresh implementation session with `Scope: back-end`; `make verify` before acceptance.
