# Naming rules

Rules for anything naming-shaped. `project.yml` points every
`**Naming**` bullet in a roadmap item at this file.

Each rule is one imperative and one line of why. A rule with no why is a
preference, and the next person will not know whether to keep it.

Numbered, because roadmap items cite them by number — renumbering breaks the
citations, so append rather than reorder.

## Files

1. **Name every file `kebab-case.ext` with a lowercase extension** —
   `code-standards.md`, `session-grid.ui`, `keep-awake.js`, `melvor-idle.toml`.
   One convention across the repo means a path is never guessed twice.

2. **Except Rust module files: name every `.rs` file under `src/` in
   `snake_case`.** The file name becomes a module identifier and `mod
   session-grid;` is not valid Rust — the hyphen is not an identifier character,
   so this is a language constraint rather than a preference.

3. **Name an integration test file in kebab-case after the behaviour it
   covers** — `tests/parking-frees-memory.rs`. Cargo makes it a test binary, so
   the file name is what appears verbatim in failure output; a shared helper
   under `tests/common/mod.rs` stays snake_case because it is a module.

4. **Name a `.ui` template after the widget it defines** — `session-grid.ui`
   beside `session_grid.rs`. The two spellings are the same name in the two
   casings their tools require, and grep finds both from either. The one
   exception is `help-overlay.ui`, which GTK looks up by this fixed name to
   find the window's `GtkShortcutsWindow`.

## Crates and modules

5. **Name a workspace crate `idle-manager-<layer>`, matching its directory under
   `crates/`.** Cargo normalises the library target to `idle_manager_<layer>`, so
   the import path is predictable from the folder name and no `[lib] name` is
   needed.

6. **Never repeat the crate or module name inside itself** — `store::SessionFile`,
   not `store::StoreSessionFile`. The path already said it, and the caller reads
   the path.

7. **Name a GTK widget's private implementation module `imp`, inside the
   widget's own directory.** Every gtk4-rs example uses that name; a different
   one means translating upstream code before you can read it.

## Identifiers

8. **Use the casing Rust's own API guidelines use** — `snake_case` for items and
   fields, `UpperCamelCase` for types and traits, `SCREAMING_SNAKE_CASE` for
   constants. Deviating makes rustc's own lints the reviewer.

9. **Name a type after the thing, not the pattern** — `SessionStore`, not
   `SessionStoreImpl` or `SessionManager`. "Manager" and "Impl" say nothing about
   what the type owns.

10. **Name a port for the capability the domain needs and its adapter for the
    technology that provides it** — `MemoryProbe` implemented by `ProcPssProbe`.
    The domain should read as a description of the problem, never of the
    machinery.

11. **Name a function for its result, not its mechanics** — `total_memory()`, not
    `calculate_total_memory()`. Every function calculates; only some return a
    total.

12. **Give a boolean the shape of a question** — `is_live`, `has_preset`. A bare
    `live` reads as a noun at the call site and inverts badly under `!`.
