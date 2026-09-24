#!/usr/bin/env bash
# Bumps the workspace version and prepends the release notes, ready for the
# release job to tag and publish.
#
# The version is never typed in: `make release-version` (backed by
# `cliff.toml` and the conventional-commit prefixes every commit already
# carries) is the only source of it, so a human cannot pick a number the
# commit history disagrees with (docs/code-standards.md rule 16). This script
# is the one place that actually writes it down, and refuses to run twice on
# the same unreleased state so the release job can never bump past what the
# commits justify.
set -euo pipefail

version=$(make --no-print-directory release-version)
if [[ -z "$version" ]]; then
  echo "release-prepare: nothing to release" >&2
  exit 1
fi

if [[ -n "$(git status --porcelain -- Cargo.toml Cargo.lock CHANGELOG.md)" ]]; then
  echo "release-prepare: Cargo.toml, Cargo.lock or CHANGELOG.md already has uncommitted changes" >&2
  exit 1
fi

# Anchored to the `[workspace.package]` table so a dependency pinned to the
# same version string (`toml = "1.1"`, say) is never touched — only the first
# `version = "…"` line between that header and the next `[…]` one is ours.
sed -i '/^\[workspace\.package\]/,/^\[/{s/^version = ".*"/version = "'"$version"'"/}' Cargo.toml

cargo update --workspace
git cliff --unreleased --tag "v$version" --prepend CHANGELOG.md

echo "$version"
