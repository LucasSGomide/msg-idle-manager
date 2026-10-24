# 01 — The flag in the domain and the workspace file

**Roadmap:** [18](../../roadmap/18-drawing-without-the-gpu/README.md) · **Scope:** back-end · **Depends on:** —

## Context

The application runs several browser games at once, one per account, and saves
every account's settings to a workspace file after each change so they come back
after a restart. Some games make the web engine's graphics-chip drawing run away
with the machine's memory; the fix the owner chose is a per-account switch that
makes the engine draw that one account with the processor instead. This slice
gives every account that switch as a stored fact, with no screen yet.

The pure domain code, which knows nothing about web engines or files, learns
that an account may be "software rendered". It gains the flag, a question that
reads it, and one transition that sets it. Because the engine decides how a page
is drawn when the page is built, setting the flag on an account whose game is
running moves that account into the starting state, the same way changing the
keep-awake flag does; the interface will reload the page and the account goes
back to live at the first paint. Setting it on a parked account only records it.
Setting the value the account already holds changes nothing and says so, so
nothing is saved or reloaded for a no-op.

The storage code learns to write the flag into the workspace file as one more
optional key on the account's entry, and to read it back. The key is left out
when the flag is off, so a file where no account uses it is written exactly as
it is today, byte for byte, and no file format version changes.

It is its own slice because both later slices read this flag: the preset slice
copies a preset's default onto a new account, and the interface slice draws the
menu check and tells the engine. Landing it first lets those two run side by
side without touching the same files.

## Technical details

- **Architecture** — `idle-manager-core` stays free of engine and file types
  (architecture rule 1); the store's file type owns the on-disk key and maps it
  to the domain (rule 7); the shell will report intent and redraw from the
  book's answer (rule 8).
- **Code standards** — `Session` in `session.rs` gains a private
  `is_software_rendered: bool`, a public `is_software_rendered()` reader beside
  `is_muted()`, and `SessionBook::set_software_rendered(&mut self, &SessionId,
  bool) -> bool` beside `set_keep_awake`: a change on `Live` moves to
  `Starting`, other liveness is untouched, an unknown id or held value returns
  `false`.
- **Code standards** — `WorkspaceBook::set_software_rendered` in
  `workspace_book.rs` delegates to the holding book, as `set_keep_awake` does.
- **Code standards** — `Account` in `workspace.rs` gains
  `pub is_software_rendered: bool`, mapped both ways in `session.rs`;
  `SessionBook::add` and `add_from_preset` set `false` (02 changes the
  latter); every fixture building `Session` or `Account` gains `false`.
- **Code standards** — `SessionEntry` in `session_file.rs` gains
  `software_rendering: bool` with `#[serde(default, skip_serializing_if =
  "std::ops::Not::not")]`, mapped in `entry_to_account` / `account_to_entry`;
  `FORMAT_VERSION` stays 3, `LegacySessionEntry` untouched.
- **Naming** — rule 12: the domain boolean is a question,
  `is_software_rendered`; the on-disk key is `software_rendering`, beside
  `keep_awake` and `muted`.
- **Code standards** — `RemoteAccount` is not changed; the phone never sees the
  flag.

## Acceptance criteria

- [x] `(unit)` a new typed-address account is not software rendered
- [x] `(unit)` setting the value an account already holds reports no change
- [x] `(unit)` setting it on a live account reports a change and moves it to `Starting`
- [x] `(unit)` setting it on a parked account leaves it `Parked`
- [x] `(unit)` the flag survives park, unpark, a layout change and a move between workspaces
- [x] `(unit)` an unknown id reports no change
- [x] `(unit)` a `workspace()` → `restore()` round trip keeps the flag
- [x] `(integration)` a software-rendered account round-trips through `sessions.toml`, and an entry without the flag has no `software_rendering` key
- [x] `(integration)` a file with `software_rendering = true` on one entry reads back with only that account flagged, and the existing workspace snapshot test passes unchanged

## References

- [Roadmap item](../../roadmap/18-drawing-without-the-gpu/README.md) — the full picture
- [Wireframes](../../roadmap/18-drawing-without-the-gpu/wireframes/) — the menu this flag will drive
- [Architecture](../../architecture.md), [Code standards](../../code-standards.md), [Naming](../../naming.md)

## Implement with

Direct implementation, TDD from the criteria.
