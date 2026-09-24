# 08 — The round trip on both systems, and the docs

**Roadmap:** [16](../../roadmap/16-release-pipeline-and-in-app-update/README.md) · **Scope:** full-stack · **Depends on:** 05, 07, 09

## Context

Every earlier slice proved its own part: the workflow publishes, the packages
build, the signature verifies, the notice renders. None of them has yet shown
the whole promise on a machine a user would have: a copy of the program that
was downloaded from a release, running, learning that a newer release exists,
fetching it, and coming back as the newer version with every account, login,
workspace, preset and zoom level exactly where it was. This slice runs that
round trip on both systems and writes it down as the item's hand-run test
script, and it writes the few paragraphs of documentation a user needs.

On Windows the run happens in the project's Windows 11 virtual machine. The
portable zip from one release is unzipped into a folder, the program is
started, an account is added and logged in, and a second release is published.
The notice appears, the update is fetched, "Restart now" is pressed, and the
program comes back at the new version with the account still logged in and
the data folder untouched. The time between the close request and the process
exiting is measured, because the helper kills a program that has not exited
within sixty seconds. The same run checks that the unsigned helper actually
runs on a clean Windows and records the SmartScreen prompt it shows.

On Linux the AppImage from one release is run in a clean Ubuntu 24.04
container with only the two system libraries the project requires, and the
same update to the next release is carried out.

The documentation is small and user-facing: how to download and run each
system's file, what the Windows warning looks like and that "Run anyway" is
the step, how the app updates itself, and the changed hand-out step in the
Windows VM guide. Both rule docs that name the seventh crate and the new tools
are checked once more against what was actually built.

## User experience

- **Flow** — On both systems: run the previous release, `☰` shows its
  version, the notice appears for the newer one, `Update` then `Restart now`,
  the app comes back at the newer version with every account restored and
  logged in.
- **States** — On Windows the first start of an unsigned download shows
  `Windows protected your PC`; `More info` → `Run anyway` starts it, and the
  README says so.
- **States** — On both systems the configuration and data folders have the
  same files, sizes and modification times before and after the update, apart
  from `sessions.toml`, which the close itself rewrites.

## Technical details

- **Architecture** — `docs/tasks/16-release-pipeline-and-in-app-update/test-script.md`
  gets its `## Setup` (publish two test releases from a `test/16-…` branch
  pointed at by a temporary `RELEASE_REPOSITORY` override, or two real
  releases), the Windows and Linux round-trip sections with one checkbox per
  action and observed result, and `## Teardown` (delete the test releases and
  tags).
- **Architecture** — Windows, per `docs/windows-vm.md`: unzip
  `IdleManager-win-Portable.zip` into `C:\idle-manager`, start
  `IdleManager.exe`, add and log into one account, record `dir /s
  %APPDATA%\idle-manager` before and after, time the close-to-exit interval
  with a stopwatch against the 60 s limit, and record whether SmartScreen or
  Smart App Control blocked anything.
- **Architecture** — Linux: `docker run --rm -it ubuntu:24.04` with `apt
  install libgtk-4-1 libwebkitgtk-6.0-4 xvfb`, run the previous release's
  AppImage under Xvfb with `RUST_LOG=idle_manager=debug`, drive the update
  through the notice with the XTest helper earlier items' runbooks used, and
  confirm the AppImage file's hash changed and the process relaunched.
- **Architecture** — `README.md` gains a `## Download` section (the two
  files, how to run each, the SmartScreen step, "the app checks for updates
  once a day and installs when you quit") and `docs/windows-vm.md` changes its
  hand-out step to the portable zip; `docs/stack.md` and
  `docs/architecture.md` are reread against the built crate and the tools and
  corrected where they drifted.
- **Architecture** — the roadmap item's Blockers on the portable swap, the
  AppImage and `libfuse2`, `clang-cl` on the runner and the 60 s limit are
  each answered in the item's `## As built` section with the measured result.
- **Code standards** — nothing in the code changes unless the round trip finds
  a defect, in which case the fix is its own `fix` commit with a subject
  written for the user (rule 30).

## Acceptance criteria

- [ ] `(manual)` Windows: the previous release's portable zip runs from
      `C:\idle-manager`, the notice offers the newer release, `Update` and
      `Restart now` bring the app back at the newer version, `Update.exe` was
      not blocked, and the account is still logged in
- [ ] `(manual)` Windows: `dir /s %APPDATA%\idle-manager` before and after
      differ only in `sessions.toml`'s timestamp, and the close-to-exit
      interval with four live accounts is recorded and under 60 s
- [ ] `(manual)` Windows: the first start of the downloaded zip shows the
      SmartScreen prompt, `Run anyway` starts it, and the README's `## Download`
      describes exactly that
- [ ] `(manual)` Linux: the previous release's AppImage runs in a clean
      `ubuntu:24.04` container with only `libgtk-4-1 libwebkitgtk-6.0-4`
      installed, updates to the newer release through the notice, and the file
      on disk carries the new release's checksum afterwards
- [ ] `(manual)` Linux: `~/.config/idle-manager` and `~/.local/share/idle-manager`
      are byte-identical before and after apart from `sessions.toml`
- [ ] `(integration)` `make verify` passes and `make roadmap-check` reports
      the item's tables current after `## As built` is written

## References

- [Roadmap item](../../roadmap/16-release-pipeline-and-in-app-update/README.md)
  — Back-end "The runbook and the Windows VM"; every Blocker
- [Wireframes](../../roadmap/16-release-pipeline-and-in-app-update/wireframes/)
  — the screens the round trip walks
- [`docs/requirements.md`](../../requirements.md) — Distribution `FR.3.2`,
  `FR.3.3`, `FR.4.5`, `FR.4.6`, `FR.4.7`, `FR.5.3`; Platform Support `FR.3.4`,
  `FR.3.5`
- [`docs/windows-vm.md`](../../windows-vm.md) — the VM the Windows half runs in
- [`docs/architecture.md`](../../architecture.md) — rule 14
- [`docs/code-standards.md`](../../code-standards.md) — rule 30

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
