# 06 — The update port, the policy and the Velopack channel

**Roadmap:** [16](../../roadmap/16-release-pipeline-and-in-app-update/README.md) · **Scope:** back-end · **Depends on:** 03, 04

## Context

The app needs to learn that a newer version exists, fetch it when the user
asks, check it, and arrange for it to install once the app has quit. This
slice builds all of that except the widget the user sees. It is split in two
parts, one pure and one that touches the network and the disk, because the
project keeps its rules separate from its machinery.

The pure part lives in the domain crate, the part of the program that knows
the rules but touches no screen, disk or network. It defines the closed list
of states an update can be in: idle, checking, up to date, available,
downloading with a percentage, verifying, ready, failed. It defines the
events that move between them: a check was asked for, by the clock or by the
user; a version was found or nothing newer was; the user asked to fetch;
progress arrived; the download passed or failed its checks; the user
dismissed the notice. One function takes a state and an event and answers the
next state and, sometimes, one thing to do: run a check, start a download,
show or hide. A second function answers whether the daily check is due, given
the time as a number, so the schedule is tested without waiting a day. A
dismissed "available" remembers the version, so the same version does not
reopen the notice within the run, while a dismissed "ready" still installs on
quit.

The machinery part lives in the update crate created two slices ago. It
implements the contract the domain declares over Velopack: ask GitHub's
release feed for the latest version, download the one full package with
progress, verify its checksum and its signature with the verifier from the
previous slice, and register the swap to happen after the process exits, with
or without a relaunch. It also reports the running version and, at launch, the
reason a previous swap did not apply, if any. Every call is blocking and is
meant to run on a worker thread; the window does that in the next slice.

## Technical details

- **Architecture** — `crates/idle-manager-core/src/update.rs`: `Version(u16,
  u16, u16)` with `FromStr`, `Display`, `Ord` (code standards rule 2);
  `UpdateState` with variants `Idle`, `Checking { manual }`, `UpToDate {
  current }`, `Available { version, notes_url }`, `Downloading { version,
  percent: u8 }`, `Verifying { version }`, `Ready { version }`, `Failed {
  version: Option<Version>, reason: String }` (rule 1); `UpdateEvent` with
  `CheckRequested { manual }`, `Found { version, notes_url }`, `NothingNewer {
  current }`, `CheckFailed { reason }`, `FetchRequested`, `Progress(u8)`,
  `Verified`, `Rejected { reason }`, `Dismissed`; `Effect` with `RunCheck`,
  `Download`; `UpdatePolicy { dismissed: Option<Version> }` with `apply(&mut
  self, state, event) -> (UpdateState, Option<Effect>)`; all re-exported from
  `lib.rs` (architecture rules 1, 9).
- **Architecture** — policy rules pinned by tests: an automatic
  `CheckFailed` or `NothingNewer` returns to `Idle`; a manual one goes to
  `Failed { version: None }` / `UpToDate`; `Found` with a version equal to
  `dismissed` stays `Idle` unless manual; `Dismissed` on `Available` records
  the version and hides; `Dismissed` on `Ready` hides but the state stays
  `Ready`; `FetchRequested` is accepted from `Available` and `Failed { version:
  Some }` only; `Progress` outside `Downloading` is ignored.
- **Architecture** — `UpdateSchedule` in the same file: `CHECK_INTERVAL_SECS:
  u64 = 86_400` (rule 5) and `next_check_due(last_check_millis: Option<u64>,
  now_millis: u64) -> bool`, true when no check has run or the interval has
  passed.
- **Architecture** — `ports.rs` gains `UpdateChannel: Debug + Send + Sync`
  with `check() -> Result<UpdateCheck, UpdateError>`, `download(&UpdateInfo,
  progress: &(dyn Fn(u8) + Send)) -> Result<VerifiedPackage, UpdateError>`,
  `apply_on_exit(&VerifiedPackage, relaunch: bool) -> Result<(),
  UpdateError>`, `current_version() -> Version`, `last_apply_failure() ->
  Option<String>`; `UpdateCheck::{UpToDate, Available(UpdateInfo)}`,
  `UpdateInfo { version, notes_url }`, `VerifiedPackage { version, path }`,
  `UpdateError` (`thiserror`: `Offline { reason }`, `RateLimited`, `Malformed
  { reason }`, `Rejected { reason }`, `Io { reason }`) (rules 5, 6, 11;
  naming rule 10).
