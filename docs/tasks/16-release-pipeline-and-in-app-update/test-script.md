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
