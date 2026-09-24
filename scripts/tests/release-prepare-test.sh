#!/usr/bin/env bash
# Pins the behaviour of `make release-version`, `make release-notes` and
# `make release-prepare` against a throwaway clone with made-up commits, so
# that behaviour is proven before a real release ever depends on it
# (docs/tasks/16-release-pipeline-and-in-app-update/
# 02-the-version-and-the-notes-from-the-commits.md).
#
# The clone is a minimal two-crate workspace, built fresh in a temp directory
# and never touching the real repository's git history. It carries its own
# copy of this repo's `Makefile`, `cliff.toml`, `rust-toolchain.toml` and
# `scripts/release-prepare.sh`, so the assertions below run the exact targets
# a real release does, not a reimplementation of them.
set -uo pipefail

# When `make test` runs this script, MAKEFLAGS is already in the environment
# and every nested `make` call below would inherit it and print "Entering
# directory"/"Leaving directory" noise into the very output being asserted on.
unset MAKEFLAGS

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
status=0

fail() {
  echo "release-prepare-test: $1" >&2
  status=1
}

assert_eq() {
  local label=$1 expected=$2 actual=$3
  if [[ "$expected" != "$actual" ]]; then
    fail "$label: expected '$expected', got '$actual'"
  fi
}

assert_contains() {
  local label=$1 haystack=$2 needle=$3
  if [[ "$haystack" != *"$needle"* ]]; then
    fail "$label: expected to find '$needle'"
  fi
}

assert_not_matches() {
  local label=$1 haystack=$2 pattern=$3
  if grep -qE "$pattern" <<<"$haystack"; then
    fail "$label: unexpectedly matched /$pattern/"
  fi
}

# --- fixture: a two-crate workspace, seeded with the real repo's release
# tooling, tagged v0.1.0 as its first (unpublished) release ---------------

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

mkdir -p "$work/crates/a/src" "$work/crates/b/src"

cat >"$work/Cargo.toml" <<'EOF'
[workspace]
resolver = "3"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
EOF

cat >"$work/crates/a/Cargo.toml" <<'EOF'
[package]
name = "throwaway-a"
version.workspace = true
edition.workspace = true
EOF
echo 'pub fn a() {}' >"$work/crates/a/src/lib.rs"

cat >"$work/crates/b/Cargo.toml" <<'EOF'
[package]
name = "throwaway-b"
version.workspace = true
edition.workspace = true
EOF
echo 'pub fn b() {}' >"$work/crates/b/src/lib.rs"

cp "$repo_root/cliff.toml" "$work/cliff.toml"
cp "$repo_root/CHANGELOG.md" "$work/CHANGELOG.md"
cp "$repo_root/Makefile" "$work/Makefile"
cp "$repo_root/rust-toolchain.toml" "$work/rust-toolchain.toml"
mkdir -p "$work/scripts"
cp "$repo_root/scripts/release-prepare.sh" "$work/scripts/release-prepare.sh"
chmod +x "$work/scripts/release-prepare.sh"

(
  cd "$work"
  git init -q
  git config user.email "release-prepare-test@example.com"
  git config user.name "release-prepare-test"
  git add -A
  git commit -q -m "chore: seed the throwaway fixture"
  git tag v0.1.0
)

run() (
  cd "$work"
  "$@"
)

# --- criterion: docs + chore commits release nothing ----------------------

run git commit -q --allow-empty -m "docs: describe the throwaway fixture"
run git commit -q --allow-empty -m "docs: add a second note"
run git commit -q --allow-empty -m "chore: tidy the fixture tree"

version=$(run make --no-print-directory release-version)
assert_eq "release-version on docs/chore only" "" "$version"

before_status=$(run git status --porcelain)
prepare_output=$(run ./scripts/release-prepare.sh 2>&1)
prepare_rc=$?
assert_eq "release-prepare exit code on nothing to release" "1" "$prepare_rc"
assert_contains "release-prepare message on nothing to release" "$prepare_output" "nothing to release"
after_status=$(run git status --porcelain)
assert_eq "clone unchanged after a no-op release-prepare" "$before_status" "$after_status"

# --- criterion: a fix commit bumps patch, a feat commit bumps minor -------

run git commit -q --allow-empty -m "fix: repair the loading spinner for players"
version=$(run make --no-print-directory release-version)
assert_eq "release-version after one fix" "0.1.1" "$version"

