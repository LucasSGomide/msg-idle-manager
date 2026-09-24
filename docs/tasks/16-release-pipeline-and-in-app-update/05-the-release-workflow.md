# 05 — The release workflow

**Roadmap:** [16](../../roadmap/16-release-pipeline-and-in-app-update/README.md) · **Scope:** back-end · **Depends on:** 02, 03, 04

## Context

Everything before this slice is a command someone could run on their own
machine: pick the version, write the notes, build the two packages, sign them,
write the checksums. This slice strings them together into the one thing the
owner wanted: push to the main branch, and if the push carries a feature or a
fix, a release appears on GitHub with both downloads, and nobody typed a
version, a note or an upload.

A second workflow file runs on every push to main, after the verify job has
passed. It asks the version tool whether there is anything to release; when
the answer is nothing, it stops there. Otherwise it writes the version and the
notes, builds and packages Linux and Windows, signs and checksums the files,
and only then does the one thing that cannot be undone: it commits the bump
and the changelog to main, tags the commit, and pushes. Then it creates the
release as a draft, uploads every file, and flips the draft to visible, so a
running copy checking for updates can never see a release that is missing a
file. Last, it records a provenance attestation for each file, so a person can
later ask GitHub which run built what they downloaded.

Two guards keep it from chasing its own tail. The commit it pushes starts with
a subject the workflow refuses to release from, and GitHub does not start
workflows for pushes made with a workflow's own token anyway. And if something
fails after the push but before the release is visible, a manual run of the
same workflow with the tag's name rebuilds from that tag and publishes,
without bumping again.

Once this lands on main, releases are real: the first one ships a program
without the in-app updater, and the release carrying the notice widget is the
first that a running copy can install by itself.

## Technical details

- **Architecture** — add `.github/workflows/release.yml`: `on: push:
  branches: [main]` and `on: workflow_dispatch: inputs: tag`; job `release`
  on `ubuntu-24.04` with `permissions: contents: write, id-token: write,
  attestations: write`, `if: github.event_name == 'workflow_dispatch' ||
  !startsWith(github.event.head_commit.message, 'chore(release):')`;
  `concurrency: release` so two pushes never release at once.
- **Architecture** — steps, each a `make` target unless noted: checkout with
  `fetch-depth: 0` and, on dispatch, `ref: ${{ inputs.tag }}`; the same apt
  and cache steps as `ci.yml`; `make bootstrap`; `make verify`; `version=$(make
  -s release-version)` into an output (on dispatch, the tag without `v`), and
  every later step `if: steps.version.outputs.version != ''`; `make
  release-prepare` (skipped on dispatch); `make linux-package`; `make
  windows-package`; write `${{ secrets.MINISIGN_SECRET_KEY }}` to a
  `mktemp` file, `make release-sign`, delete the file; `make
  release-checksums`.
- **Architecture** — the irreversible step (push only): `git config` the
  Actions bot, `git commit -am "chore(release): v$version [skip ci]"`, `git
  tag "v$version"`, `git push --follow-tags origin HEAD:main`; then `make -s
  release-notes > notes.md` before the commit so the body is the unreleased
  section; `gh release create "v$version" --draft --title "v$version"
  --notes-file notes.md dist/releases/*/* dist/releases/SHA256SUMS`, then
  `gh release edit "v$version" --draft=false`; then
  `actions/attest-build-provenance` with `subject-path: dist/releases/**`.
- **Architecture** — on dispatch the create step first checks `gh release
  view "v$tag"`; when the release exists as a draft it uploads with `gh
  release upload --clobber` and publishes; when it is already published the
  job stops with a notice.
- **Architecture** — `docs/stack.md` "Bootstrap" gains the release flow in
  four lines; `docs/code-standards.md` rule 30's rationale names the
  workflow as the reason the subjects are published unedited.
- **Code standards** — the workflow's top comment says why the push is the
  last step before publishing and why the release is a draft first (rule 16).

## Acceptance criteria

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
- [ ] `(integration)` `make verify` passes

## References

- [Roadmap item](../../roadmap/16-release-pipeline-and-in-app-update/README.md)
  — Back-end "Publishing"; Technical References on `GITHUB_TOKEN` pushes and
  `[skip ci]`
- [`docs/requirements.md`](../../requirements.md) — Distribution `FR.2.5`,
  `FR.3.1`, `FR.3.4`, `FR.3.5`, `FR.5.2`
- [`docs/architecture.md`](../../architecture.md) — rule 4
- [`docs/code-standards.md`](../../code-standards.md) — rules 16, 29, 30

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
