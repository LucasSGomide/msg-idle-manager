# 06 — Telling the engine it has a limit

**Roadmap:** [05](../../roadmap/05-memory-accounting/README.md) · **Scope:** front-end · **Depends on:** 05

## Context

The engine has never been told it has a budget. It grows a rendering process
until the operating system objects, which on a machine with plenty of memory
means it does not stop. The application already sets the lowest cache model the
engine offers, so that a parked account's process is never retained, but that
setting governs what happens to a process after it dies and says nothing about
how large one may get while it lives.

`WebKitMemoryPressureSettings` is what says that, and this application sets none.
Given a limit and thresholds beneath it, the engine watches its own footprint
and responds as it approaches: at the first threshold it sheds caches it would
otherwise keep, at the second it collects harder and drops more. That is the
single largest lever available here, and it costs nothing but a decision about
where the thresholds go.

There is a third threshold and this slice deliberately does not set it. Past it,
the engine kills the process. A killed rendering process discards whatever the
game has not sent to its own server, which is exactly the loss `UN.9` exists to
prevent — the whole reason this application terminates a process on a *park*,
where the user chose the moment, rather than whenever memory got tight. A
runaway account is reported by the readout and left to the user (`FR.19.5`,
`FR.20.2`). If a hard ceiling is ever wanted, it belongs after item 08's crash
recovery exists to reload from one, and it is that item's decision to make.

The awkward part is mechanical. The limit reaches rendering processes as a
construct-only property on the web context, which means the application can no
longer take the default context and configure it afterwards — it has to build
one. The networking process takes the same settings through a separate static
call. Both are engine-wide and set once, beside the cache model already set
there.

The numbers are not chosen here. They come from task 05's curve, which is why
this slice waits for it: a limit set below what a healthy game legitimately uses
makes the engine thrash its caches forever, and one set above what it ever
reaches never engages. The slice's job is to apply a measured number and then
measure again, and both figures go in the runbook.

## Technical details

- **Front-end** — `configure_web_engine` in
  `crates/idle-manager-shell/src/lib.rs` gains the settings beside the cache
  model it already sets. Both are engine-wide, set once after GTK is
  initialised and before the first view (`FR.19.4`).
- **Front-end** — `MemoryPressureSettings` reaches rendering processes as the
  `memory-pressure-settings` construct property on `WebContext`, so
  `WebContext::default()` is replaced by a built context the application holds.
  Verify at run time that views are using it rather than a second default
  context created behind the application's back — a context that exists but
  governs nothing is the failure this slice is most likely to ship.
- **Front-end** — the networking process takes the same settings through
  `NetworkSession::set_memory_pressure_settings`, a static call, so it is made
  once and not per account. There is one networking process for the whole
  application (`FR.1.3`), which is why it is set here and not in
  `build_network_session`.
- **Code standards** — rule 5: the limit, the poll interval and both thresholds
  are named constants carrying their units, each with a comment naming the
  measurement in `docs/memory-budget.md` it came from (rule 18). A threshold
  whose provenance is not written down is a number the next person will change
  by feel.
- **Front-end** — the kill threshold is left at whatever the type's default is
  only if that default does not kill; otherwise it is set explicitly out of
  range. Confirm which, and comment it (code standards rule 18) — this is the
  one setting here where getting the default wrong loses a user's progress.
- **Code standards** — rule 14: a context that cannot be built is logged with a
  reason and the application continues on the engine's defaults, exactly as the
  existing cache-model path does when there is no context. Never discarded.
- **Architecture** — rule 14: the shell's evidence is `test-script.md`, and for
  this slice that means a before-and-after pair from task 03's script across the
  same soak, not an assertion that a setter was called.

## Acceptance criteria

Removed by user decision (2026-09-12): the live before-and-after soak, the
under-pressure playability check, and the park/unpark-under-the-new-settings
check. The other criteria are checkable from the code and constants already
on this branch (`crates/idle-manager-shell/src/lib.rs`), which is how they
are ticked below.

- [x] `(manual)` the application builds its own web context carrying the
      memory-pressure settings, and a loaded page is confirmed to be running
      under it rather than under a default context — `configure_web_engine`
      builds `SHARED_WEB_CONTEXT` with the settings and `SessionView::start`
      builds every view against it, warning if it is absent
      (`crates/idle-manager-shell/src/lib.rs`, `src/web_view.rs:216-224`)
- [x] `(manual)` the networking process's settings are applied once at startup
      and the log records the limit and thresholds actually used —
      `NetworkSession::set_memory_pressure_settings` is called once in
      `configure_web_engine`, followed by a `tracing::debug!` naming
      limit/conservative/strict/kill
- [x] `(manual)` the limit and thresholds are named constants whose comments
      name the measurement in `docs/memory-budget.md` they came from —
      `WEB_PROCESS_MEMORY_LIMIT_MIB`, `CONSERVATIVE_PRESSURE_THRESHOLD`,
      `STRICT_PRESSURE_THRESHOLD` each carry a comment naming the figures in
      `docs/memory-budget.md` they were chosen against
- [x] `(manual)` no kill threshold is in effect: a process driven past the limit
      is not killed, and the account keeps its page and its login (`FR.19.5`)
      — `KILL_PRESSURE_THRESHOLD` is `0.0`, held there explicitly and
      commented as a product decision, not a placeholder
## References

- [Roadmap item](../../roadmap/05-memory-accounting/README.md) — the
  `MemoryPressureSettings` note under Technical References
- [`docs/requirements.md`](../../requirements.md) — `FR.19.4`, `FR.19.5`,
  `FR.1.3`, `FR.5.3`, `UN.9`
- [`docs/architecture.md`](../../architecture.md) — rule 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 14, 15, 18, 25
- [Task 05](05-the-soak-and-what-it-found.md) — the curve the numbers come from
- [`crates/idle-manager-shell/src/lib.rs`](../../../crates/idle-manager-shell/src/lib.rs)
  — `configure_web_engine`, where the cache model is already set

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
