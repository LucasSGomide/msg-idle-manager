# Test script — 16 Releasing from GitHub, and updating from inside the app

## Setup

- [x] `make bootstrap` on a machine that already has the SDK downloads → `Summary Successfully installed cargo-deny, cargo-watch, cargo-xwin, cargo-nextest!`, `system-check: GTK 4 and WebKitGTK development files present`, both `windows-sdk-fetch` and `windows-crt-fetch` report `already present; skipping the download`
- [x] `make verify` on a clean working tree → exits 0, ending with `arch-check: layer boundaries hold` then `roadmap tables are up to date`
- [ ] Two real releases against the production repository, `v0.1.0` then a later `v0.1.1` (or whichever versions the real commit history gives), each published by pushing a releasable commit to `main` and letting `.github/workflows/release.yml` run for real — `VelopackChannel::new` in `crates/idle-manager/src/main.rs` takes `RELEASE_REPOSITORY` (`https://github.com/LucasSGomide/msg-idle-manager`) as a compile-time constant with no environment or runtime override, so the `test/16-…` branch and temporary `RELEASE_REPOSITORY` override this task file itself suggests is not something the shipped code supports without a throwaway rebuild pointed at a second repository; the straightforward path is two ordinary releases on the real repository (not run: this session must not push, tag, publish a release, or otherwise touch GitHub)

## Teardown

- [ ] Delete any throwaway branch pushed to `origin` only to prove a red CI run or a cache miss
- [ ] If task 08's round trip published throwaway assets to a scratch repository rather than two ordinary production releases, delete those releases and their tags (not applicable if the round trip instead used two real, keepable releases on the production repository)

## 01 — Verifying every push on GitHub

- [x] `make verify` on `feat/16-release-pipeline` → exits 0, ending with `arch-check: layer boundaries hold` then `roadmap tables are up to date` (reuses the Setup step above)
- [x] `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/ci.yml"))'` → no error, confirming `ci.yml` is well-formed YAML
- [x] `actionlint .github/workflows/ci.yml` (v1.7.7) → exit 0, no findings
- [x] `grep -oP '^GVSBUILD_VERSION := \K.*' Makefile` → `2026.8.0`; `grep -oP '^WINDOWS_CRT_PACKAGE := \K.*' Makefile` → `Microsoft.VC.14.44.17.14.CRT.Redist.X64.base` — the same two commands the workflow's "Read the versions the caches are keyed on" step runs
- [x] Append a badly-formatted function to a source file and run `make fmt-check` → non-zero exit, `Diff in .../lib.rs` printed, `Error 1` — confirms the same `fmt-check` step the workflow runs is what turns a formatting break red; the file was then reverted with `git checkout --`
- [ ] Push a branch and open a pull request against `main` → the first run of `ci.yml` is green, and its log shows `make verify` ending with `arch-check: layer boundaries hold` and `roadmap tables are up to date` (needs a real push; not run from here)
- [ ] Push again with `Cargo.lock` and the `Makefile` unchanged → the run's log shows `Cache restored from key …` for all four caches (the GTK SDK, the CRT package, `cargo-xwin`'s cache, the cargo registry), and neither the gvsbuild zip nor the CRT package is downloaded (needs a second real run; not run from here)
- [ ] Push a commit that breaks formatting on a throwaway branch → the workflow goes red at the `fmt-check` step (needs a real push; not run from here — the local equivalent above shows the same step fails the same way)

## 02 — The version and the notes from the commits

