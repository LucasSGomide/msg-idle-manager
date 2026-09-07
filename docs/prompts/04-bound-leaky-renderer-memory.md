# Goal: Keep a leaking game's renderer from growing unbounded, without needing the game's team to fix it

**Status:** not executed
**Rating:** —
**Run:** standalone. Touches `idle-manager-shell` (`web_view.rs`, `lib.rs`,
`window/imp.rs`) and probably `idle-manager-core` (`SessionBook`); no overlap
with the open zoom / polish prompts.

## Context

Measured on 2026-09-07, release build (`make release`), against trivial pages
(`example.com` / `example.org`):

| Process | RSS |
| --- | --- |
| `idle-manager` (UI / main) | ~160 MB |
| `WebKitNetworkProcess` (one, shared across all accounts) | ~70 MB |
| `WebKitWebProcess` — **one per live account** | ~175 MB for a near-blank page |

So the app's own floor is ~175 MB per running account. That is inherent: every
account has its own `WebKitNetworkSession` for login isolation (`FR.1.1`,
`FR.1.2`), and WebKitGTK gives each its own renderer — it cannot group accounts
into one process the way Chrome groups same-site tabs.

The problem is one specific game, **Huntera**: its renderer climbs steadily
(confirmed with `watch -n 30 'ps -eo pid,rss,etime,comm | grep WebKitWebProc'` —
RSS rises with no plateau while the game merely idles), reaching ~5 GB in a
single process over a session. That ~4.8 GB is Huntera's own JavaScript —
detached DOM, uncleared timers, battle/chat logs that never trim, state arrays
that only grow. **We cannot patch their code.** The root fix is theirs.

What is ours to do: bound the blast radius so a leaky game never OOMs the
machine, and does it invisibly. Idle games are server-authoritative — Huntera
saves progress on its own server continuously — so tearing a renderer down and
rebuilding it costs the user nothing but a ~2 s reload, and nothing at all for
an off-grid account.

Two knobs are currently unused:

- `enable_developer_extras(true)` is set unconditionally in `web_view.rs`
  (`configure()`). The inspector backend retains DOM / network history buffers.
  Turning it off in release builds is a small, free win — but keep it on in
  debug builds, it is how a page that will not load gets looked at, and item 08
  has not shipped its failure UI yet.
- `WebKitMemoryPressureSettings` (exposed by `webkit6` 0.6.1:
  `MemoryPressureSettings::new()`, `set_memory_limit(u32 MB)`,
  `set_conservative_threshold`, `set_strict_threshold`, `set_kill_threshold`,
  `set_poll_interval`). Applied to the network process with the static
  `NetworkSession::set_memory_pressure_settings(&mut s)` (call once, before the
  first session is built), and to renderers via
  `WebContext::builder().memory_pressure_settings(&s).build()` with views
  constructed against that context. **Caveat:** memory pressure can only
  reclaim *discardable* memory — image cache, decoded bitmaps, dead JS objects,
  offscreen backing stores. It does not touch a true leak's live objects. So it
  buys time between recycles, it does not replace them.

## What to investigate / decide

1. **Confirm the shape of the leak** with a heap snapshot, not just RSS.
   `enable_developer_extras` is already on: right-click a Huntera slot →
   Inspect → Memory → two snapshots ~10 min apart → the retained-size diff
   names what is leaking (expect "Detached HTMLDivElement ×N" or a growing
   array). Record the growth rate (MB/hour idle) in the write-up. This snapshot
   is also the bug report to hand Huntera.

2. **Pick the mitigation.** The options, roughly in order of preference:

   - **Scheduled silent recycle (preferred).** Every N hours per live account —
     or when its renderer crosses a soft RSS/PSS threshold (item 05 already
     samples PSS per process, `idle-manager-metrics`) — rebuild the view during
     a moment the account is not the focused slot: `terminate_web_process()` +
     drop the `WebView` + build a fresh one. This is exactly what park→unpark
     already does; the recycle is park+unpark with no user intent and no
     placeholder flash for an off-grid account. A `reload()` on the same view
     is **not** enough — it often does not return the old heap to the OS; the
     process has to die. The account's liveness stays `Live` throughout (it is
     not "parked", the user did not stop it) — decide whether it passes through
     `Starting` for the row marker or recycles marker-free.

   - **`kill_threshold` backstop.** Set it so WebKit kills a renderer that
     exceeds, say, 1.5–2 GB. Pairs with item 08's crash-reload (which must land
     first, or the killed renderer just stays dead). Cruder than the scheduled
     recycle — it reacts at the edge rather than pre-empting — but it is a few
     lines and a safety net even if the recycle ships.

   - **`strict_threshold` aggressive.** Lower it so WebKit dumps caches and runs
     full GC more often. Trims the non-leak portion of every renderer, leaky or
     not. Cheap, always-on, complements the above.

   - **Targeted userscript.** The app already injects document-start scripts
     (`keep-awake.js`, `page-console.js`). A per-game script with a
     `MutationObserver` could cap a known-growing Huntera subtree. It works but
     it is a patch against someone else's markup — it breaks when they change
     their HTML and it is per-game maintenance. Stopgap only; do not build
     infrastructure around it.