- **Architecture** — `crates/idle-manager-update/src/channel.rs`:
  `VelopackChannel::new(repository_url: &str) -> Result<Self, ChannelSetup>`
  building `velopack::UpdateManager` over `sources::GithubSource::new(url,
  None, false)`; `check` maps `UpdateCheck::UpdateAvailable` to `Available`
  with `notes_url = <repo>/releases/tag/v<version>`; `download` calls
  `download_updates` with a progress channel forwarded to the callback, then
  fetches `<asset>.minisig` from the same release with `ureq` (already a
  dependency through Velopack) and calls `signature::verify_package`, mapping
  a rejection to `UpdateError::Rejected` after the file is gone; `apply_on_exit`
  calls `wait_exit_then_apply_updates(asset, false, relaunch, [])`;
  `current_version` reads the manager's current version; `last_apply_failure`
  reads Velopack's log for the previous run's failure line, `None` when there
  is none.
- **Architecture** — the update crate's `Cargo.toml` adds `ureq` at the
  version Velopack pins and `sha2`; `docs/stack.md` lists it with the reason;
  `scripts/arch-check.sh` needs no change (03 added the crate's rule).
- **Code standards** — every public item carries a `///` contract (rule 17);
  `Version` and every state derive `Debug`, `Clone`, `PartialEq`, `Eq` (rule
  4); core tests at the foot of `update.rs` (rule 24), one subject per test
  (rule 23), no display and no network (rule 25).

## Acceptance criteria

- [ ] `(unit)` `Version::from_str` parses `0.2.0` and `v0.2.0`, rejects `0.2`
      and `0.2.0-beta`, and `Version(0,3,0) > Version(0,2,9)`
- [ ] `(unit)` from `Idle`, `CheckRequested { manual: false }` yields
      `Checking` with `Effect::RunCheck`; then `NothingNewer` yields `Idle` with
      no effect, and `CheckFailed` yields `Idle`; the same two events after a
      manual request yield `UpToDate` and `Failed { version: None }`
- [ ] `(unit)` `Found(0.3.0)` yields `Available`; `Dismissed` then hides and
      records `0.3.0`; a later automatic `Found(0.3.0)` stays `Idle` while
      `Found(0.4.0)` yields `Available` again and a manual `Found(0.3.0)` yields
      `Available`
- [ ] `(unit)` `FetchRequested` from `Available` yields `Downloading { percent:
      0 }` with `Effect::Download`; `Progress(42)` yields `Downloading {
      percent: 42 }`; `Verified` yields `Ready`; `Rejected` yields `Failed {
      version: Some }`, from which `FetchRequested` is accepted again
- [ ] `(unit)` `Dismissed` on `Ready` leaves the state `Ready`, and
      `FetchRequested` from `Idle`, `Checking` or `Ready` changes nothing
- [ ] `(unit)` `next_check_due(None, t)` is true; `next_check_due(Some(t), t +
      86_399_000)` is false; `next_check_due(Some(t), t + 86_400_000)` is true
- [ ] `(integration)` `VelopackChannel::new` accepts
      `https://github.com/<owner>/<repo>` and rejects a non-GitHub URL with
      `ChannelSetup`
- [ ] `(integration)` `download` against a local fixture (a release directory
      served by a test HTTP server on `127.0.0.1` with a `.nupkg` and a
      `.minisig` from a different key) returns `UpdateError::Rejected` and the
      downloaded file is gone
- [ ] `(integration)` `make arch-check` passes and `make verify` passes

## References

- [Roadmap item](../../roadmap/16-release-pipeline-and-in-app-update/README.md)
  — Back-end "The port, the policy and the seventh crate"; the three
  interaction diagrams' state names; Technical References on Velopack's API
- [Wireframes](../../roadmap/16-release-pipeline-and-in-app-update/wireframes/)
  — `update-notice.md`'s state table is the vocabulary `UpdateState` fixes
- [`docs/requirements.md`](../../requirements.md) — Distribution `FR.4.1`,
  `FR.4.4`, `FR.4.6`, `FR.4.8`, `FR.5.1`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 2, 3, 5, 6, 9,
  11, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 2, 4, 5, 12,
  17, 23, 24, 25
- [`docs/naming.md`](../../naming.md) — rules 5, 10

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
