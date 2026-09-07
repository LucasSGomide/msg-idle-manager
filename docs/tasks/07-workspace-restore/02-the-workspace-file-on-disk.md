# 02 — The workspace file on disk

**Roadmap:** [07](../../roadmap/07-workspace-restore/README.md) · **Scope:** back-end · **Depends on:** 01

## Context

The previous slice taught the rules layer to describe its whole arrangement as
one value and to rebuild itself from one. This slice puts that value on disk and
reads it back, in a small text file a curious user can open and edit.

The file is written in TOML, the same hand-editable format the game files
already use, and it gets its own set of types rather than being written straight
out of the program's internal ones. That separation is deliberate: the file is a
contract with people who already have one on their disk, so renaming something
inside the program must never turn into a broken file on somebody's machine. The
file carries a version number from its first line, decided here rather than
later, because the format will certainly grow and adding a version afterwards
means guessing at files that predate it. A version this build does not
understand is a refusal, not a shrug.

Every value the file holds maps to a specific choice the program can make. A
state that reads neither "running" nor "parked", or an arrangement that is not
one of the three, is a failure to read the file — never a quiet fallback to some
default, because a quiet fallback here silently rearranges somebody's work.

Writing has to survive a machine losing power mid-write, since the whole point
of this item is that the arrangement is durable. So the write goes to a
temporary file next to the real one and is then renamed over it, an operation
the filesystem completes entirely or not at all. Next to the real one matters:
a rename across two filesystems is a copy, and a copy can be interrupted.

Reading has to tell two very different situations apart. There is no file at all
— the first time anyone runs the program, which is not an error and not worth a
word to the user. And there is a file that will not parse, from a bad hand-edit
or a half-written file, which is a real problem. In that case nothing is thrown
away: the unreadable file is renamed aside and kept, and the failure says where
it went, so whoever cares can go and look at what they broke.

Finally, the file's home follows the convention rather than a preference. It is
configuration, so it sits under the configuration directory beside the game
files, not with the accounts' stored data. Nothing hardcodes that path.

## Technical details

- **Architecture** — `session_file.rs` in `idle-manager-store`, named for the
  artifact on disk that `FR.8.1` already calls the session file. Its own serde
  types mapped to and from the domain `Workspace`, never `derive(Serialize)` on a
  domain type (rule 7); a `thiserror` enum out of the crate (rule 11, code
  standards rule 12); integration tests under `tests/` (rule 14).
- **Naming** — `TomlWorkspaceStore` for the adapter, naming the technology the
  way `TomlPresetCatalogue` does while the port stays capability-named (rules 9,
  10); the on-disk shape is `SessionFile` and its entries `SessionEntry`, the
  same shape `preset.rs` uses for `PresetFile`. Snake_case module beside no
  directory of its own (rules 2, 6, code standards rule 9).
- **Back-end** — the format: a top-level `version` integer first, the active
  layout, and an array of account tables. Each account carries name, start
  address, liveness, slot or nothing for off-grid, keep-awake, zoom and the
  optional browser identity. `deny_unknown_fields`, as `PresetFile` has, so a
  mistyped key is a reported failure rather than a value silently ignored.
- **Back-end** — liveness on disk is running or parked only. `Starting` and
  `Queued` describe a moment during a session, never a saved wish, and task 01's
  `SessionBook::workspace()` has already flattened them.
- **Back-end** — every field maps from a domain type rather than from a raw
  string, so a value the domain cannot represent cannot be written, and an
  unrecognised one read back is a parse failure with its own variant — never a
  default (`FR.8.1`).
- **Back-end** — the write creates a temporary file in the target's own
  directory, writes it, then renames it over the target. Never a system
  temporary directory: a cross-filesystem rename is a copy and a copy can be
  interrupted, which is the exact failure this defends against.
- **Back-end** — the read's error enum separates the missing file, a malformed
  file and an unknown version. Malformed and unknown-version both rename the
  original aside to a kept name and carry that path on the error, so the caller
  can name it to the user; the bytes are never destroyed and never overwritten
  by the next save.
- **Back-end** — `paths.rs` gains the workspace file's location under the XDG
  configuration directory beside `presets/`, read from the environment, matching
  `presets_dir` exactly and completing `FR.8.3`'s split against the profile
  directories under the data directory.
- **Testing** — an `insta` snapshot of the written file pins the format
  including its version key; integration tests under `tests/` cover the round
  trip, the missing file, the malformed file, the unknown version and the
  quarantine, each against a temporary directory rather than the real XDG
  location, as `profile-directories-are-isolated.rs` does.

## Acceptance criteria

- [x] `(integration)` writing a workspace and reading it back yields the same
      accounts in the same order, with the same names, addresses, states, slots,
      keep-awake flags, zooms and identities, and the same active layout
- [x] `(integration)` the written file matches its `insta` snapshot, `version`
      key included
- [x] `(integration)` reading a directory that holds no workspace file reports
      the missing case, distinct from every parse failure
- [x] `(integration)` a file that is not valid TOML reports the malformed case,
      is renamed aside under the name the error carries, and its original bytes
      are still readable there
- [x] `(integration)` a file whose `version` this build does not understand
      reports its own case and is kept aside the same way, rather than being read
      as if it were current
- [x] `(integration)` a liveness value, a layout or a key the domain cannot
      represent is a parse failure naming the problem, never a default
- [x] `(integration)` a write over an existing workspace file leaves no
      temporary file behind, and the temporary it used was in the target's own
      directory
- [x] `(integration)` an account saved off-grid reads back off-grid, and one
      saved in a slot reads back in that slot
- [x] `(unit)` the workspace file resolves under the XDG configuration
      directory beside the presets folder, from the environment rather than a
      hardcoded path

## References

- [Roadmap item](../../roadmap/07-workspace-restore/README.md) — the full
  picture, including the "Saving after a change" diagram that fixes the
  write-then-rename order
- [`docs/requirements.md`](../../requirements.md) — `FR.8.1`, `FR.8.3`
- [`docs/architecture.md`](../../architecture.md) — rules 2, 5, 7, 11, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 3, 5, 9, 12, 13,
  14, 17, 21, 22, 23
- [`docs/naming.md`](../../naming.md) — rules 2, 3, 6, 9, 10
- [`docs/stack.md`](../../stack.md) — `toml` and `insta` are already
  dependencies of this crate, so the format work adds nothing to `deny.toml`

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
