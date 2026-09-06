# 03 — Profile directories port and its XDG adapter

**Roadmap:** [01](../../roadmap/01-isolated-accounts-and-layouts/README.md) · **Scope:** back-end · **Depends on:** 02

## Context

Every game account needs a private folder on the user's disk. That is where the
site's cookie lives, along with the storage the game saves progress in, its
offline databases and its cached files. Two accounts of the same game are only
strangers to each other because they were handed two different folders. Nothing
else keeps them apart.

This slice decides where those folders are and creates them. On Linux there is a
convention for where an application keeps a user's data — an environment
variable names the directory, and applications that ignore it and write a hard
coded path are the ones that break when someone moves their home directory or
runs the program somewhere unusual. So the location is read from that
convention, never written into the source, and each account gets a data folder
and a cache folder named after the identifier the program minted for it. Naming
them after the identifier rather than the display name is what lets someone
rename an account later without a single file moving.

The slice is split in two on purpose. The part of the program that decides
things is not allowed to know about files at all, and the part that draws the
window is not allowed to know where files are kept. So the deciding half
declares only what it needs — "give me the two folders for this account, and
make them if they are not there" — as a named capability with no implementation,
and a separate half fulfils it using the Linux convention. The one place that
knows those two halves belong together is the program's start-up code, which
builds the real implementation and hands it to the window. A build check fails
if any other part of the program reaches across that line.

Creating a folder can fail, and the two ways it fails need telling apart: the
folder could not be created at all, or it is there but cannot be written to.
Both are reported as distinct, matchable failures rather than one vague error.

## Technical details

- **Architecture** — rules 5 and 6: `ProfileLocator` in
  `crates/idle-manager-core/src/ports.rs` returns a data directory and a cache
  directory for a `SessionId` and creates them if absent; the trait names the
  capability, the adapter names the technology.
- **Architecture** — rule 1: the port introduces no filesystem dependency into
  the core; it is a trait over paths only.
- **Architecture** — rule 3: `crates/idle-manager/src/main.rs` is the only place
  that knows the locator is XDG-backed; it constructs it and hands it to the
  shell, which depends on the trait and not on the store crate.
- **Architecture** — `XdgProfileLocator` lives in
  `crates/idle-manager-store/src/paths.rs`, using the `directories` crate, with
  session profiles under the XDG data directory. The session file and the config
  directory belong to a later roadmap item and are not written here.
- **Architecture** — rules 11 and 14, code standards rule 12: failures are a
  `thiserror` enum distinguishing "could not create" from "exists but not
  writable", covered by integration tests under `crates/idle-manager-store/tests/`
  against a temporary directory.
- **Naming** — rules 3 and 10: the integration test file is kebab-case named
  after the behaviour it pins; the port names the capability and the adapter the
  technology.

## Acceptance criteria

- [x] `(integration)` locating directories for a new identifier creates a data
      directory and a cache directory under the XDG data root and returns both
      paths
- [x] `(integration)` locating twice for the same identifier succeeds both times
      and returns the same two paths
- [x] `(integration)` two different identifiers get two different data
      directories
- [x] `(integration)` with the XDG data root pointed at a temporary directory,
      every returned path is inside it — no path is hardcoded
- [x] `(integration)` a data root that exists but cannot be written returns the
      not-writable variant, distinct from the could-not-create variant
- [x] `(integration)` `make arch-check` passes: the core reaches no filesystem
      crate and the shell reaches no store crate
- [x] `(manual)` `make run` builds the locator in the binary, hands it to the
      shell and still opens the window

## References

- [Roadmap item](../../roadmap/01-isolated-accounts-and-layouts/README.md) — the full picture, including the "Adding a game account" diagram where the locator is called
- [`docs/architecture.md`](../../architecture.md) — rules 1, 3, 5, 6, 11 and 14
- [`docs/code-standards.md`](../../code-standards.md) — rule 12
- [`docs/naming.md`](../../naming.md) — rules 3 and 10

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