- [x] `make release-version` on this repository, which has no release tag yet → prints `0.1.0`, matching `Cargo.toml`'s `[workspace.package] version`
- [x] `make release-notes` on this repository → prints `## [unreleased]` followed by a `### New` group listing every `feat` subject since the project's first commit (`Domain layer for sessions, layouts, and profile locations`, …), with no `docs`, `chore`, `test` or `refactor` line among them
- [x] Build a throwaway clone (`mktemp -d`, `git init`, a minimal two-crate workspace, `cliff.toml`/`Makefile`/`scripts/release-prepare.sh` copied in), tag it `v0.1.0`, then add two `docs` commits and one `chore` commit → `make release-version` prints nothing, `./scripts/release-prepare.sh` exits 1 printing `release-prepare: nothing to release`, and `git status --porcelain` is empty before and after
- [x] On that clone, add one `fix` commit → `make release-version` prints `0.1.1`; add one `feat` commit → it prints `0.2.0`
- [x] On that clone, put a `chore` commit on a topic branch and merge it back with `git merge --no-ff` → `make release-notes` lists the `feat` subject under `### New` and the `fix` subject under `### Fixed`, with no `docs`, `chore` or `Merge` line anywhere in the output
- [x] `./scripts/release-prepare.sh` on that clone → prints `0.2.0`; `Cargo.toml`'s `[workspace.package]` section now reads `version = "0.2.0"`; `Cargo.lock` lists both `throwaway-a` and `throwaway-b` at `version = "0.2.0"`; `CHANGELOG.md` starts with a `## [0.2.0] - 2026-09-24` section holding the same `New`/`Fixed` lines `release-notes` printed
- [x] `./scripts/release-prepare.sh` again on that now-dirty clone → exits 1 printing `release-prepare: Cargo.toml, Cargo.lock or CHANGELOG.md already has uncommitted changes`, and `md5sum` of `Cargo.toml` and `CHANGELOG.md` is unchanged from before the run
- [x] A separate throwaway clone tagged `v0.2.0` with a single `feat!:` commit on top → `make release-version` prints `0.3.0`, never `1.0.0`
- [x] `make release-tools-test` → runs `scripts/tests/release-prepare-test.sh`, which builds the clone above itself and prints `release-prepare-test: all assertions passed`; `make verify` on the working tree → exits 0, ending with `arch-check: layer boundaries hold` then `roadmap tables are up to date` (reuses the Setup step above; `release-tools-test` runs as part of `make test` inside it)

## 03 — Packaging both systems with Velopack

- [x] `dotnet tool install -g vpk` (installed a user-level .NET 9 SDK first with `dotnet-install.sh --install-dir ~/.local/dotnet`, no sudo) → `Tool 'vpk' (version '1.2.158') was successfully installed.`; `apt download clang-21 lld-21 llvm-21 libclang-common-21-dev libobjc-15-dev libpfm4 llvm-21-linker-tools llvm-21-runtime` then `dpkg-deb -x` each into `~/.local/llvm21` (no sudo, apt has no root-owned install step) → `~/.local/llvm21/usr/lib/llvm-21/bin/clang-cl --version` prints `Ubuntu clang version 21.1.8`, `lld-link --version` prints `Ubuntu LLD 21.1.8`
- [x] `./scripts/system-check.sh clang-cl lld-link` with the tools above off `PATH` → exits 1, printing `system-check: missing clang-cl` and `system-check: missing lld-link`, each followed by `  apt install clang lld llvm`; with the tools on `PATH` → exits 0, printing `system-check: clang-cl present` and `system-check: lld-link present`
- [x] `make windows-package` (with `clang-cl`/`lld-link`/`vpk` on `PATH`) → ends `windows-package: dist/releases/win/IdleManager-win-Portable.zip carries Update.exe`, `... carries current/idle-manager.exe`, `windows-package: wrote dist/releases/win/IdleManager-win-Portable.zip, dist/releases/win/IdleManager-0.1.0-full.nupkg and dist/releases/win/releases.win.json (43M)`; `ls dist/releases/win` lists exactly `IdleManager-0.1.0-full.nupkg`, `IdleManager-win-Portable.zip`, `RELEASES`, `assets.win.json`, `releases.win.json` — no `IdleManager-win-Setup.exe`
- [x] `make linux-package` → ends `linux-package: wrote dist/releases/linux/IdleManager.AppImage, dist/releases/linux/IdleManager-0.1.0-linux-full.nupkg and dist/releases/linux/releases.linux.json (6.2M); idle-manager links GTK and WebKitGTK from the system` — the script's own `ldd` check on the AppImage's extracted binary passed (both `libgtk-4.so.1` and `libwebkitgtk-6.0.so.4` resolve to `/usr/lib/x86_64-linux-gnu/...`, not into the extracted `squashfs-root`)
- [ ] The AppImage under a clean `ubuntu:24.04` container with only `libgtk-4-1 libwebkitgtk-6.0-4` installed, under Xvfb → not run: no working Docker daemon in this environment (`docker info` cannot reach `/var/run/docker.sock` or the Desktop backend, and there is no sudo to start one); see the task file's own criterion note for a local, non-containerized sanity check that ran instead
- [x] `./target/release/idle-manager --veloapp-install /tmp/veloinstall-test` → exits 0 in `0.036s` with no output; `stat -c '%y %n' ~/.config/idle-manager/*.toml` before and after shows unchanged timestamps (last modified 2026-09-14/09-20/09-24 14:14, all before this run)
- [x] Edit `crates/idle-manager-shell/Cargo.toml` to add `idle-manager-update.workspace = true`, then `./scripts/arch-check.sh` → exits 1, printing `arch-check: idle-manager-shell must not depend on idle-manager-update (docs/architecture.md)` (both the native and the `--target x86_64-pc-windows-msvc` pass); the edit was then reverted and `./scripts/arch-check.sh` → `arch-check: layer boundaries hold`
- [x] `cargo deny check` with `deny.toml`'s `RUSTSEC-2024-0388` entry removed → `error[unmaintained]: \`derivative\` is unmaintained` naming `velopack → idle-manager-update → idle-manager`, `advisories FAILED`; with `CDLA-Permissive-2.0` removed instead → `error[rejected]: failed to satisfy license requirements` naming `webpki-roots v1.0.9` reached through `ureq → velopack → idle-manager-update → idle-manager`, `licenses FAILED`; with both restored → `advisories ok, bans ok, licenses ok, sources ok`
- [x] `make windows-check` (with `clang-cl`/`lld-link` on `PATH`) → runs `cargo xwin clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- --deny warnings`, ends `Finished \`dev\` profile [unoptimized + debuginfo] target(s)`, no warnings; `make verify` (same `PATH`) → exits 0, `Summary [...] 553 tests run: 553 passed, 1 skipped`, ending `arch-check: layer boundaries hold` then `roadmap tables are up to date` (reuses the Setup step above)

