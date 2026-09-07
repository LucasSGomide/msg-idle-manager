# 02 — What the engine does to a hidden page

**Roadmap:** [04](../../roadmap/04-keep-awake/README.md) · **Scope:** front-end · **Depends on:** —

## Context

The rest of this feature rests on two facts nobody in this project has actually
checked, and both are cheap to check. This slice checks them and leaves behind
the small piece of code that makes them checkable again later.

The first fact is about the browser engine's own behaviour. A page the engine
believes nobody is looking at gets slowed down. The program deliberately keeps an
account that has no place on the screen fully laid out — parked just outside the
window's edge rather than removed — precisely so the engine does not treat it as
hidden. Whether that trick actually works has never been measured. If it does,
this whole feature only matters while the window is minimised, which is a much
smaller prize. If it does not, the feature protects the case that happens every
day. The answer changes how much everything after this is worth, so it is read
first and written down.

The second fact is a matter of vocabulary. The engine exposes a list of internal
switches, each with a short text name, and two of them are the ones this feature
needs to turn off: the one that stretches out a hidden page's timers, and the one
that suspends a hidden page's animations. Those names are not published anywhere
the project can read at build time — they come from the engine build installed on
the machine, and they can differ between versions. So the program asks the engine
for its list and prints it, once, when it starts. Somebody reads the two names off
that list, and they become named constants in the code with the engine version
they were read from recorded beside them. The printing stays, because the next
engine upgrade is when this stops being obvious again.

Nothing a user can see changes in this slice. No setting is turned off, no script
is injected, no menu appears. What it leaves behind is a start-up log of every
switch the engine offers, two constants naming the two that matter, and two
recorded measurements in the item's runbook that the following slices are built
on top of.

## User experience

- **States** — nothing visible changes. Every state the sidebar draws —
  `Current`, `Visible`, `Background`, `Parked`, `Starting`, and the empty list —
  renders exactly as it did before this slice, and every account still loads its
  game into its slot.
- **Flow** — the observable evidence is a log line and two written-down answers:
  the engine's full switch list appears once at start-up, and the runbook records
  what an out-of-sight page and a minimised window's page each report about their
  own visibility.

## Technical details

- **Front-end** — in `crates/idle-manager-shell/src/web_view.rs`, walk
  `Settings::all_features()` (`webkit6` 0.6.1, `src/auto/settings.rs:1383`) once
  at shell start-up and log each `Feature`'s `identifier()`, `name()`,
  `category()` and `is_default_value()` (`src/auto/feature.rs`) at `debug`. The
  feature list is a build-time property of the engine, so once is enough and the
  walk belongs beside the shell's other start-up work, not per view.
- **Front-end** — read the identifiers for hidden-page timer throttling and
  hidden-page CSS animation suspension off that log and add them as constants.
  They are not in the `webkit6` crate — `src/auto/feature.rs` exposes only the
  accessors — and they may differ between the engine versions `docs/stack.md`
  allows, which is the item's second blocker and is settled here.
- **Front-end** — look the two `Feature` objects up by identifier once and hold
  them in a lazily-initialised list, never searched per view: the feature list is
  a build-time property of the engine and cannot change while the application
  runs. `Feature` is a glib boxed type and is not `Sync`, so the holder is a
  `thread_local!` `OnceCell` on the GTK main context (architecture rule 10), not
  a `static OnceLock`. Task 03 is the only reader.
- **Code standards** — rule 5 names every constant, so the two identifiers are
  `SCREAMING_SNAKE_CASE` constants carrying the engine's own spelling verbatim.
  Rule 18 comments them with the constraint: the values come from the engine
  build, not from the crate, and the start-up log is how they are re-read after
  an upgrade.
- **Front-end** — a lookup that finds neither feature in this build is a
  `tracing::warn` naming the identifier that was missing, never a panic (code
  standards rules 14 and 15). The feature list is not a stable API and a future
  engine may rename or remove either; a session that runs without one switch is
  better than an application that will not start.
- **Front-end** — the item's first blocker is settled on this slice's runbook.
  With the application open and one account out of sight, read what that page
  reports about its own visibility — `document.visibilityState` and
  `document.hidden`, through the developer inspector the shell already enables or
  the page-console bridge already installed in `web_view.rs` — and record the
  answer. Then minimise the window and read the same values again for an account
  that had a slot. Both answers go in `test-script.md`.
- **Front-end** — no setting is changed and no script is added in this slice.
  `SessionView::start` builds exactly the view it built before, so a page that
  stops loading after this slice is the start-up walk's fault and nothing else.
  Architecture rules 10 and 12 are unchanged: the walk is a plain function call
  on the GTK main context at start-up, and no widget gains an `imp` member.
- **Testing** — architecture rule 14 and code standards rule 25: no test in this
  repository may require a display server, so this slice's evidence is the item's
  `test-script.md` and its criteria are `(manual)`.

## Acceptance criteria

- [x] `(manual)` starting the application with
      `RUST_LOG=idle_manager_shell=debug` logs every feature the engine build
      exposes, each with its identifier, name, category and default value, once
      for the process however many accounts are open
- [x] `(manual)` the exact identifiers for hidden-page timer throttling and
      hidden-page CSS animation suspension are read off that log, added as named
      constants in `web_view.rs`, and recorded in `test-script.md` with the
      `webkit6` and WebKitGTK versions they were read from
- [x] `(manual)` looking up an identifier this build does not expose logs one
      `tracing` warning naming it and the application still starts and loads a
      game
- [x] `(manual)` with the application open and one account out of sight, that
      page's `document.visibilityState` and `document.hidden` are read and
      recorded in `test-script.md`
- [x] `(manual)` with the window minimised, a page that held a slot reports
      itself hidden, and the reading is recorded beside the out-of-sight one —
      taken on a real desktop, since the Xvfb display has no window manager to
      answer an iconify request
- [x] `(manual)` nothing the sidebar or the grid draws changes: rows, dots,
      state words, action buttons and slots render exactly as before, and every
      account still loads its game

## References

- [Roadmap item](../../roadmap/04-keep-awake/README.md) — the full picture,
  including the "What a hidden page does" flowchart whose dotted edge this slice
  measures, and the Technical References for `webkit6`
- [`docs/requirements.md`](../../requirements.md) — `FR.6.2` names the two
  switches; `FR.6.4` is the sentence that reads two ways until this slice's
  measurement settles it
- [`docs/architecture.md`](../../architecture.md) — rules 10, 12, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 14, 15, 18, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 8
- [`docs/design.md`](../../design.md) — rule 1; nothing this slice draws changes,
  so it only has to stay true

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
