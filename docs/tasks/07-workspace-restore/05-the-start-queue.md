# 05 — The start queue

**Roadmap:** [07](../../roadmap/07-workspace-restore/README.md) · **Scope:** front-end · **Depends on:** 04

## Context

Opening the program now brings back the user's whole arrangement, with every
account that was running sitting in its slot waiting its turn. Nothing takes
those turns. This slice adds the thing that does.

It brings the games back one at a time. The first account starts; only once its
page has settled does the second begin, and so on until none are waiting.
Starting six games together would spike memory as six rendering engines come up
at once, saturate the connection as six games fetch their assets, and leave the
window unusable for as long as it all takes. One at a time is gentler on the
machine and much easier to follow: the user watches their games fill in, in
order, rather than staring at a frozen window.

Each start uses the same path that starting a parked account by hand already
uses, unchanged. That is deliberate — restoring must not become a second,
subtly different way to bring an account up, because a second way is a second
set of bugs and the first one is already tested.

The hard part is deciding when one account is finished so the next may begin.
The engine reports when a page has loaded, which is the honest signal, but a
game that never finishes loading — a bad connection, a server having a bad day —
would otherwise hold up everything behind it forever. So the queue moves on for
either of two reasons: the page settled, or it has waited long enough. The
waiting time is a named constant with its unit in its name, and how long it
should be is a question for measurement against a real game rather than a
number invented here; the item records that as an open question and this slice's
runbook is where the measurement gets written down.

The queue exists only while there is something to restore. Once the last account
has come up, nothing is waiting, the queue is gone, and the window behaves
exactly as it does after any hand-started account — there is no lingering mode
to get stuck in.

## User experience

- **Flow** — the accounts that were running load one after another rather than
  together. Each row shows that it is starting, then that it is running, in
  order, while the ones behind it read as queued.
- **States** — **restoring**: exactly one account reads as starting at any
  moment; every account still waiting reads as queued, and its slot shows the
  queued panel with no button.
- **States** — **restored**: every row reads as running or parked, nothing reads
  as queued, and no panel is left standing in a slot whose game came up.
- **States** — **a game that will not load**: the queue moves on once it has
  waited long enough, so the accounts behind it come up regardless; the stuck
  account is left in the state its own load leaves it in.

## Technical details

- **Front-end** — `start_queue.rs`: a small type built from the identifiers
  `SessionBook::start_order` gives it, which starts exactly one at a time and
  holds a weak reference to the window, as every other deferred callback in this
  crate does. It owns no policy about *which* accounts come back — that came
  from the core — only when the next one may begin.
- **Front-end** — each start calls item 03's existing unpark path unchanged, so
  restoration adds no second way to bring an account up and inherits the
  starting marker, the placeholder handling and the load-changed wiring already
  tested there.
- **Front-end** — the queue advances on the view's load-finished signal or on a
  named timeout, whichever comes first, so a game that hangs cannot strand the
  ones behind it. The timeout is a constant whose name carries its unit (code
  standards rule 5) and whose value is justified by the measurement recorded in
  this folder's `test-script.md` — the roadmap item's third blocker, which asks
  what "settled" really means for an idle game and can only be answered against
  a real one.
- **Front-end** — the queue re-reads the book when a turn comes and skips an
  identifier the book no longer reports as queued, so a state that changed while
  the queue was draining is never overwritten by a start nobody asked for.
- **Architecture** — the waiting is entirely the shell's: `glib::timeout` on the
  GTK main context, with every widget call on that context and no thread or
  clock anywhere near the core (rules 9, 10).
- **Front-end** — the queue drops cleanly when the window goes away mid-drain:
  an upgrade that fails ends the queue rather than firing against a dropped
  window, matching how `start_session` and `finish_starting` already guard.
- **Testing** — the next-up decision is a pure function with unit tests at the
  foot of the file; everything else is `(manual)` in the runbook against the
  headless harness, since `cargo test` never requires a display server
  (architecture rule 14, code standards rules 24, 25).

## Acceptance criteria

- [x] `(manual)` reopening with three accounts saved as running loads them one
      after another: each reads starting then running before the next leaves
      queued, and at no moment do two read as starting
- [x] `(manual)` while the queue drains, every account still waiting shows the
      queued panel with no button in its slot, and the one in progress shows the
      starting panel
- [x] `(manual)` an account saved as parked is never started by the queue and
      reads as parked throughout the whole restore
- [x] `(manual)` once the last account is up, no row reads as queued, no panel is
      left over a loaded game, and parking and starting by hand behave exactly as
      they did before
- [x] `(manual)` an account whose page never finishes loading holds the queue no
      longer than the named timeout, after which the accounts behind it come up
      normally
- [x] `(manual)` closing the window while the queue is still draining ends the
      restore without an error in the log and starts nothing further
- [x] `(unit)` the next-up decision skips an identifier the book no longer
      reports as queued, and reports the queue finished when none remain

## References

- [Roadmap item](../../roadmap/07-workspace-restore/README.md) — the full
  picture, including the "Restoring on launch" diagram whose queue half this
  slice implements, and the third blocker on what counts as settled
- [The launch window wireframe](../../roadmap/07-workspace-restore/wireframes/launch-window.md)
  — the whole item's screen; this slice is the queue draining across it
- [`docs/requirements.md`](../../requirements.md) — `FR.8.2`
- [`docs/design.md`](../../design.md) — rules 1, 2, 4
- [`docs/architecture.md`](../../architecture.md) — rules 8, 9, 10, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 6, 7, 13, 14,
  15, 17, 24, 25
- [`docs/naming.md`](../../naming.md) — rules 2, 9, 11

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