## 04 — Signing, checksums and the verifier

- [x] `apt-get download minisign` then `dpkg-deb -x minisign_0.12-1build1_amd64.deb ~/.local/minisign-tmp` and copying `usr/bin/minisign` to `~/.local/bin/minisign` (no sudo, apt has no root-owned install step) → `minisign -v` prints `minisign 0.12`
- [x] `./scripts/system-check.sh minisign` with `~/.local/bin` off `PATH` → exits 1, printing `system-check: missing minisign` followed by `  apt install minisign`; with `~/.local/bin` on `PATH` → exits 0, printing `system-check: minisign present`
- [x] `minisign -G -W -p release/minisign.pub -s ~/.config/idle-manager-release/minisign.key` → `The secret key was saved as /home/lucas-gomide/.config/idle-manager-release/minisign.key`, `The public key was saved as release/minisign.pub`; `chmod 600 ~/.config/idle-manager-release/minisign.key` → `ls -l ~/.config/idle-manager-release/minisign.key` shows `-rw-------`; the secret key file lives only there, outside the repository, and is never printed or committed
- [x] `cargo test -p idle-manager-update` → `running 4 tests` (`a_package_signed_with_the_release_key_verifies_and_survives`, `a_signature_from_a_different_key_is_rejected_and_the_package_is_deleted`, `a_package_changed_by_one_byte_is_rejected_and_deleted`, `a_missing_signature_file_is_reported_by_its_path`), `test result: ok. 4 passed; 0 failed; 0 ignored`
- [x] `make windows-package` and `make linux-package` (reusing task 03's own steps above; `DOTNET_ROOT=~/.local/dotnet` on `PATH` was needed for `vpk` to find its .NET runtime in this shell) → both end the same way task 03 recorded, writing the ten files under `dist/releases/win/` and `dist/releases/linux/`
- [x] `MINISIGN_SECRET_KEY_FILE=~/.config/idle-manager-release/minisign.key make release-sign` → writes one `.minisig` beside every one of the ten files under `dist/releases/*/*`; running it again prints `release-sign: <file> is already signed` for all ten and writes no new file; `minisign -V -p release/minisign.pub -m <file> -Q` passes for each of the ten, printing `timestamp:...\tfile:<name>\thashed`
- [x] `make release-sign` with `MINISIGN_SECRET_KEY_FILE` unset → exits 1, printing `release-sign: set MINISIGN_SECRET_KEY_FILE to the secret key's path`
- [x] `make release-checksums` → writes `dist/releases/SHA256SUMS` with twenty lines, one per asset and its `.minisig`; `(cd dist/releases && sha256sum -c SHA256SUMS)` → all twenty lines print `: OK`
- [x] `make verify` (`LLVM_BIN=~/.local/llvm21/usr/lib/llvm-21/bin`, `DOTNET_ROOT=~/.local/dotnet` and `~/.local/bin` on `PATH`) → exits 0, `Summary [...] 557 tests run: 557 passed, 1 skipped`, ending `arch-check: layer boundaries hold` then `roadmap tables are up to date` (reuses the Setup step above)

## 05 — The release workflow

- [x] `python3 -c 'import yaml; yaml.safe_load(open(".github/workflows/release.yml"))'` → no error, confirming `release.yml` is well-formed YAML
- [x] `actionlint .github/workflows/release.yml` (v1.7.12) → exit 0, no findings
- [x] A local bare repo as `origin` in a throwaway clone of the working tree (so a real push/tag never reaches GitHub), reusing the already-signed `dist/releases/*` from task 04's own run: `make -s release-version` → `0.1.0`; `make release-prepare` → exits 0, leaves `CHANGELOG.md` changed (`Cargo.toml`/`Cargo.lock` unchanged because the version did not move — no tag exists yet in this repository); `make -s release-notes > notes.md` → the same `## [unreleased]` `### New`/`### Fixed` body task 02 already pinned
- [x] On that clone: `git commit -am "chore(release): v0.1.0 [skip ci]"`, `git tag v0.1.0`, `git push --follow-tags origin HEAD:main` (the workflow's exact commands) → the commit changes only `CHANGELOG.md` (the version case above), the tag is created, and the push reaches the local bare remote, never GitHub
- [x] The workflow's "Publish the release" step body, extracted verbatim and run against a stub `gh` on `PATH` that logs every call: the push path calls `gh release create v0.1.0 --draft --title v0.1.0 --notes-file notes.md` followed by every file under `dist/releases/*/*` and `dist/releases/SHA256SUMS`, then `gh release edit v0.1.0 --draft=false`; `workflow_dispatch` with `gh release view` failing (release deleted) takes the same create-then-edit branch; with `gh release view` reporting `isDraft: true` it calls `gh release upload v0.1.0 --clobber <assets>` then `gh release edit v0.1.0 --draft=false`; with `isDraft: false` (already published) it prints the `::notice::` line and calls nothing further — all four branches matched the workflow's own control flow
- [ ] A push to `main` with only `docs` commits, a push carrying a `feat` commit, the real `gh attestation verify` and `minisign -V` checks against a downloaded asset, a run failing at `make release-sign` on a throwaway branch, and a `workflow_dispatch` recovery run against a real deleted release — none run: each needs a real push or a real GitHub Actions run against the actual repository, which this session must not perform (no push, no tag, no GitHub Release, no repository secret)
- [x] `make verify` (`LLVM_BIN=~/.local/llvm21/usr/lib/llvm-21/bin`, `DOTNET_ROOT=~/.local/dotnet` and `~/.local/bin` on `PATH`) → exits 0, `Summary [...] 557 tests run: 557 passed, 1 skipped`, ending `arch-check: layer boundaries hold` then `roadmap tables are up to date` (reuses the Setup step above)

## 06 — The update port, the policy and the Velopack channel

- [x] `cargo test -p idle-manager-core` → `test result: ok. 211 passed; 0 failed; 0 ignored`, including `update::tests::version_from_str_parses_dotted_numbers_with_or_without_a_leading_v`, `update::tests::a_higher_minor_number_orders_above_a_lower_one`, `update::tests::fetching_from_available_downloads_progresses_verifies_and_can_be_retried_on_rejection`, `update::tests::next_check_due_is_true_exactly_a_day_later` and every other `update::tests::*` case the acceptance criteria name
- [x] `cargo test -p idle-manager-update` → `test result: ok. 4 passed` in `src/lib.rs` (`channel::tests::a_github_repository_url_passes_validation`, `a_github_repository_url_with_a_trailing_slash_passes_validation`, `a_non_github_url_is_rejected`, `a_github_url_with_extra_path_segments_is_rejected`), `test result: ok. 1 passed` in `tests/downloading-a-release.rs` (`a_download_signed_with_a_different_key_is_rejected_and_the_package_is_deleted`, against a `FixtureServer` on `127.0.0.1` serving a `releases.linux.json` feed, a `.nupkg` and a `.minisig` signed with `tests/fixtures/wrong-key.minisig` — never the real GitHub), `test result: ok. 4 passed` in `tests/signature-rejects-what-the-key-did-not-sign.rs` (task 04's own suite, unaffected)
- [x] `cargo clippy -p idle-manager-core -p idle-manager-update --all-targets` (`LLVM_BIN=~/.local/llvm21/usr/lib/llvm-21/bin` on `PATH`) → `Finished` with zero warnings printed (the full workspace `make lint`, `cargo clippy --workspace --all-targets -- --deny warnings`, passes too, as part of the `make verify` run below)
- [x] `./scripts/arch-check.sh` → `arch-check: layer boundaries hold` (both the native pass and the `--target x86_64-pc-windows-msvc` pass; `idle-manager-update`'s own forbidden list — `idle-manager-shell`, `-store`, `-metrics`, `-remote` and every GTK/WebView2 crate — needed no change, since this task adds no new edge for it to cross)
- [x] `make verify` (`LLVM_BIN=~/.local/llvm21/usr/lib/llvm-21/bin` on `PATH`) → exits 0, `Summary [...] 578 tests run: 578 passed, 1 skipped`, `cargo xwin clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- --deny warnings` finishes with no warnings, ending `arch-check: layer boundaries hold` then `roadmap tables are up to date` (reuses the Setup step above)

## 07 — The update notice, the menu and the window wiring

- [x] `cargo test -p idle-manager-shell update_notice` → `test result: ok. 10 passed`: `idle_shows_nothing`, `checking_reads_checking_for_updates_with_no_link_or_button`, `up_to_date_names_the_running_version_with_no_link_or_button`, `available_names_the_version_with_a_link_and_an_update_button`, `downloading_shows_the_percentage_with_an_insensitive_update_button`, `verifying_reads_checking_the_download_with_an_insensitive_update_button`, `ready_names_the_version_with_a_restart_now_button`, `a_failure_with_a_version_shows_its_reason_with_a_try_again_button`, `a_check_failure_with_no_version_shows_its_reason_with_no_link_or_button` (`notice_text`'s eight arms) and `the_update_notice_template_is_readable_from_the_registered_bundle`
- [x] `cargo test -p idle-manager-shell` → `test result: ok. 162 passed; 0 failed; 0 ignored`, including the nine tests above and `the_window_stylesheet_is_readable_from_the_registered_bundle`'s new assertion that `window.css` styles `.update-notice`
- [x] `cargo build -p idle-manager` then, under `Xvfb :98 -screen 0 1280x800x24` with a throwaway `XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`XDG_CACHE_HOME` and `dbus-run-session`, `target/debug/idle-manager` on a first run (`RUST_LOG=info`) → the log reads `the update channel could not be prepared; updates are disabled` with `error=could not prepare the update channel: This application is not properly installed: Could not locate '/usr/bin/' in executable path .../target/debug/idle-manager` (`VelopackChannel::new` genuinely fails for a `cargo build` binary, exactly as the task file says it must), immediately followed by `activated; presenting the main window` and `no saved workspace; opening a first run` — the `NoUpdateChannel` fallback never stops the app (architecture rule 3)
- [x] In that same run, the launch-time automatic check surfaces as one log line and nothing on screen: `the update check failed error=could not reach the update feed: no update channel is available in this build` at `WARN`, with no corresponding message-strip or update-notice text asserted (none exists to assert against a log) — matches `**States**`'s "Automatic check finds nothing or fails: nothing on screen, one warning line in the log for a failure"
- [x] `gdbus call --session --dest org.idlemanager.IdleManager --object-path /org/idlemanager/IdleManager/window/1 --method org.gtk.Actions.DescribeAll` against that same running instance → the answer includes `'check-for-updates': (true, '', [])` alongside `'show-help-overlay'` and `'show-phone'` — the main menu's action is registered and enabled from the first launch, even offline (`**Entry**`)
- [ ] With a test release newer than the running version (a real Velopack install, `VelopackChannel::new` succeeding): the notice appearing under the header bar, `Update` downloading with the percentage rising, ending at `Version X is ready. It installs when you quit Idle Manager.` with `Restart now`, and a game still ticking in its slot — not run: needs an actual installed, Velopack-managed copy pointed at a real or fixture GitHub release, which this sandboxed `cargo build` binary is not (the point of the check above)
- [ ] `Restart now` closing the window, `sessions.toml`'s mtime updating before the process exits, the app relaunching at the new version, and every running account coming back through the start queue — not run: same reason, needs a real installed copy and an actual Velopack swap-and-relaunch
- [ ] The `☰` menu's `Idle Manager <version>` line read visually as insensitive, and the notice's exact on-screen text for `You have the latest version, <version>.` and `Could not check for updates: …` — not run: no display to screenshot in this sandbox (code standards rule 25 puts GTK's own rendered output in this file, not `cargo test`); the check above confirms the `check-for-updates` action itself is wired and enabled, which is as far as a headless D-Bus probe reaches
- [ ] A test release whose `.minisig` was made with another key ending in `The update could not be verified and was discarded.` with `Try again`, and Velopack's packages folder left holding no `.nupkg` — not run: needs a real installed copy and a real download; task 06's own `idle-manager-update` integration test already proves the channel's `download` rejects and deletes a mismatched signature (`test-script.md`'s `## 06` section above), which is the machinery this slice's window code calls unchanged
- [ ] With both a failed save and an available update, the strip sitting first and the notice directly beneath it; dismissing the notice while `Ready` hiding it and quitting still applying the update — not run: needs a real available update to reach `Ready`, and the stacking itself is a visual check (`root_box.insert_child_after` places the notice right after the strip, read in the diff, not run on screen)
- [x] `cargo fmt --check` → no diff; `cargo clippy --workspace --all-targets -- --deny warnings` → `Finished` with zero warnings
- [x] `make verify` (`LLVM_BIN=~/.local/llvm21/usr/lib/llvm-21/bin`, `DOTNET_ROOT=~/.local/dotnet` on `PATH`) → exits 0, `Summary [...] 589 tests run: 589 passed, 1 skipped`, `cargo xwin clippy --workspace --all-targets --target x86_64-pc-windows-msvc -- --deny warnings` finishes with no warnings, ending `arch-check: layer boundaries hold` then `roadmap tables are up to date` (reuses the Setup step above)

## 08 — The round trip on both systems, and the docs

No GitHub release exists yet at the time this section was written: nothing has
been pushed to `main`, `.github/workflows/release.yml` has never run for
real, and the Releases page is empty. Every step below that needs a real
release therefore stayed unrun, and every `(manual)` box this task's own
acceptance criteria name stays unticked. What follows is the exact, hand-run
walk the owner runs once two real releases exist, plus what this session
could and did check without one.

### Windows — in the project's Windows 11 VM (`docs/windows-vm.md`)

- [ ] Publish the first real release (`v0.1.0`), unzip
      `IdleManager-win-Portable.zip` into `C:\idle-manager`
      (`docs/windows-vm.md`'s updated hand-out step), and run
      `current\idle-manager.exe` for the first time on this clean VM copy →
      Windows shows `Windows protected your PC`; click `More info`, then
      `Run anyway` — not run: needs the first real release
- [ ] Add one account and log into it, then add three more and log into each,
      so four accounts are live at once → all four show as running in the
      sidebar — not run: needs the release above
- [ ] `dir /s %APPDATA%\idle-manager > before.txt` → captures every file's
      name, size and timestamp before the update — not run
- [ ] Publish the second real release (`v0.1.1` or whatever version the real
      commit history gives, so long as it is newer than the first), then in
      the running app either wait for the daily check or use `☰` →
      `Check for updates` → the notice reads `Version <n> is available.` with
      a `What's new` link and an `Update` button — not run: needs the second
      real release
- [ ] Press `Update` → the line reads `Downloading version <n>… <percent>%`
      with every one of the four accounts still visibly running, ending
      `Version <n> is ready. It installs when you quit Idle Manager.` with a
      `Restart now` button — not run
- [ ] Start a stopwatch, press `Restart now` → the window closes; stop the
      stopwatch the moment the process is gone from Task Manager, and record
      the interval against the 60 s `Update.exe` kills at — not run: this is
      the roadmap item's own open Blocker ("`Update.exe` kills the app 60 s
      after asking it to exit"), never timed with four live accounts before
      this task
- [ ] The app relaunches → `☰` reads `Idle Manager <n>` (the newer version,
      insensitive) and every one of the four accounts comes back logged in
      through the start queue, one at a time — not run
- [ ] `dir /s %APPDATA%\idle-manager > after.txt`, `fc before.txt after.txt`
      → the only difference is `sessions.toml`'s own size or timestamp line;
      every account's own folder, cookies and login are otherwise identical
      — not run
- [ ] With Smart App Control off (the VM's own default, per
      `docs/windows-vm.md` "What the VM can and cannot prove"), confirm
      `Update.exe` itself was not blocked during the swap above (no
      SmartScreen prompt on the helper, only on the first unzip-and-run) —
      not run; Smart App Control's own refusal (the roadmap item's other open
      Blocker) cannot be shown in this VM at all, on or off, without turning
      it on and accepting the VM stops proving anything else in the same run

### Linux — a clean `ubuntu:24.04` container

- [ ] `docker run --rm -it ubuntu:24.04`, `apt update && apt install -y
      libgtk-4-1 libwebkitgtk-6.0-4 xvfb`, copy in the first real release's
      `IdleManager.AppImage`, `chmod +x` it, then under `Xvfb :99 -screen 0
      1280x800x24` with `RUST_LOG=idle_manager=debug` run it → the log shows
      the window activating with no missing-library error — not run: no
      working Docker daemon in this environment (no sudo, no Docker socket;
      the same limitation task 03's own test-script entry above already
      recorded for its own container check)
- [ ] Add one account and log into it → the sidebar shows it running — not
      run, same reason
- [ ] `sha256sum IdleManager.AppImage` and `cp -a ~/.config/idle-manager
      ~/.local/share/idle-manager /tmp/before` → captures the running
      version's checksum and a copy of both data folders before the update —
      not run
- [ ] Publish the second real release, then drive `☰` → `Check for updates` →
      `Update` → `Restart now` through the notice with the same
      dlopen-`XTest` synthetic-input helper earlier items' runbooks used
      under `Xvfb` (`docs/ui-redesign-runbook.md`'s driver) → the notice
      reaches `Ready` then the process exits and a new one starts — not run
- [ ] `sha256sum IdleManager.AppImage` again → differs from the checksum
      captured above and matches the second release's published checksum in
      `SHA256SUMS` — not run
- [ ] `diff -rq ~/.config/idle-manager /tmp/before/idle-manager` and the
      same for `~/.local/share/idle-manager` → no difference reported apart
      from `sessions.toml` — not run

### What this session did check without a release

- [x] Whether the AppImage needs `libfuse2` to run at all, independent of the
      still-open container check above: `file dist/releases/linux/IdleManager.AppImage`
      → `ELF 64-bit LSB pie executable, x86-64, version 1 (SYSV), static-pie
      linked, BuildID[...]`; `ldd dist/releases/linux/IdleManager.AppImage` →
      `not a dynamic executable`; on this development machine, which has
      `libfuse3` installed but no `libfuse2` package and no `libfuse.so.2` at
      all (`dpkg -l | grep libfuse2` and `ldconfig -p | grep libfuse.so.2`
      both print nothing), running the freshly-built AppImage under a fresh
      `xvfb-run` and throwaway `XDG_CONFIG_HOME`/`XDG_DATA_HOME` still worked:
      the process started, mounted its own bundled `squashfuse`
      (`vpk`'s own packaging log names the runtime
      `appimagekit-runtime-x86_64`), and wrote its usual
      `<config>/idle-manager/presets/*.toml` files — `libfuse2` is not
      needed; the roadmap item's Blocker was updated in place with this
      finding
- [x] `make linux-package` (`DOTNET_ROOT=~/.local/dotnet` on `PATH`) rerun
      fresh in this session → same shape task 03/04 already recorded:
      `[16:50:24 INF] Velopack CLI 1.2.158`, `Creating AppImage with
      appimagekit-runtime-x86_64 runtime`, ending `linux-package: wrote
      dist/releases/linux/IdleManager.AppImage,
      dist/releases/linux/IdleManager-0.1.0-linux-full.nupkg and
      dist/releases/linux/releases.linux.json (7.4M); idle-manager links GTK
      and WebKitGTK from the system`
- [x] `test -f release/README.md && test -f release/minisign.pub` (the two
      files the new root `README.md`'s `## Download` section links to and
      names) → both exist, confirming the links resolve
- [x] `make LLVM_BIN=~/.local/llvm21/usr/lib/llvm-21/bin verify`
      (`DOTNET_ROOT=~/.local/dotnet` on `PATH`) → exits 0, `Summary [...]
      589 tests run: 589 passed, 1 skipped`, ending `arch-check: layer
      boundaries hold` then `roadmap tables are up to date` — no regression
      from the documentation-only changes this section describes (reuses the
      Setup step above)
- [x] `make roadmap-check` after ticking this task's `(integration)`
      criterion and running `make roadmap-sync` → `roadmap tables are up to
      date`, confirming item 16's table reflects 1/6 criteria now met with
      the other five still open
