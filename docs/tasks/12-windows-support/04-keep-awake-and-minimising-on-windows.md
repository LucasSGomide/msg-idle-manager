# 04 — Keep-awake and minimising on Windows

**Roadmap:** [12](../../roadmap/12-windows-support/README.md) · **Scope:** front-end · **Depends on:** 03

## Context

Idle games keep playing while nobody watches them, but browsers slow pages down
to save power when their window is minimised. This application lets the user
choose, account by account, whether a game is kept awake. A kept-awake game
runs at full speed while the window is minimised. The other games are allowed
to rest, which saves processor time. On Linux that choice is made by turning
individual engine switches on or off for each account.

WebView2, the engine used on Windows, cannot do that per account. Its slowdown
switches are set once for the whole engine, and every account shares that one
engine to save memory. This slice carries the per-account choice over another
way, by telling the engine which views count as visible.

When the window is minimised, each game that is not kept awake is marked
invisible, so the engine slows it down. Each kept-awake game stays marked
visible and keeps running. The small script that keeps animation-frame
callbacks firing while nothing is drawn is injected for kept-awake games, just
as on Linux. When the window comes back, every game is marked visible again.
Changing an account's keep-awake setting while the window is minimised takes
effect at once.

Whether a game marked visible inside a minimised window really keeps its timers
at full speed has not been proven. So this slice starts by measuring it in the
Windows virtual machine, and it only relies on the approach once the measurement agrees. It is
a slice of its own because it is the one place the two engines differ in
approach, and it needs its own before-and-after measurement.

## User experience

- **Flow** — Minimise the window → accounts with keep-awake on keep running at
  full rate; the others may be slowed by the engine; restoring shows every game
  where it was.
- **Flow** — Toggling an account's keep-awake from its row menu while the window
  is minimised (for example from the taskbar preview) takes effect at once.
- **States** — The sidebar's keep-awake mark and its row menu entry are
  unchanged.
- **Pattern** — Where a row's settings live is unchanged (design rule 5); the
  keep-awake mark keeps its place on the row's trailing edge (design rule 6).

## Technical details

- **Architecture** — on Windows, `EngineView::set_background(bool)` sets
  `ICoreWebView2Controller::SetIsVisible(!background)` through `ffi.rs`. Task 01
  added it as a no-op on Linux, and it stays one there.
- **Architecture** — `window/imp.rs` connects `notify::state` on the window's
  `gdk::Toplevel` surface after realize. When `GDK_TOPLEVEL_STATE_MINIMIZED`
  turns on, it calls `set_background(true)` on every live account whose
  keep-awake is off. When it turns off, it calls `set_background(false)` on
  every live account (architecture rule 8: the window reads keep-awake from the
  session book and never stores its own copy).
- **Architecture** — `SessionView::set_keep_awake` also calls
  `set_background(minimised && !on)`, so a toggle while minimised applies at
  once. A view started while minimised gets the same call right after it is
  built.
- **Architecture** — the Windows `EngineProfile` injects `keep-awake.js` for a
  keep-awake account and reloads on toggle, mirroring Linux's
  `rebuild_script_set` step. `BROWSER_ARGS` stays one constant for the shared
  environment; per-view flags would make view creation fail.
- **Architecture** — measure first. Use a test page served with `python -m
  http.server` whose `setInterval(…, 100)` logs its tick count every second
  through `console.log`. Record the ticks per second with the window minimised,
  for a visible view and for a background view, and write them into
  `test-script.md`. Minimise the application inside Windows, never the RDP
  client, which stops the whole session drawing. If a visible view does not tick at about 10 per second,
  stop and record the finding on the roadmap item's Blockers instead of
  shipping.
- **Code standards** — the minimise decision is a pure function,
  `fn background_for(minimised: bool, keep_awake: bool) -> bool`, unit-tested
  on every platform (rules 21–25). The platform call stays in `ffi.rs`
  (rule 28 as amended).

## Acceptance criteria

- [x] `(unit)` `background_for` returns `true` only for minimised with
      keep-awake off, and `false` for the other three combinations
- [x] `(integration)` `make windows-check` and `make verify` pass
- [x] `(manual)` in the Windows VM, with the tick page in a keep-awake account and the
      window minimised for 60 s, the log shows about 10 ticks per second
      throughout
- [x] `(manual)` in the Windows VM, with the tick page in an account whose keep-awake
      is off and the window minimised for 60 s, the log shows the throttled
      rate (at most 1 tick per second)
- [x] `(manual)` in the Windows VM, turning keep-awake on for the throttled account
      while minimised brings its log back to about 10 ticks per second within
      a few seconds of the reload
- [x] `(manual)` in the Windows VM, restoring the window shows every game where it was,
      drawing normally, with no reload
- [x] `(manual)` on Linux, item 04's keep-awake test-script steps still pass

## References

- [Roadmap item](../../roadmap/12-windows-support/README.md) — Front-end
  "Keep-awake and minimising on Windows", Blocker 4
- [Wireframes](../../roadmap/12-windows-support/wireframes/) — no screen
  changes in this slice
- [`docs/requirements.md`](../../requirements.md) — Platform Support `FR.1.9`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10
- [`docs/code-standards.md`](../../code-standards.md) — rules 21–25, 28
- [`docs/design.md`](../../design.md) — rules 5, 6

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The `(manual)` steps run in the
Windows VM ([`docs/windows-vm.md`](../../windows-vm.md)).
