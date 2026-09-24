# 09 — Proving the pipeline on GitHub

**Roadmap:** [16](../../roadmap/16-release-pipeline-and-in-app-update/README.md) · **Scope:** back-end · **Depends on:** 01, 03, 05, 07

## Context

Slices 01, 03, 05 and 07 built the verify workflow, the packages, the publish
workflow and the update notice, and proved every part a developer's machine
can prove. What is left can only be seen on GitHub itself or on a copy of the
program that a real release installed: a workflow run going green or red, the
caches coming back on a second run, a push to the main branch producing a
signed release, and the update notice driving a real download. None of it can
happen before this branch lands, because landing is what starts the first
release.

This slice holds exactly those checks, moved here unchanged from the slices
that built the work, so those slices can be accepted on what they delivered
and this one is ticked as each check is actually observed. It is its own slice
because it has no code to write, only runs to watch, and it has to wait for
the land. Task 08's round trip needs two real releases and follows it.

## Technical details

- **Architecture** — nothing new is built; a check that fails here is fixed on
  a new `fix/16-…` branch and the check is run again.
- **Code standards** — before the first land the repository must hold the
  `MINISIGN_SECRET_KEY` Actions secret (the file at
  `~/.config/idle-manager-release/minisign.key`, see `release/README.md`) and
  the workflow token must have read and write permission, or the first release
  run fails at `make release-sign`.

## Acceptance criteria

From [01](01-verifying-every-push-on-github.md) — Verifying every push on GitHub:

- [ ] `(e2e)` the first run of `ci.yml` on a pull request against `main` is
      green, and its log shows `make verify` ending with
      `arch-check: layer boundaries hold` and `roadmap tables are up to date`
- [ ] `(e2e)` a second run with an unchanged `Cargo.lock` and Makefile
      restores all four caches (`Cache restored from key …` for each) and
      downloads neither the gvsbuild zip nor the CRT package
- [ ] `(e2e)` a push that breaks formatting on a throwaway branch makes the
      workflow red at the `fmt-check` step, proving the gate blocks rather than
      reports

From [03](03-packaging-both-systems-with-velopack.md) — Packaging both systems with Velopack:

- [ ] `(e2e)` the AppImage starts on a clean `ubuntu:24.04` container with
      only `libgtk-4-1 libwebkitgtk-6.0-4` installed, under Xvfb, and the
      window template loads (the `starting idle-manager` log line appears and
      the process is alive after 10 s); if `libfuse2` turns out to be needed,
      the criterion records it and the Blocker on the roadmap item is updated
      — not run: this session has no working Docker daemon (`docker info`
      fails to reach `/var/run/docker.sock` or the Desktop backend, and there
      is no sudo to start one), so the container could not be built. A local,
      non-containerized sanity run (`xvfb-run -a
      dist/releases/linux/IdleManager.AppImage`, and the same for the
      extracted binary and for the plain non-packaged `idle-manager`) shows
      identical behaviour for all three: the `starting idle-manager` log line
      appears, then the process exits within a fraction of a second under
      this machine's Xvfb — a pre-existing headless-display limitation of
      this sandbox, not something task 03 introduced (the plain, unpackaged
      binary built before this task's changes does the same). No FUSE-related
      error appeared running the AppImage directly (this machine has
      `fuse3`/`libfuse3`, not `libfuse2`), so there is no evidence either way
      that `libfuse2` is needed; the Blocker is left as written until the
      real container test can run.

From [05](05-the-release-workflow.md) — The release workflow:

- [ ] `(e2e)` a push to `main` with only `docs` commits since the last tag runs
      `release.yml` to the version step and skips every later step, with no
      commit, tag or release created
- [ ] `(e2e)` a push to `main` carrying a `feat` commit produces, in one run: a
      `chore(release): vX.Y.Z [skip ci]` commit on `main` changing only
      `Cargo.toml`, `Cargo.lock` and `CHANGELOG.md`; a tag `vX.Y.Z`; and a
      published release whose assets are `IdleManager-win-Portable.zip`,
      `IdleManager.AppImage`, both `.nupkg` files, both `releases.*.json`
      feeds, one `.minisig` per package and `SHA256SUMS`
- [ ] `(e2e)` the release body equals the new `CHANGELOG.md` section, and no
      second `release.yml` run starts from the release commit
- [ ] `(e2e)` `gh attestation verify IdleManager.AppImage --repo <owner>/<repo>`
      passes for a downloaded asset, and `minisign -V -p release/minisign.pub
      -m <asset>` passes with the matching `.minisig`
- [ ] `(e2e)` a run made to fail at `make release-sign` (a deliberately missing
      secret on a throwaway branch set as the workflow's target) leaves `main`,
      the tag list and the Releases page unchanged
- [ ] `(e2e)` a `workflow_dispatch` run with `tag: vX.Y.Z` for a tag whose
      release was deleted rebuilds and republishes the same assets without a
      new commit or a new tag

From [07](07-the-update-notice-the-menu-and-the-window.md) — The update notice, the menu and the window wiring:

- [ ] `(manual)` with a test release newer than the running version, the notice
      appears under the header bar after launch, `Update` downloads with the
      percentage rising, and the line ends at `Version X is ready. It installs
      when you quit Idle Manager.` with `Restart now`, while a game keeps
      ticking in its slot
- [ ] `(manual)` `Restart now` closes the window, `sessions.toml`'s mtime
      updates before the process exits, the app relaunches at the new version,
      and every running account comes back through the start queue
- [ ] `(manual)` `☰` shows `Idle Manager <version>` insensitive and `Check for
      updates`; on the latest version the notice reads `You have the latest
      version, <version>.` with only `×`; offline it reads `Could not check for
      updates: …` with only `×`
- [ ] `(manual)` a test release whose `.minisig` was made with another key
      ends in `The update could not be verified and was discarded.` with
      `Try again`, and Velopack's packages folder holds no `.nupkg`
- [ ] `(manual)` with both a failed save and an available update, the strip
      sits first and the notice directly beneath it; dismissing the notice while
      `Ready` hides it and quitting still applies the update

## References

- [Roadmap item](../../roadmap/16-release-pipeline-and-in-app-update/README.md)
- [`test-script.md`](test-script.md) — the `## 09` section holds the hand-run steps
- [`release/README.md`](../../../release/README.md) — the signing key and the secret

## Implement with

No code. Run the steps in `test-script.md`'s `## 09` section after the land,
tick each criterion here as it is observed, then run `make roadmap-sync`.
