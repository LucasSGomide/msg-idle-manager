# 03 — The zoom memory port and the account state file

**Roadmap:** [09](../../roadmap/09-interactive-zoom/README.md) · **Scope:** back-end · **Depends on:** 01

## Context

Chosen sizes are worth nothing if they vanish when the application closes, so
something has to write them down. This slice builds that, and it is the first
time this application has ever written a file on an account's behalf. Everything
it keeps until now is either the game's own storage, written by the browser
engine, or the small game description files, written by hand.

Each account already owns a folder on disk holding two directories: one for the
game's cookies and storage, one for its disposable cache. This slice puts a third
thing at the top of that folder — a small text file holding only the sizes that
account's owner has actually chosen, one entry per arrangement, and nothing for
an arrangement nobody has adjusted. The names inside the file are written out in
full and in a spelling chosen for the file rather than borrowed from the code, so
renaming something in the program later cannot break a file already sitting on
somebody's disk.

The rules layer must not know any of that. It knows only that something can be
asked for an account's chosen sizes and told to store them, and this slice
defines that agreement on one side and the file-shaped answer to it on the other,
matching the two arrangements the application already has for finding an
account's folder and for reading the game descriptions.

Reading is deliberately unable to fail. A missing file is the normal state of an
account nobody has adjusted, and an unreadable one is no worse than a missing one
— both mean "nothing has been chosen". A file with one nonsensical value loses
that one value and keeps the rest, with a line in the log naming what was
dropped; the same tolerance the game description files already get. Nothing here
can stop the application from starting. Writing can fail, and when it does the
caller is told why rather than the failure being quietly swallowed, because
whoever asked for the write is the one who decides that losing a remembered size
is not worth interrupting anybody over.

## Technical details

- **Architecture** — rule 5: the port `ZoomMemory` goes in the core's `ports.rs`,
  named for the capability rather than the technology (naming rule 10), and its
  adapter `TomlZoomMemory` in `idle-manager-store`. Rule 1 is why the port exists
  at all: `scripts/arch-check.sh` forbids `serde` and `toml` in the core.
- **Back-end** — `read` never fails and returns the remembered map directly,
  exactly as `PresetCatalogue::read` returns a reading rather than a `Result`.
  `write` returns a `thiserror` enum in the core beside `ProfileError`
  (architecture rule 11, code standards rule 12) so the caller logs the failure
  with a reason rather than the adapter discarding it (code standards rule 14).
- **Architecture** — rule 7: `idle-manager-store` gains `account_state.rs` whose
  on-disk shape is its own serde record mapped to and from the domain type, with
  the three arrangements spelled as kebab-case string keys — `single`,
  `side-by-side`, `grid` — under a `zoom` table. Only the arrangements that have
  been changed appear (`FR.12.4`).
- **Back-end** — mapping back in is per key and tolerant (`FR.12.6`): a value
  `ZoomLevel::new` rejects is dropped with a warning naming the key while its
  siblings still apply, the same tolerance
  `crates/idle-manager-store/src/preset.rs:230` already gives a bad zoom in a
  game file; an unrecognised key is ignored the same way; a file that is absent,
  or will not parse at all, is an empty map and a warning, never a startup
  failure.
- **Back-end** — the file is `state.toml` at the root of the account's profile
  folder, a sibling of the `data` and `cache` directories `XdgProfileLocator`
  creates at `crates/idle-manager-store/src/paths.rs:76`. That root is not on
  `ProfileDirectories`, so `paths.rs` grows one small helper resolving an
  account's profile root from the data directory and both the locator and
  `TomlZoomMemory` derive their paths through it, written once rather than twice
  in one crate.
- **Back-end** — `TomlZoomMemory` mirrors the locator's pair of constructors: one
  reading the XDG data directory from the environment, one rooted at an explicit
  directory for tests. Writing is a plain whole-file write of the settled map,
  not an edit in place, which is what keeps a partial write from producing a
  half-valid file.
- **Testing** — integration tests under `crates/idle-manager-store/tests/`
  (architecture rule 14), the file named in kebab-case after the behaviour
  (naming rule 3), including an `insta` snapshot of a written file as the format
  contract. `toml`, `serde` and `insta` are already dependencies of this crate
  per `docs/stack.md`, so this adds no dependency and no `deny.toml` change.
- **Naming** — rules 1, 3, 6, 8, 10 fix the file name, the test file name, the
  port name and the adapter name.

## Acceptance criteria

- [x] `(integration)` a map written for two arrangements reads back as exactly
      those two arrangements and their sizes
- [x] `(integration)` the file is written as `state.toml` at the account's
      profile root, a sibling of that account's `data` and `cache` directories
- [x] `(integration)` reading an account with no `state.toml` returns nothing
      remembered and no error
- [x] `(integration)` a value outside the accepted zoom range is dropped and the
      other arrangements in the same file still read back
- [x] `(integration)` an unrecognised key under the zoom table is ignored and the
      recognised keys still read back
- [x] `(integration)` a `state.toml` that is not valid TOML at all reads as
      nothing remembered rather than failing
- [x] `(integration)` an `insta` snapshot of a written file shows only the
      changed arrangements, under a `zoom` table with kebab-case keys
- [x] `(integration)` writing into a profile root the process cannot write
      returns the write error carrying a reason, and never panics

## References

- [Roadmap item](../../roadmap/09-interactive-zoom/README.md) — the full picture,
  including the "Where an account's size comes from" diagram this slice makes
  durable
- [`docs/requirements.md`](../../requirements.md) — `FR.12.3`, `FR.12.4`,
  `FR.12.6`, `FR.12.8`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 3, 5, 6, 7, 11, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 12, 14, 15, 17, 21,
  24
- [`docs/naming.md`](../../naming.md) — rules 1, 3, 6, 8, 10
- [`docs/stack.md`](../../stack.md) — `toml`, `serde` and `insta` as existing
  dependencies of this crate

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
