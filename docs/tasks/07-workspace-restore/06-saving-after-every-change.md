# 06 — Saving after every change

**Roadmap:** [07](../../roadmap/07-workspace-restore/README.md) · **Scope:** front-end · **Depends on:** 04, 05

## Context

The program can now read an arrangement back and bring it up. It still never
writes one, so everything restored is whatever happened to be in the file
already. This slice is the other half: from here on, anything the user does that
changes the arrangement is saved, without them asking and without a confirmation
to click.

Saving is never something the user requests, so it must never be something they
can get wrong. Every action that changes the arrangement — adding an account,
parking one, starting one, changing whether a game keeps running while hidden,
moving an account between slots, switching between the three arrangements — now
also asks for the arrangement to be written. Asking is not the same as writing.
A user dragging through all three arrangements in two seconds has changed things
three times and wants one file on disk, so a request starts a short wait, and
another request during that wait restarts it. A burst becomes one write.

The write itself must not make the window stutter. Writing to disk can block for
as long as the disk feels like blocking, and the window's own work all happens
on a single thread, so the write is handed to a worker and only its result comes
back to the window. That result is not thrown away. A save that failed is
exactly the bug that quietly takes a night of progress with it, so a failure is
logged with its reason and shown to the user in the strip the launch slice
added, which is why that strip was built as a reusable widget rather than a
message hardcoded for one occasion.

One case deserves special care: the user makes a change and closes the window a
moment later, while the short wait is still running. Nothing else will happen to
trigger the write, so closing has to finish it first. Without that, the very
last thing a user did before quitting is the one thing that never survives —
which would undermine the entire item.

## User experience

- **Flow** — change anything: add an account, move one, park one, change a
  background setting, switch the arrangement. The change is saved with no action
  from the user and no confirmation.
- **Flow** — close and reopen at any point and find that same state, including a
  change made a moment before closing.
- **States** — **saving**: nothing. There is no indicator, no spinner and no
  disabled control; the window stays fully responsive while a save is in flight.
- **States** — **a save that failed**: the strip under the header bar carries one
  line saying the arrangement could not be saved and why, dismissible, in the
  same place and shape the unreadable-file message uses.

## Technical details

- **Front-end** — every site that already sends an intent to the core follows it
  with a save request: the layout toggles, adding an account either way, parking
  and starting, the keep-awake toggle, and focusing a session, which reassigns
  slots. A request is cheap — it only rearms a timer — so a site in doubt asks.
- **Front-end** — the saver holds one `glib::timeout`, rearmed on each request,
  so a burst of changes collapses into a single write. The delay is a named
  constant carrying its unit (code standards rule 5).
- **Front-end** — when the timer fires it takes the value from
  `SessionBook::workspace()`, hands the write to `gio::spawn_blocking` and takes
  the result back with `glib::spawn_future_local`, so the disk never blocks the
  GTK main context and every widget touch stays on it (architecture rule 10).
- **Front-end** — a `Starting` or `Queued` account is written as running, which
  task 01 already decided in `SessionBook::workspace()`; nothing here reshapes
  the value on its way out.
- **Front-end** — the window's close request flushes a pending save before the
  window goes, so the last change before quitting is not the one change that is
  lost. A flush at close is the one place the write is allowed to be waited on.
- **Front-end** — a failed write is logged with `tracing` in fields, with the
  path and the reason, and shown in task 04's message strip. It is never dropped
  with `let _ =` and never reduced to a log line nobody reads (code standards
  rules 14, 15).
- **Design** — the failed-save message reuses the window-level message rule task
  04 wrote into `docs/design.md`, in the same place and shape as the
  unreadable-file message; if the rule needs a second sentence to cover a message
  raised mid-session rather than at launch, that sentence goes in the rule rather
  than into this widget's code.
- **Testing** — `(manual)` in this folder's `test-script.md` against the headless
  harness, with `XDG_CONFIG_HOME` pointed at a temporary tree so the real
  workspace file is never touched; `cargo test` requires no display server
  (architecture rule 14, code standards rule 25).

## Acceptance criteria

- [x] `(manual)` adding an account, parking one, starting one, toggling keep
      running when hidden, moving one between slots and switching arrangement
      each survive closing and reopening the program
- [x] `(manual)` switching quickly through all three arrangements writes the file
      once, not three times
- [x] `(manual)` a change made immediately before closing the window is in the
      file after the program exits
- [x] `(manual)` the window stays responsive during a save — the arrangement can
      be switched while a write is in flight, with no freeze and no lost change
- [x] `(manual)` a burst of changes leaves no temporary file behind in the
      configuration directory, and the workspace file parses after every one
- [x] `(manual)` with the configuration directory made unwritable, a change shows
      one line in the strip saying the arrangement could not be saved with its
      reason, logs the same with the path, and the program keeps working
- [x] `(manual)` closing the program during a restore, while accounts are still
      queued, and reopening it brings those accounts back as running rather than
      as anything else
- [x] `(manual)` the strip's message from a failed save dismisses on its close
      button and a later successful save leaves it dismissed

## References

- [Roadmap item](../../roadmap/07-workspace-restore/README.md) — the full
  picture, including the "Saving after a change" diagram, which is deliberately
  the one interaction in this item with no screen behind it
- [The launch window wireframe](../../roadmap/07-workspace-restore/wireframes/launch-window.md)
  — the strip this slice reuses for a failed save
- [`docs/requirements.md`](../../requirements.md) — `FR.8.1`, `FR.8.3`
- [`docs/design.md`](../../design.md) — rules 1, 3, and the window-level message
  rule task 04 wrote
- [`docs/architecture.md`](../../architecture.md) — rules 3, 8, 10, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 6, 7, 13, 14,
  15, 17, 25
- [`docs/naming.md`](../../naming.md) — rules 9, 11, 12

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
