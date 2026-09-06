# 01 — Application shell and the resource pipeline

**Roadmap:** [01](../../roadmap/01-isolated-accounts-and-layouts/README.md) · **Scope:** front-end · **Depends on:** —

## Context

This slice makes the application open a window for the first time. Today the
repository holds five empty Rust libraries and a program that prints its own
name and exits. After this slice, running it opens a desktop window with a title
bar of its own, a button labelled "Add game" on the right of that bar, a linked
group of three buttons beside it that will later choose how many games are
visible at once, and, filling the rest of the window, a short line of text above
a button reading "Add your first game". None of those controls does anything
yet. They are the frame every later slice hangs behaviour on.

The other half of the slice is invisible, and it is why this goes first. The
project's rules say a widget is described in a markup file rather than assembled
line by line in code, and that those markup files are compiled into the program
itself so there are no loose files to install beside the binary. Nothing in the
repository does that compiling yet: there is no build step, no folder for the
markup, and no list telling the build which files to bundle. All of it is
created here, once. Every later slice adds a markup file to that list instead of
inventing the mechanism again, which is the whole reason this is a slice of its
own rather than the first half of a bigger one.

The empty state matters more than it looks. Someone opening the application for
the first time has no accounts, and a window that is simply blank reads as
broken software. One sentence and a button that starts the only action available
is what tells them the program is working and waiting for them.

## User experience

- **Entry** — the application window at launch. It opens empty, with a single
  "Add game" button in the header bar and nothing else.
- **Flow** — the header bar carries "Add game" on the trailing edge and the
  three layout toggles beside it, drawn as a linked group of three so it reads
  as one choice with three answers rather than three independent switches.
- **States** — **empty**: no accounts yet, so the content area is a centred
  block of one line of text and an "Add your first game" button, which is the
  same action as the header bar's.
- **New pattern** — the window and its empty state. `docs/design.md` holds no
  numbered rules yet; it owes a rule for where an empty state's action button
  sits relative to its text once this ships.

## Technical details

- **Architecture** — rule 13: add `crates/idle-manager-shell/build.rs`
  compiling `resources/` into a GResource, with
  `resources/idle-manager.gresource.xml` listing `ui/window.ui`. This is the
  first use of the pipeline, so the build step and both directories are new.
- **Architecture** — rule 12: `src/window.rs` holds the public wrapper and
  `src/window/imp.rs` the `ObjectSubclass`, with the composite template bound
  from the resource path.
- **Architecture** — rule 10: every widget call runs on the GTK main context;
  nothing in this slice spawns work off it.
- **Architecture** — rule 3: `crates/idle-manager/src/main.rs` builds the
  `gtk::Application`, asks the shell for the window on `activate`, and returns
  `anyhow::Result`. It gains a `gtk4` dependency; it must not gain a store one.
- **Architecture** — rule 11: a failure loading the resource bundle comes back
  from the shell as a `thiserror` enum and is printed by the binary.
- **Naming** — rules 1, 2 and 4: `window.ui` in kebab-case beside `window.rs`
  and `window/imp.rs` in snake_case, the template named after the widget it
  defines.
- **Code standards** — the empty state is the window's second child, so the
  later grid slice swaps between them rather than rebuilding the window.

## Acceptance criteria

- [x] `(unit)` the compiled bundle registers and a lookup of the window
      template's resource path returns its bytes, with no display server
- [x] `(manual)` `make run` opens a window whose header bar shows "Add game" on
      the trailing edge and three linked layout toggles beside it
- [x] `(manual)` with no accounts the content area shows one line of text above
      an "Add your first game" button, centred, and stays centred when the
      window is resized
- [x] `(manual)` closing the window ends the process and `make run` exits 0
- [x] `(manual)` `make verify` passes: format, clippy, tests, audit, layer
      boundaries and the roadmap check

## References

- [Roadmap item](../../roadmap/01-isolated-accounts-and-layouts/README.md) — the full picture
- [Wireframes](../../roadmap/01-isolated-accounts-and-layouts/wireframes/) — `main-window.md` describes the header bar and the empty state
- [`docs/architecture.md`](../../architecture.md) — rules 3, 10, 11, 12, 13 and the shell's folder layout
- [`docs/code-standards.md`](../../code-standards.md)
- [`docs/naming.md`](../../naming.md) — rules 1, 2 and 4
- [`docs/design.md`](../../design.md) — no numbered rules yet; this slice is one of the two patterns it owes a rule for

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
