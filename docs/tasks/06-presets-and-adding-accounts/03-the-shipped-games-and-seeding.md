# 03 — The shipped game files and first-run seeding

**Roadmap:** [06](../../roadmap/06-presets-and-adding-accounts/README.md) · **Scope:** back-end · **Depends on:** 02

## Context

The previous slice made the program read a folder of game files. This one puts
the first files in it. Three games ship with the program — Huntera, Baiaki Idle
and Lorvath — as three small text files kept in the repository and carried
inside the built program. The first time the program runs and finds the settings
folder is not there yet, it creates it and writes those three files into it.

After that the folder belongs to the user. Files are never overwritten, never
put back if deleted, and never updated by a later version of the program. That
rule is deliberate rather than lazy: the whole reason the games live in a folder
of editable text rather than in a list built into the program is that somebody
should be able to fix a game's address, or add a game nobody has heard of,
without waiting for a release. A program that rewrote the folder on every launch
would quietly undo the edits it invites. The cost is that a shipped file with a
mistake in it stays wrong on machines that already ran the program once, which
is the trade this design accepts.

None of the three files says anything about how the program should introduce
itself to the game. That field exists, but it is an override, not a default. The
browser engine's own identity is what works for every game tried so far, and an
earlier version of this program that claimed to be a different browser broke a
real sign-in outright — the engine cannot back the claim up and the game's bot
check refuses the mismatch. So the field ships absent from all three, with a
comment in each file explaining what it is for and when to reach for it.

Doing the writing here, inside the part of the program that already owns every
other touch of the disk, is what keeps the program installable by copying one
binary: there is no separate installation step for anyone to run or forget.

## Technical details

- **Back-end** — three files under `presets/` at the repository root, kebab-case
  like every other non-Rust file (naming rule 1): `huntera.toml` (Huntera,
  `https://huntera.com.br/`), `baiaki-idle.toml` (Baiaki Idle,
  `https://baiakidle.com/`) and `lorvath.toml` (Lorvath,
  `https://lorvath.com/`).
- **Back-end** — the three are embedded into the binary with `include_str!`, the
  same way and for the same reason `web_view.rs` embeds its injected scripts:
  this is their only reader, the read stays infallible, and nothing has to be
  installed alongside the binary.
- **Back-end** — they are written into the presets directory **only when that
  directory is missing**, from inside the body of the constructor task 02
  settled. An existing directory — even an empty one — is the user's and is
  never written to, which is also what keeps every criterion task 02 ticked
  still true. The composition root is untouched by this task, so it collides
  with nothing task 04 edits.
- **Back-end** — each file carries `name`, `url`, `zoom` and the keep-awake
  default, and a commented-out `user_agent` line with one sentence saying it is
  an override to reach for only when a game turns the engine away (`FR.10.5`).
- **Back-end** — `zoom` ships as a plain multiplier. Ship `1.0` for a game that
  reads correctly in a full window and a measured value for one that does not;
  record the reading that settled each number in the item's `test-script.md`, so
  the next person changing it knows what it was chosen against.
- **Back-end** — a presets directory that cannot be created is a
  `tracing::warn` naming the path, after which the program carries on with an
  empty catalogue rather than failing to start (code standards rules 14, 15).
  The dialog's empty state already covers what the user sees.
- **Testing** — integration tests under `crates/idle-manager-store/tests/`
  against temporary directories, plus an `insta` snapshot of one shipped file's
  text so a change to the format has to be reviewed deliberately rather than
  noticed later. `insta` is already a development dependency of this crate and
  `docs/stack.md` names it for exactly this kind of contract.

## Acceptance criteria

- [x] `(integration)` constructing the catalogue over a path that does not exist
      creates the directory and writes exactly the three shipped files into it
- [x] `(integration)` reading that freshly seeded directory yields three presets
      — Huntera, Baiaki Idle and Lorvath — sorted by display name
- [x] `(integration)` each shipped file parses into a preset carrying that game's
      address and no browser identity
- [x] `(integration)` constructing the catalogue a second time over the seeded
      directory writes nothing: a file edited by hand keeps its edit and a file
      deleted by hand stays deleted
- [x] `(integration)` a directory that already exists but is empty is left empty
      and yields no presets
- [x] `(integration)` a committed `insta` snapshot pins one shipped file's text
      exactly as it is written to disk
- [x] `(integration)` a presets directory that cannot be created leaves the
      catalogue empty and its construction successful, rather than failing

## References

- [Roadmap item](../../roadmap/06-presets-and-adding-accounts/README.md) — the
  full picture, including why the folder is the user's after the first run and
  why the browser identity ships empty
- [`docs/requirements.md`](../../requirements.md) — `FR.10.1`, `FR.10.5`
- [`docs/architecture.md`](../../architecture.md) — rules 2, 7, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 13, 14, 15, 16,
  17, 18, 21, 23
- [`docs/naming.md`](../../naming.md) — rules 1, 3
- [`docs/stack.md`](../../stack.md) — `insta` for snapshots of the on-disk format

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
