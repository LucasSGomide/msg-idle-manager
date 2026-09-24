# Code standard rules

Rules for anything code-shaped. `project.yml` points every `**Code standards**`
bullet in a roadmap item at this file.

Each rule is one imperative and one line of why. A rule with no why is a
preference, and the next person will not know whether to keep it.

Numbered, because roadmap items cite them by number — renumbering breaks the
citations, so append rather than reorder.

Naming is in [`naming.md`](naming.md); crate boundaries are in
[`architecture.md`](architecture.md).

## Types

1. **Make illegal states unrepresentable.** Two booleans describe four states
   when the domain has three, and the fourth becomes a bug nobody wrote.

   ```rust
   enum Liveness { Live, Parked }
   enum Visibility { InSlot(SlotId), OffGrid }
   ```

2. **Wrap every identifier in a newtype.** `SessionId(String)` and
   `PresetId(String)` are both strings to the compiler until you make them not,
   and then swapping them stops compiling instead of stopping the app.

3. **Take borrowed arguments and return owned values.** `&str` and `&[T]` let a
   caller pass what it already has; returning `String` or `Vec<T>` frees the
   caller from a lifetime it did not ask for.

4. **Derive `Debug` on every public type and `Clone` only where a caller needs
   it.** A missing `Debug` makes a test failure unreadable; a reflexive `Clone`
   hides that something is being copied on every frame.

5. **Name every constant and give it a unit.** `RESTART_DELAYS_SECS`,
   `MEMORY_BUDGET_MIB`. A bare `120` in a call reads as a mystery at the call
   site and as a different mystery in the test.

## Functions and modules

6. **Keep a function at one level of abstraction.** If you want a comment to
   mark a section inside it, that section is a function and the comment is its
   name.

7. **Return early.** `let ... else`, `?` and a guard clause keep the happy path
   at one indent; an `if` pyramid hides which branch is the normal one.

8. **Default to private.** `pub(crate)` before `pub`, and `pub` only on what the
   crate genuinely offers — every public item is a promise the whole workspace
   may start depending on.

9. **Put a module in `foo.rs` beside its `foo/` directory, never in
   `foo/mod.rs`.** Six tabs called `mod.rs` are six tabs you cannot tell apart.

10. **Group imports as std, external, internal, one blank line between.** Import
    types by name and call free functions through their module — `fs::read_to_string`
    says where it comes from, a bare `read_to_string` does not.

11. **Glob-import nothing except `gtk::prelude::*`.** Traits that must be in
    scope for method syntax are the one case where naming them adds no
    information.

## Errors

12. **Return a `thiserror` enum from a library crate.** A caller inside the
    workspace has to distinguish "no session file yet" from "the session file is
    corrupt", and a string cannot be matched on.

13. **Never use `unwrap()` outside a test.** Clippy denies it. `expect` is
    allowed only where the message states the invariant that makes the failure
    impossible: `.expect("template child set in class_init")`.

14. **Never discard a `Result` with `let _ =`.** Handle it, log it with a reason,
    or return it — a silently dropped failure is the bug that takes a night of
    idle progress with it.

15. **Log with `tracing`, in fields, not with `println!` or string
    interpolation.** `warn!(session = %id, attempt, "reload failed")` can be
    filtered and grepped; a formatted sentence cannot.

## Comments

16. **Write why, never what.** The line says what it does. Delete any comment
    that restates it — including the ones you were about to write out of habit.

17. **Give every public item a `///` that states its contract.** What the caller
    gets, what makes it fail, what it costs. Not how it is implemented; that is
    what the body is for.

18. **Comment a workaround with the constraint that forced it.** "WebKit
    suspends scripted animations with no setting to prevent it" earns its line
    forever; "loop over sessions" does not.

19. **Leave no commented-out code, no banner comments, no change history.** Git
    has all three, and it keeps them accurate.

20. **Anchor a `TODO` to a roadmap number** — `// TODO(07): per-session
    attribution`. An unanchored TODO is a decision nobody has agreed to make.

## Tests

21. **Name a test after the behaviour it pins.** `parking_frees_the_web_process`,
    not `test_park`. The name is what you read when it fails at 2am.

22. **Separate arrange, act and assert with blank lines, not comments.** Three
    paragraphs are as legible as three labels and stay true when the code moves.

23. **Assert one subject per test.** A test that checks five things reports the
    first failure and hides the other four.

24. **Keep unit tests in a `#[cfg(test)] mod tests` at the foot of the file they
    test, and cross-crate tests in `tests/`.** A unit test needs the module's
    private items; an integration test proves the public surface is enough.

25. **Never require a display server in `cargo test`.** GTK behaviour is covered
    by the item's `test-script.md`, which is run by a human with a screen.

## Tooling

26. **Run `make fmt` before committing and hand-format nothing.** Formatting
    arguments are the cheapest arguments to stop having.

27. **Fix a clippy warning rather than allowing it.** An `#[allow]` must be
    scoped to the smallest item that needs it and carry a comment saying why the
    lint is wrong here.

28. **Write no `unsafe`.** If glib's `wrapper!` or `object_subclass` expansions
    trip `unsafe_code`, scope an `#[allow(unsafe_code)]` to that `imp` module
    with a comment — never relax the workspace lint for everyone. A second
    case: COM calls, `WindowHandle::borrow_raw` and the Win32 memory calls
    roadmap item 12's Windows engine needs are `unsafe` by definition — exactly
    one module per crate, `web_engine/webview2/ffi.rs` in the shell and
    `process_tree/ffi.rs` in metrics, carries the scoped allow, with a
    `// SAFETY:` line on every `unsafe` block and only safe functions exposed.

29. **Make `make verify` pass before opening the work for review.** It is
    `fmt-check`, `lint`, `test`, `audit`, `arch-check` and `roadmap-check`, which
    is the whole standard in one command.

30. **Name a `feat`, `fix` or `perf` subject after what changed for the person
    playing the games, never for the developer.** `git-cliff` publishes it
    verbatim as a release note, and `.github/workflows/release.yml` runs
    unattended on every push to `main` — nobody proofreads or rewrites the
    subject before it goes out — before: `feat(shell): wire the pager's arrows
    into the window`; after: `feat(shell): turn pages of a workspace with the
    header-bar arrows`.

## Review checklist

- Could a state in this type never happen? Then it should not typecheck.
- Is any comment restating its line? Delete it.
- Does any public item lack a `///` contract?
- Is there an `unwrap`, a bare `let _ =`, or a `println!`?
- Does a test name describe a behaviour?
- Did a dependency cross a layer? `make arch-check` answers.
