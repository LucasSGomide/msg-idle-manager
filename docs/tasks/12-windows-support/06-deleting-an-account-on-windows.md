# 06 — Deleting an account on Windows

**Roadmap:** [12](../../roadmap/12-windows-support/README.md) · **Scope:** full-stack · **Depends on:** 04

## Context

The application can delete an account for good. A confirmation window asks
first, and then the account's game data and its folder on disk are removed. On
Linux that runs as a measured sequence: ask the engine to clear the account's
data, drop the engine objects, wait a quarter of a second, then remove the
folder.

On Windows the order has to change. There, every account's engine data lives in
one shared folder. Inside it, each account has a named profile, and the shared
engine keeps those files open for as long as it runs. Removing files from under
it would fail or corrupt the engine. WebView2 provides its own call to delete
one profile, and this slice uses it.

On Windows, deleting an account closes the account's game and asks the engine
to delete that account's profile. Once the engine reports it done, the
account's own small folder is removed, as on Linux. That folder now holds only
its saved zoom settings and two empty folders. If the engine's profile deletion
fails, the confirmation window shows the same error page with `Retry` and
`Close` that Linux shows for a failed removal. No other account's profile or
folder is ever touched.

It is unconfirmed which WebView2 version added the profile-deletion call. So
the slice first checks for the call on the runtime the Windows machine has. If
the call is missing, it falls back to removing that one profile's folder inside
the shared engine folder after the view is gone, and records which path was
taken.

This is a slice of its own because it changes the one destructive action in
the application. It needs its own careful, hand-run proof that a sibling
account survives.

## User experience

- **Entry** — `Delete account…` in an account row's menu, as on Linux (item 11).
- **Flow** — Confirm → the dialog shows progress and cannot be dismissed →
  on success the account is gone from its workspace, the sidebar and the grid.
- **States** — Failure: the dialog turns into an error naming the profile or
  folder and the reason, with `Retry` and `Close`, exactly as on Linux.
- **Pattern** — item 11's delete-account window and its error page are reused
  unchanged (`delete_account_dialog.rs`); no new status is added to the sidebar
  (design rule 1).

## Technical details

- **Architecture** — `EngineProfile::delete(done)` on Windows closes the
  account's live view if one exists and gets the environment's
  `ICoreWebView2Profile` for the account's profile name. Through `ffi.rs` it
  calls `ICoreWebView2Profile8::Delete` and completes `done` on the profile's
  `Deleted` event, on the GTK main context (architecture rule 10). On Linux,
  `delete` wraps the existing clear-website-data step, so
  `account_deletion.rs` calls one engine-neutral operation.
- **Architecture** — `account_deletion.rs` becomes engine-neutral. The order is
  engine `delete`, then drop of the view, holder and profile, then item 11's
  quarter-second wait, then the existing `ProfileRemoval::remove` of
  `profiles/<id>/`. Linux keeps item 11's measured order exactly.
- **Architecture** — fallback. If `cast::<ICoreWebView2Profile8>()` fails on the
  installed runtime, `delete` waits for the view's controller to close and then
  removes `<engine_data_root>/EBWebView/<profile name>/` (verify the exact
  folder name on the machine and record it). It reports a filesystem error
  through the same `done` result. The path taken is logged at `info` with the
  runtime version.
- **Architecture** — a failed engine deletion maps to item 11's existing error
  state with its `Retry` and `Close`. `Retry` runs the whole sequence again, and
  a profile already gone counts as deleted (Session Management `FR.21.11`).
- **Code standards** — the order is a pure `fn deletion_steps(engine) ->
  [Step; N]`, unit-tested for both engines (rules 21–23). The COM calls stay in
  `ffi.rs` with `// SAFETY:` lines (rule 28 as amended). A comment names the
  shared-folder constraint behind the reorder (rule 18).

## Acceptance criteria

- [ ] `(unit)` `deletion_steps` lists engine delete, drop, wait, then folder
      removal for both engines, and the Linux list equals item 11's order
- [ ] `(integration)` the existing store `ProfileRemoval` tests still pass,
      including a folder already gone counting as removed
- [ ] `(integration)` `make windows-check` and `make verify` pass
- [ ] `(manual)` in the Windows VM, deleting one of two logged-in accounts of the same
      game removes it from the sidebar and grid, removes its profile folder
      inside the engine folder and its `profiles/<id>/` folder, and the other
      account is still logged in after a relaunch
- [ ] `(manual)` in the Windows VM, the log names which deletion path ran (engine call
      or fallback) and the runtime version
- [ ] `(manual)` in the Windows VM, with the account's `profiles/<id>/` folder made
      read-only, deletion shows the error page with `Retry` and `Close`. After
      the folder is made writable again, `Retry` completes the deletion
- [ ] `(manual)` on Linux, item 11's delete-account test-script steps still pass

## References

- [Roadmap item](../../roadmap/12-windows-support/README.md) — Front-end
  "Deleting an account on Windows", Blocker 5
- [Wireframes](../../roadmap/12-windows-support/wireframes/) — no new screen;
  item 11's `delete-account-window.md` applies
- [`docs/requirements.md`](../../requirements.md) — Platform Support `FR.1.5`;
  Session Management `FR.21.8`–`FR.21.11`
- [`docs/architecture.md`](../../architecture.md) — rules 3, 5, 10
- [`docs/code-standards.md`](../../code-standards.md) — rules 18, 21–23, 28
- [`docs/design.md`](../../design.md) — rule 1

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The `(manual)` steps run in the
Windows VM ([`docs/windows-vm.md`](../../windows-vm.md)).