3. **Decide where the recycle interval / thresholds live.** They are policy and
   should be measured, not guessed (`FR.7.3` already says budget thresholds
   come "from a real measurement of at least three games"). A named constant
   with its unit (code standards rule 5), justified in a runbook, is the
   minimum. A per-account or per-preset override is a possible extension — a
   preset could carry `recycle_after_hours` the way it carries `keep_awake`.

## Constraints

1. **Respect the crate layering** (`docs/architecture.md`, `make arch-check`).
   Any timing / clock lives in the shell (`glib::timeout`), never the core
   (architecture rule 9). If the core needs to know an account was recycled
   (e.g. a `SessionBook` method), it takes the decision as an argument and owns
   no timer. The engine settings (`MemoryPressureSettings`,
   `enable_developer_extras`) live in `web_view.rs` / `lib.rs` only.
2. **Reuse the park/unpark machinery.** The recycle must go through the same
   `SessionView::stop()` → `SessionView::start()` path parking uses, so it
   inherits the terminate-then-drop ordering (`FR.5.2`) and adds no third way
   to tear a view down. `SessionView::dormant` / `start` from item 07 are the
   building blocks.
3. **A recycle is invisible for an off-grid account and near-invisible for a
   visible one.** No message strip, no modal, no lost focus. A visible slot may
   show the `Starting` placeholder briefly (item 03's, unchanged) — confirm
   against a real game that the flash is acceptable, or recycle only while the
   account is off-grid / not the focused slot.
4. **Keep-awake interaction.** A keep-awake account is running at full speed
   *because* it loses progress while throttled — recycling it still tears its
   process down for ~2 s. Decide whether keep-awake accounts are recycled on
   the same schedule, a longer one, or only at the `kill_threshold` edge.
5. **`enable_developer_extras`** stays `true` in debug builds
   (`cfg!(debug_assertions)`), `false` in release — mirror how
   `enable_write_console_messages_to_stdout` is already gated in the same
   function.
6. **This has no roadmap item and adds a genuinely new behaviour (the app
   tearing down a running view on its own).** Per the repo's planning rules it
   wants a `docs/requirements.md` entry and a roadmap item before
   implementation — **run `/msg-pre-roadmap` then `/msg-roadmap-plan-item`, or
   flag it to the user and confirm how they want to proceed, before writing
   code.** It pairs naturally with item 05 (`FR.7.3` measurement) and depends
   on item 08 for the `kill_threshold` path. If told to proceed as a prompt,
   follow the branch-first rule and run `make verify` before handing back.
7. **New behaviour is backed by tests where it can be.** The next-recycle
   decision (which account is due, given the last-recycle times and the
   threshold) is a pure function with unit tests, the way item 07's
   `start_queue::next_up` is. The teardown/rebuild itself is `(manual)` in the
   item's `test-script.md`, driven against the headless harness the item-07
   slices used (`Xvfb` + `ffmpeg` + a ctypes XTest driver): a fixture with a
   page that grows its own heap (`data:text/html` + a script that appends to an
   array on an interval), left running long enough to cross the threshold, then
   check `ps -C WebKitWebProcess` shows the pid changed and RSS dropped.

## Output

Rust across `idle-manager-shell` (`web_view.rs` for `MemoryPressureSettings` +
`enable_developer_extras` gating, a recycle scheduler — likely its own
`renderer_recycle.rs` — and `window/imp.rs` wiring) and probably
`idle-manager-core` (a `SessionBook` hook if the core needs to record a
recycle). Possibly `idle-manager-metrics` if the trigger reads PSS. A
`docs/requirements.md` entry and a roadmap item first, unless the user says
otherwise. A heap-snapshot write-up of the Huntera leak to send upstream.
