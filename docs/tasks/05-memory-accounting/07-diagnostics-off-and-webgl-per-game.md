# 07 — Diagnostics off by default, WebGL per game

**Roadmap:** [05](../../roadmap/05-memory-accounting/README.md) · **Scope:** full-stack · **Depends on:** 03

## Context

Two costs are being charged to every account in every run, and neither is
something the person running the application asked for.

The first is the web inspector. Its backend is switched on unconditionally,
which was the right call while a login that would not work was the hardest
problem here and the inspector was how anyone looked at one. It is not free: the
engine instruments a page differently when something might attach to it. Nobody
is attaching to it during a week-long idle run, and the cost is paid by every
account for all of that week.

The second is louder. Every resource a page loads gets two signal handlers
attached to log a failure that almost never comes. A game that polls its server
loads resources for as long as it runs, which means this is a cost that scales
with uptime rather than with account count — the worst shape a cost can have in
an application meant to be left running for days.

Both become conditional (`FR.19.6`). A debug build keeps them, because that is
where anybody debugging is; a release run enables them by asking, through the
same environment the tracing filter already reads. Nothing is removed — the
inspector is how the next login problem gets solved, and this slice must leave
it one switch away rather than one commit away.

The third change is different in kind. WebGL is enabled for every account
whether its game draws with it or not, and a graphics context is not free. But
which games need it is a fact about games, not about this application, and the
application already has a place to keep facts about games: the preset file each
one carries, holding its address, its zoom and its browser identity. WebGL joins
them, defaulting to enabled so that no game measured or unmeasured changes
behaviour when this lands (`FR.19.7`). A game measured not to need it then gets
it turned off in its own file, tested against that game, exactly as item 06
settled the browser identity.

Hardware acceleration is explicitly not touched, and the reason is worth
recording so nobody reaches for it later. The engine's 6.0 API removed the
on-demand policy and leaves only always and never; always is the default, and
the rendering process measured on 2026-09-08 had the Mesa driver mapped into it.
The application is already accelerating wherever the machine allows, which is
what the browser it is compared against does. Turning it off would trade
rendering for memory in the one direction the comparison says is wrong.

## Technical details

- **Front-end** — `configure` in `crates/idle-manager-shell/src/web_view.rs`
  stops calling `set_enable_developer_extras(true)` unconditionally and stops
  connecting `log_resource` unconditionally. Both read one switch: on in a debug
  build, or when the environment asks in a release one.
- **Code standards** — rule 5: the environment variable's name is a named
  constant, and rule 18: the comment on the switch says what it costs, since
  that is the constraint that forced the change and the reason not to revert it.
- **Front-end** — the existing `set_enable_write_console_messages_to_stdout`
  already keys off `cfg!(debug_assertions)`; the new switch subsumes it rather
  than sitting beside it as a second, differently-spelled notion of "we are
  debugging".
- **Back-end** — `Preset` in `crates/idle-manager-core/src/preset.rs` gains a
  WebGL field, and its on-disk shape in
  `crates/idle-manager-store/src/preset.rs` gains the matching key with a
  default of enabled (architecture rule 7: the file's type is the store's, mapped
  to and from the domain type). An existing preset file without the key keeps
  today's behaviour.
- **Back-end** — the key is spelled for the file, not borrowed from the code,
  and a value that is not a boolean is tolerated the way a bad zoom already is
  at `crates/idle-manager-store/src/preset.rs` — the field falls back to its
  default with a warning naming the file, and the rest of the preset still
  loads.
- **Front-end** — `apply_account_settings` applies it beside the zoom and the
  identity it already applies, before the first `load_uri`, so a page is never
  drawn with a context it then loses.
- **Architecture** — rule 14: the core's field is a unit test, the store's
  mapping an integration test, and the engine settings are `test-script.md`.
- **Testing** — the before-and-after here is measured with task 03's script on
  the same game, which is why this slice depends on it. A claim that something
  got cheaper, with no pair of figures, is not acceptance.

## Acceptance criteria

- [ ] `(unit)` a preset carries a WebGL field and a preset that does not set it
      defaults to enabled
- [ ] `(integration)` a preset file with the WebGL key set false reads back as
      disabled, and one without the key reads back as enabled
- [ ] `(integration)` a preset file whose WebGL key holds a non-boolean falls
      back to enabled with a warning naming the file, and the file's other
      fields still load
- [ ] `(manual)` in a release build with the switch unset, right-click offers no
      Inspect Element and no per-resource lines appear in the log
- [ ] `(manual)` with the switch set, the inspector opens and the per-resource
      lines return
- [ ] `(manual)` a debug build keeps both without the switch being set
- [ ] `(manual)` an account whose preset disables WebGL loads and plays its
      game, and one whose preset enables it is unchanged from before this slice
- [ ] `(manual)` the same game measured with `make memory-report` before and
      after this slice records both figures in the runbook

## References

- [Roadmap item](../../roadmap/05-memory-accounting/README.md) — "Where the
  memory goes, and what can be done about it", and the
  `HardwareAccelerationPolicy` note under Technical References
- [`docs/requirements.md`](../../requirements.md) — `FR.19.6`, `FR.19.7`,
  `FR.6.1`, `FR.10.5`
- [`docs/architecture.md`](../../architecture.md) — rules 7, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 14, 15, 18, 24,
  25
- [`docs/naming.md`](../../naming.md) — rules 2, 8, 12
- [`crates/idle-manager-shell/src/web_view.rs`](../../../crates/idle-manager-shell/src/web_view.rs)
  — `configure`, `apply_account_settings` and `log_resource`

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
