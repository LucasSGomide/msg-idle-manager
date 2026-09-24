# Test script — 16 Releasing from GitHub, and updating from inside the app

## Setup

- [x] `make bootstrap` on a machine that already has the SDK downloads → `Summary Successfully installed cargo-deny, cargo-watch, cargo-xwin, cargo-nextest!`, `system-check: GTK 4 and WebKitGTK development files present`, both `windows-sdk-fetch` and `windows-crt-fetch` report `already present; skipping the download`
- [x] `make verify` on a clean working tree → exits 0, ending with `arch-check: layer boundaries hold` then `roadmap tables are up to date`

## Teardown

- [ ] Delete any throwaway branch pushed to `origin` only to prove a red CI run or a cache miss

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