run git commit -q --allow-empty -m "feat: add a daily streak counter for players"
version=$(run make --no-print-directory release-version)
assert_eq "release-version after fix and feat" "0.2.0" "$version"

# A merge commit's own subject is never conventional, so it has to disappear
# from the notes the same way a docs or chore commit does.
run git checkout -q -b topic
run git commit -q --allow-empty -m "chore: work on a topic branch"
run git checkout -q -
run git merge --no-ff -q -m "Merge branch 'topic'" topic

# --- criterion: release-notes groups feat under New, fix under Fixed, and
# carries no docs, chore or merge line -------------------------------------

notes=$(run make --no-print-directory release-notes 2>/dev/null)
assert_contains "release-notes New group" "$notes" "### New"
assert_contains "release-notes feat subject" "$notes" "Add a daily streak counter for players"
assert_contains "release-notes Fixed group" "$notes" "### Fixed"
assert_contains "release-notes fix subject" "$notes" "Repair the loading spinner for players"
assert_not_matches "release-notes excludes docs/chore/merge" "$notes" "docs|chore|Merge"

# --- criterion: release-prepare writes the version and the notes ----------

prepared_version=$(run ./scripts/release-prepare.sh)
prepare_rc=$?
assert_eq "release-prepare exit code when there is something to release" "0" "$prepare_rc"
assert_eq "release-prepare prints the version it wrote" "0.2.0" "$prepared_version"

toml_version=$(run sed -n '/^\[workspace\.package\]/,/^\[/p' Cargo.toml | grep '^version' | head -1)
assert_eq "Cargo.toml workspace.package version" 'version = "0.2.0"' "$toml_version"

lock_versions=$(run grep -A1 -E 'name = "throwaway-(a|b)"' Cargo.lock | grep -c 'version = "0.2.0"')
assert_eq "Cargo.lock lists both workspace crates at 0.2.0" "2" "$lock_versions"

changelog_head=$(run sed -n '/^## \[0.2.0\]/,/^## \[/p' CHANGELOG.md)
assert_contains "CHANGELOG.md starts with the 0.2.0 section" "$changelog_head" "## [0.2.0]"
assert_contains "CHANGELOG.md 0.2.0 section carries the feat line" "$changelog_head" "Add a daily streak counter for players"
assert_contains "CHANGELOG.md 0.2.0 section carries the fix line" "$changelog_head" "Repair the loading spinner for players"

# --- criterion: a second release-prepare on the now-dirty clone refuses ----

before_toml=$(run md5sum Cargo.toml)
before_changelog=$(run md5sum CHANGELOG.md)
second_output=$(run ./scripts/release-prepare.sh 2>&1)
second_rc=$?
assert_eq "second release-prepare exit code" "1" "$second_rc"
assert_contains "second release-prepare message" "$second_output" "uncommitted changes"
after_toml=$(run md5sum Cargo.toml)
after_changelog=$(run md5sum CHANGELOG.md)
assert_eq "Cargo.toml unchanged by the refused second run" "$before_toml" "$after_toml"
assert_eq "CHANGELOG.md unchanged by the refused second run" "$before_changelog" "$after_changelog"

# --- criterion: a breaking commit while the version is 0.x bumps minor,
# never major ---------------------------------------------------------------

breaking_work="$(mktemp -d)"
trap 'rm -rf "$work" "$breaking_work"' EXIT
cp "$repo_root/cliff.toml" "$breaking_work/cliff.toml"
cp "$repo_root/Makefile" "$breaking_work/Makefile"
(
  cd "$breaking_work"
  git init -q
  git config user.email "release-prepare-test@example.com"
  git config user.name "release-prepare-test"
  git commit -q --allow-empty -m "feat: init the throwaway fixture"
  git tag v0.2.0
  git commit -q --allow-empty -m "feat!: rework the fixture's data model"
)
breaking_version=$(cd "$breaking_work" && make --no-print-directory release-version)
assert_eq "a breaking feat while 0.x bumps minor, not major" "0.3.0" "$breaking_version"

if [[ $status -eq 0 ]]; then
  echo "release-prepare-test: all assertions passed"
else
  echo "release-prepare-test: FAILED" >&2
fi
exit $status
