# 02 — Reading the game files from the configuration folder

**Roadmap:** [06](../../roadmap/06-presets-and-adding-accounts/README.md) · **Scope:** back-end · **Depends on:** 01

## Context

Desktop programs keep their settings in a folder the desktop conventions set
aside for exactly that, somewhere under the user's home directory. This program
already uses the neighbouring folder — the one for a program's *data* — to give
each game account its own private storage. This slice claims the settings folder
too, and puts a `presets` directory inside it: one small text file per game.

The files are written in TOML, a plain format of keys and values that a person
can edit in any text editor. This slice is the part of the program that reads
that directory and turns each file into the record of a game the logic layer
learned about in the previous slice.

Reading is deliberately forgiving. Idle games are a long tail and the point of
keeping the folder in plain text is that a user adds their own by copying a file
and editing five lines — which means somebody will eventually mistype one. A
file that will not parse, or is missing something it needs, must not take the
rest of the folder down with it. So the reader hands back two lists: the games
it understood, and, for each file it could not use, that file's name and one
line saying what was wrong. It also reports which folder it looked in, so a
screen can name it when there is nothing to show.

The shape of the file gets its own types inside this part of the program, kept
separate from the types the logic layer uses, even though the two currently
carry the same five things. The folder is a contract with users who have been
invited to edit it by hand: a rename inside the program must never turn into a
broken file on somebody's disk, and the file format will grow fields the logic
layer has no interest in.

## Technical details

- **Architecture** — `preset.rs` in `idle-manager-store` implementing the
  catalogue port as `TomlPresetCatalogue`, the adapter named for the technology
  where the port is named for the capability (rule 5, naming rule 10).
- **Architecture** — the on-disk shape gets its own serde types mapped to and
  from the domain types (rule 7). Not theoretical here: the format gains fields
  over time and the domain `Preset` will not match it one-for-one for long.
- **Back-end** — extend `paths.rs` with the presets directory under the XDG
  **config** directory (`ProjectDirs::config_dir`), beside the profiles item 01
  put under the XDG **data** directory (`FR.8.3`); item 07 completes the split
  with the session file. Mirror the existing locator's shape — `new()` reading
  the environment, `under(path)` for tests that must not touch the real
  location.
- **Back-end** — the constructor's signature is settled here, `new()` and
  `under(path)`, because task 03 lands first-run seeding inside its body and
  must not have to change the composition root to do it.
- **Back-end** — read `*.toml` files in that directory only, not recursively,
  and sort the result by display name. Anything with another extension is
  ignored outright — neither a game nor a failure — so a stray note or backup
  file in the folder never produces a line on screen.
- **Back-end** — a file that will not parse, is missing a required key, or
  carries a zoom outside the accepted range becomes a failure entry naming the
  file and the reason, never an error that hides the rest of the folder. The
  reason is a `thiserror` enum, so a caller can tell the cases apart (code
  standards rule 12).
- **Back-end** — `zoom` is a plain multiplier, `zoom = 0.8`, exactly what the
  engine takes; the file expresses no screen arithmetic. `user_agent` is
  optional and absent means the engine's own identity (`FR.10.5`); the key
  present but empty is a failure entry, not a silently empty identity.
- **Testing** — integration tests under `crates/idle-manager-store/tests/`,
  kebab-case, named for the behaviour, each against its own temporary directory
  (architecture rule 14, naming rules 1, 3). `toml` 1.1 and `serde` 1.0 are
  already workspace dependencies of this crate, so this adds no dependency and
  no `deny.toml` change.

## Acceptance criteria

- [x] `(integration)` a directory holding three well-formed files yields three
      presets, sorted by display name
- [x] `(integration)` a file's display name, address, zoom and keep-awake default
      each reach the domain preset unchanged
- [x] `(integration)` a file with no `user_agent` key yields a preset carrying no
      browser identity, and a file whose `user_agent` is empty is a failure entry
      instead
- [x] `(integration)` a file whose TOML will not parse is reported as a failure
      naming that file and the reason, and every other file in the directory
      still yields a preset
- [x] `(integration)` a file missing a required key is reported as a failure the
      same way
- [x] `(integration)` a file whose zoom is zero or negative is reported as a
      failure rather than yielding a preset
- [x] `(integration)` a directory that exists and is empty yields no presets and
      no failures
- [x] `(integration)` a file whose extension is not `.toml` yields neither a
      preset nor a failure
- [x] `(integration)` the catalogue reports the directory it read, and that
      directory sits under the XDG config directory rather than the data
      directory

## References

- [Roadmap item](../../roadmap/06-presets-and-adding-accounts/README.md) — the
  full picture, including the Technical References on why the format needs its
  own types
- [`docs/requirements.md`](../../requirements.md) — `FR.8.3`, `FR.10.1`,
  `FR.10.5`
- [`docs/architecture.md`](../../architecture.md) — rules 2, 5, 7, 11, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 3, 5, 7, 12, 13,
  14, 15, 17, 21, 23
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 3, 6, 9, 10
- [`docs/stack.md`](../../stack.md) — `toml` 1.1 and `serde` 1.0 in `store` only

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
