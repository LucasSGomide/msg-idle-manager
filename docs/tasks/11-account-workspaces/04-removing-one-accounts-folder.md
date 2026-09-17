# 04 — Removing one account's folder

**Roadmap:** [11](../../roadmap/11-account-workspaces/README.md) · **Scope:** back-end · **Depends on:** 01

## Context

This application runs several accounts of browser idle games in one window. Each
account keeps its own folder of website data on disk, holding the game's logins,
cookies and saved progress, so accounts of the same game never see each other's
sessions. An account that is no longer played can today only be left in the list
forever, with that folder still on disk. A later slice adds "Delete account" to
each account's menu. This slice builds the one step of deleting that touches the
disk: removing exactly one account's folder, and nothing else.

The rules layer of the app must never know about files. So this slice adds a
port, a small contract the rules layer owns, which says "remove this account's
data" and "tell me where it lives". The file store provides the real version, and
tests can provide a fake. The real version removes the account's folder with
everything in it. It treats a folder that is already gone as success, because a
deletion that failed half way and is then retried must be able to finish. When
removal really fails, for example because the folder cannot be written to, it
gives back a one-line reason ready to show a person.

It deliberately does not wait for the browser engine to let go of the account's
files. The engine was measured on this machine, and it never closes a deleted
account's cookie and security-policy databases while the app runs. Removing the
folder works anyway, and the other accounts keep their logins. Waiting would
simply never end.

This is its own slice because it can be proved completely against a real
temporary folder, well before any window exists to trigger it. It shares no file
with the tree and switching work, so it can run beside it.

## Technical details

- **Back-end** — a new `ProfileRemoval` port in
  `crates/idle-manager-core/src/ports.rs` has `remove(&self, id: &SessionId) ->
  Result<(), ProfileRemovalError>` and `folder(&self, id: &SessionId) ->
  PathBuf`. The domain's deletion flow must not know about the filesystem, and a
  test needs a fake (architecture rules 5 and 6).
- **Back-end** — the port is `Send + Sync`, because the shell calls it through
  `gio::spawn_blocking`.
- **Back-end** — `ProfileRemovalError` is a `thiserror` enum whose `Display` is a
  one-line reason ready to show (architecture rule 11).
- **Back-end** — there is no open-file check or settle wait in the port. On
  WebKitGTK 2.52.6 the network process kept `cookies.sqlite` and
  `hsts-storage.sqlite` open 60 s after the session was dropped.
  `remove_dir_all` still succeeded, and the sibling account kept its cookie and
  localStorage (roadmap item, Technical References).
- **Back-end** — `XdgProfileRemoval` in `crates/idle-manager-store/src/paths.rs`
  implements the port over `account_profile_dir` at `:74`. `folder` returns that
  path.
- **Back-end** — `XdgProfileRemoval::remove` calls `fs::remove_dir_all`, and an
  `io::ErrorKind::NotFound` counts as success, so a retry after a partial removal
  succeeds (`FR.21.11`). It removes exactly that one folder and never a parent or
  sibling (`FR.21.8`).
- **Back-end** — `XdgProfileRemoval` is exported from the store crate but not yet
  built in `crates/idle-manager/src/main.rs`. The deletion slice wires it in
  through `WindowPorts` (architecture rule 3).
- **Testing** — integration tests go under `crates/idle-manager-store/tests/`
  against a temporary data directory (code standards rules 21, 23). This task's
  `test-script.md` section holds the `cargo test` run for those tests and its
  pass count. The deletion slice's section exercises the port end to end.

## Acceptance criteria

- [x] `(integration)` `remove` deletes the account's profile folder together
      with its nested files and subfolders
- [x] `(integration)` removing one account's folder leaves a sibling account's
      folder and every file in it untouched
- [x] `(integration)` `remove` on an account whose folder does not exist returns
      `Ok`
- [x] `(integration)` `folder` returns the same path `account_profile_dir` gives
      for that account
- [x] `(integration)` removing a folder that sits inside a read-only directory
      returns `Err`, and its `Display` is a single line with no newline
- [x] `(unit)` an `Arc<dyn ProfileRemoval>` backed by a fake can be moved into a
      `std::thread` and called there

## References

- [Roadmap item](../../roadmap/11-account-workspaces/README.md) — the full
  picture, including the "Deleting an account" diagram whose "Profile removal"
  step this slice provides, and the Technical References measurement
- [`docs/requirements.md`](../../requirements.md) — `FR.21.8`, `FR.21.10`,
  `FR.21.11`
- [`docs/architecture.md`](../../architecture.md) — rules 3, 5, 6, 11
- [`docs/code-standards.md`](../../code-standards.md) — rules 21, 23
- [`docs/naming.md`](../../naming.md) — rules 6, 9

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
