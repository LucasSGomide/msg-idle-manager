# 04 — The frame-callback shim

**Roadmap:** [04](../../roadmap/04-keep-awake/README.md) · **Scope:** front-end · **Depends on:** 03

## Context

There are two separate mechanisms slowing down a page nobody is looking at, and
the previous slice only turned off the one the browser engine lets a program turn
off. This slice deals with the other one.

Browsers hand a page a callback once per drawn frame, and that callback is how a
great many games drive their whole loop: do a little work, ask to be called again
next frame, repeat. A browser that is not drawing the page never hands the
callback out, so a game built that way does not run slowly when it is hidden — it
stops completely, and starts again the moment you look at it. The engine offers no
setting anywhere to prevent that.

The only way past it is to give the page a small piece of script that runs before
the game's own code, and to have that script quietly replace the frame callback.
While the page reports itself hidden, a request for the next frame is answered
from a plain timer instead. While the page reports itself visible, every request
goes straight to the engine's own implementation, untouched. The game asks for its
next frame exactly as it always did; something answers.

Replacing a browser function like this is a workaround for a limitation, not a
feature, and it is exactly the kind of code that becomes a mystery in a year. So
the file opens by saying which limitation forced it. The rate the timer fires at
is a genuine guess: too slow and a game counting frames loses progress, too fast
and every hidden account spends processor time. There is no measurement behind the
number yet, so whatever is chosen is written down alongside what it was tried
against.

There is one detail worth knowing about how the script gets installed. The
program can only ask the engine to run a script "before this document's own code",
and the document that is already open has run its code long ago. So adding the
script has no effect until the next load, which is why turning the setting on
reloads the page — the reload the previous slice already does is what puts this in
place. Turning the setting off removes the script and reloads for the same reason.
Removing it needs care: the only call available clears every script the page has,
including the unrelated one the program uses to forward the page's console
messages into its own logs, so that one has to be put back.

## User experience

- **Flow** — turning "keep running when hidden" on now also gives the account's
  page the replacement frame callback, in place before the game's own code runs.
  The single reload the toggle already performs is what installs it.
- **Flow** — turning it off removes it and reloads once more, back to the
  engine's ordinary behaviour for a hidden page.
- **Flow** — parking an account with the setting on and starting it again gives
  the new view the same script, because it is built from the same remembered
  setting.
- **States** — nothing new is drawn. **Off**, **on** and **reloading** render
  exactly as the previous slice left them: the menu item's check, the `Starting`
  marker during the reload, and the row's insensitive action button.

## Technical details

- **Front-end** — add `crates/idle-manager-shell/resources/js/keep-awake.js`,
  kebab-case like every non-Rust file (naming rule 1). It holds the smallest
  possible shim: while the document reports itself hidden, answer
  `requestAnimationFrame` from a timer at a fixed interval; while it reports
  itself visible, hand every request straight to the engine's own implementation.
- **Code standards** — rule 18: the file opens with a comment naming the
  constraint that forced it — the engine suspends scripted animations for a
  hidden page with no setting anywhere to prevent it — because without that line
  the file reads as a performance hack somebody should delete.
- **Code standards** — rule 5: both the interval and the identifier bookkeeping
  that lets a cancelled request actually cancel are named constants. The ids the
  shim hands out must not collide with the engine's, and `cancelAnimationFrame`
  has to cancel either kind.
- **Front-end** — the source is brought in with `include_str!` beside
  `PAGE_CONSOLE_JS` in `web_view.rs`, for the reason already recorded there: it
  is the only caller, it needs the source before any widget exists, and
  `include_str!` keeps the read infallible so the holder needs no error path for
  it. The roadmap item says GResource; the sibling script's own comment settles
  the mechanism the other way, and the two scripts stay consistent rather than
  splitting.
- **Front-end** — install through the view's `UserContentManager` as
  `UserScript::new(source, UserContentInjectedFrames::AllFrames,
  UserScriptInjectionTime::Start, &[], &[])` (`webkit6` 0.6.1,
  `src/auto/user_script.rs:22`), added with `UserContentManager::add_script`
  (`src/auto/user_content_manager.rs:42`). Document-start timing in all frames is
  what makes it work: the game's own code must find the replacement already in
  place.
- **Front-end** — turning the flag off calls `remove_all_scripts`
  (`src/auto/user_content_manager.rs:99`) and re-adds the page-console bridge
  script, because that call clears every script on the manager including the one
  item 01 installed. Split the existing `page_console_bridge` so the script set
  can be rebuilt on a manager that already exists. There is no API to apply a
  document-start script to a document already loaded, which is why
  `SessionView::set_keep_awake` ends in a reload.
- **Front-end** — the content manager is a construct property of the view
  (`src/auto/web_view.rs:163`), so the view `SessionView::start` builds is given
  the script set the remembered flag implies and an account parked with keep-awake
  on comes back with the shim already in place (`FR.6.3`).
- **Testing** — architecture rule 14 and code standards rule 25: no test may
  require a display server, and this repository has no JavaScript test harness, so
  this slice's evidence is the item's `test-script.md`.

## Acceptance criteria

- [x] `(manual)` `keep-awake.js` opens with a comment naming the constraint that
      forced it — the engine suspends scripted animations for a hidden page with
      no setting to prevent it (code standards rule 18)
- [x] `(manual)` the interval and the identifier bookkeeping are named constants,
      and the interval chosen is recorded in `test-script.md` with what it was
      tried against, since no measurement stands behind it
- [x] `(manual)` with keep-awake on, a page loaded into that account reports at
      document start that its frame-callback function is the replacement, before
      any of the page's own scripts have run
- [ ] `(manual)` with keep-awake on and the window minimised, a counter driven
      only by frame callbacks keeps advancing at roughly the shim's interval,
      where task 02 measured it frozen for 22 seconds without it. Needs a desktop
      with a window manager, since the Xvfb display cannot minimise
- [ ] `(manual)` with keep-awake off, the same counter stops while the window is
      minimised and resumes when it is restored
- [x] `(manual)` a frame request cancelled while the page is hidden never fires,
      and one requested while hidden and still pending when the page becomes
      visible fires exactly once
- [x] `(manual)` turning keep-awake off removes the shim and leaves the
      page-console bridge working — the page's own console output still reaches
      `tracing`
- [x] `(manual)` at least one real idle game is loaded with the shim in place and
      either plays normally or is recorded in `test-script.md` as rejecting the
      patched frame callback
- [x] `(manual)` nothing new is drawn: the menu item's check, the `Starting`
      marker during the reload and the row's insensitive action button render
      exactly as the previous slice left them

## References

- [Roadmap item](../../roadmap/04-keep-awake/README.md) — the full picture,
  including the "Turning keep-awake on for an account" interaction diagram and
  the Technical References for `webkit6`
- [Wireframes](../../roadmap/04-keep-awake/wireframes/) — the row settings menu
  this slice's reload is triggered from
- [`docs/requirements.md`](../../requirements.md) — `FR.6.3`
- [`docs/architecture.md`](../../architecture.md) — rules 10, 12, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 14, 18, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2
- [`docs/design.md`](../../design.md) — rules 1 and 2; nothing this slice draws
  changes, so they only have to stay true

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
