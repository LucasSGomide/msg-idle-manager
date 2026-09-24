# 02 — The version and the notes from the commits

**Roadmap:** [16](../../roadmap/16-release-pipeline-and-in-app-update/README.md) · **Scope:** back-end · **Depends on:** —

## Context

Every commit in this repository already starts with a type word: `feat` for a
new feature, `fix` for a repair, `docs` for documentation, and so on. This
slice uses those words to do two jobs nobody has to do by hand: pick the next
version number, and write the release notes.

The rule for the number is simple. Since the last release, a feature commit
raises the middle number, a fix raises the last number, and a commit of any
other kind raises nothing. While the version starts with zero, a change marked
as breaking also raises the middle number, because a `1.0` has not been
promised yet. The whole project shares one version number, written once in the
root manifest, and this slice writes it there and refreshes the lock file so
the two agree.

The notes come from the same commits. Feature subjects are listed under "New",
fixes under "Fixed", speed-ups under "Faster". Documentation, housekeeping,
test and refactor commits never reach the notes, and neither do the merge
subjects. The generated section is put at the top of a changelog file and,
later, becomes the text of the release page and what the app's own "What's
new" link opens.

Because those notes go out with nobody rewriting them, the subject of a
feature or fix commit has to be written for the person playing the games, not
for the developer. This slice writes that down as a coding rule with an
example.

Three commands wrap the tool: one prints the next version or nothing, one
prints the notes, one writes both into the files. The last one refuses to run
when there is nothing to release or when the files already carry uncommitted
edits, so the release job can never bump twice. Everything is tested against a
throwaway clone with made-up commits, so the behaviour is pinned before a real
release depends on it.

## Technical details

- **Architecture** — add `cliff.toml` at the repository root: `[git]
  conventional_commits = true`, `filter_unconventional = true`, `tag_pattern =
  "v[0-9]*"`, `commit_parsers` mapping `^feat` → group `New`, `^fix` →
  `Fixed`, `^perf` → `Faster`, and `^(docs|chore|test|refactor|ci|style|build)`
  → `skip = true`; `[bump] features_always_bump_minor = true`,
  `breaking_always_bump_major = false`; `[changelog] header`, `body` and
  `trim = true` rendering `## [X.Y.Z] - YYYY-MM-DD` then one `- subject` line
  per commit under each group, scope stripped.
- **Architecture** — Makefile targets: `release-version` (`git cliff
  --bumped-version 2>/dev/null | sed 's/^v//'`, printing nothing when the
  output equals the current tag or no releasable commit exists),
  `release-notes` (`git cliff --unreleased --strip all`), `release-prepare`
  (`./scripts/release-prepare.sh`); `bootstrap`'s `cargo install` line gains
  `git-cliff`; all three listed in `.PHONY` and with `##` help lines.
- **Architecture** — `scripts/release-prepare.sh`: reads the version from
  `make release-version`, exits 1 with `nothing to release` when empty, exits 1
  when `git status --porcelain -- Cargo.toml Cargo.lock CHANGELOG.md` is not
  empty, rewrites the `version = "…"` line inside `[workspace.package]` of the
  root `Cargo.toml` with `sed` anchored to that section, runs `cargo update
  --workspace`, runs `git cliff --unreleased --tag "v$version" --prepend
  CHANGELOG.md`, and prints the version. A seeded `CHANGELOG.md` with the
  `cliff.toml` header is committed so `--prepend` has a file.
- **Architecture** — `scripts/tests/release-prepare-test.sh`, run by a new
  `make release-tools-test` target that `make test` calls: builds a temporary
  clone with `git init`, a `v0.1.0` tag, then synthetic `docs`, `chore`,
  `feat` and `fix` commits, and asserts the three targets' outputs.
- **Code standards** — `docs/code-standards.md` gains rule 30 under
  "Tooling": a `feat`, `fix` or `perf` subject names what changed for the
  person running the app, because it is published verbatim as a release note;
  before: `feat(shell): wire the pager's arrows into the window`; after:
  `feat(shell): turn pages of a workspace with the header-bar arrows`.
- **Code standards** — the Makefile comment above the three targets says why
  the version is derived rather than typed (rule 16), and `docs/stack.md`
  "Development tooling" lists `git-cliff` with its entry points.

## Acceptance criteria

- [x] `(integration)` on a clone tagged `v0.1.0` followed by two `docs` and one
      `chore` commit, `make release-version` prints nothing and
      `release-prepare` exits 1 with `nothing to release`, leaving every file
      unchanged
- [x] `(integration)` on the same clone plus one `fix` commit,
      `make release-version` prints `0.1.1`; plus one `feat` commit, it prints
      `0.2.0`
- [x] `(integration)` `make release-notes` on that clone lists the `feat`
      subject under `New` and the `fix` subject under `Fixed`, and contains no
      `docs`, `chore` or `Merge` line
- [x] `(integration)` `make release-prepare` writes `0.2.0` into
      `[workspace.package] version`, `Cargo.lock` lists every workspace crate at
      `0.2.0`, and `CHANGELOG.md` starts with a `## [0.2.0]` section holding the
      same lines `release-notes` printed
- [x] `(integration)` running `release-prepare` a second time on the now-dirty
      clone exits 1 and changes nothing
- [x] `(integration)` a `feat!:` commit while the version is `0.x` yields
      `0.3.0`, not `1.0.0`
- [x] `(integration)` `make verify` passes, with `release-tools-test` part of
      `make test`

## References

- [Roadmap item](../../roadmap/16-release-pipeline-and-in-app-update/README.md)
  — Back-end "Choosing the version and writing the notes"; Technical References
  on `git-cliff`
- [`docs/requirements.md`](../../requirements.md) — Distribution `FR.2.1`,
  `FR.2.2`, `FR.2.3`, `FR.2.4`, `FR.2.6`
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 16, 29 and
  the new rule 30
- [`docs/stack.md`](../../stack.md) — "Development tooling", "Version policy"

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
