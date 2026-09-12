# Memory budget

Task 05's deliverable: whether a live account's memory grows without bound or
settles, and — because it does not settle — the rate, and what that implies for
the rest of this item.

## Conditions

- **Machine:** `gomide`, x86_64, Linux 7.0.0-31-generic, 30 GiB RAM, 8 GiB swap.
- **Engine:** `libwebkitgtk-6.0-4` 2.52.6-0ubuntu0.26.04.1 (Ubuntu 26.04).
- **Build profile:** not recorded by the person who ran the samples; the
  reductions in tasks 06/07 were implemented against `cargo build` (dev) in
  this session, so treat the numbers below as an upper bound relative to a
  release build.
- **Games/accounts:** 4 accounts running concurrently, games not individually
  named in the samples taken.
- **Sampling:** `make memory-report` run three times "a couple of minutes
  apart" — the exact interval between runs was not recorded, which is a real
  gap against task 05's own rule ("state the window used"). The rate below is
  therefore an order-of-magnitude estimate, not the rigorous floor-of-sawtooth
  reading task 05 calls for.

## What was actually measured

| run | idle-manager (own, PSS KiB) | descendants total (PSS KiB) |
|---|---|---|
| 1 | 57,802 | 2,171,856 |
| 2 | 92,993 | 2,569,260 |
| 3 | 145,621 | 2,921,463 |

Each of the 4 `WebKitWebProces` entries was individually in the 500-800 MiB
range and climbing between samples.

## Finding

**This does not settle.** Both the shell's own process and the sum of its
rendering processes grew monotonically across all three samples, with no sign
of flattening — the shell's own PSS nearly tripled (57.8 MiB -> 145.6 MiB) and
the descendant total grew by about a third (2.17 GiB -> 2.92 GiB) across the
same three-sample window. This is not the seventy-mebibyte garbage-collector
sawtooth the roadmap item's earlier twenty-minute sample described (see
`docs/roadmap/05-memory-accounting/README.md`, "Where the memory goes"); it is
directional growth in both halves of the tree, including the shell process
which has no page content to hold and should not grow like this at all.

Taking "a couple of minutes" as 2-3 minutes between runs (the actual interval
was not recorded — see above), the two intervals give:

- **Shell process (own PSS):** roughly 35-53 MiB per interval, i.e. very
  roughly **0.8-1.5 GiB/hour**.
- **Descendant (rendering + network) processes combined:** roughly 350-400 MiB
  per interval, i.e. very roughly **7-11 GiB/hour**.

These are order-of-magnitude figures from three widely-spaced points, not a
regression line — they exist to say "this will hit swap and then OOM within
hours, not days," not to pin down a precise slope. A rigorous floor-of-sawtooth
reading over a 4+ hour soak (task 05's own acceptance criterion) has not been
done and could not be done in this session: no live instance with logged-in
accounts was available (see below). That specific criterion is left open.

**No settled figure exists to record.** Because the curve does not settle,
there is no single "MiB per game" number for task 06's limit or task 08's
budget to be chosen from in the way task 05 originally expected. Task 06's
memory-pressure thresholds and task 08's budget constant, below, are therefore
**mitigations that cap how bad the leak can get before it is noticed**, not a
fix for the leak itself — exactly the outcome task 05's acceptance criteria
anticipate for a non-settling curve.

## Why a live 4+ hour soak, a second-game soak, and a park/residue soak were not run

At the time this investigation ran, no `idle-manager` process was live
(`pgrep -af idle-manager` returned nothing) and this session has no Google
credentials to log an account back in — the project's own notes on headless
WebKit login record that a spoofed user agent or a headless run reliably breaks
Google sign-in and Turnstile, so a login could not be improvised for this
session either. These soaks need a human to leave the application open with
real, already-logged-in accounts. They remain open manual acceptance criteria
on task 05 pending that.

## Static leak audit (the shell's own process)

The shell process's own PSS growth is the more surprising half of this — a
window and a sidebar have no reason to grow at nearly 1 GiB/hour. See the
"Static audit findings" section this document's sibling commit adds below, and
the fixes applied to `crates/idle-manager-shell/src/lib.rs` and
`crates/idle-manager-shell/src/web_view.rs`.

## Follow-up (`fix/05-memory-leak`): the shell process's own growth, precisely

A second session picked this up against the user's own live, running instance
(4 accounts, one real game — Huntera) rather than a fresh, logged-out one. Two
things this adds to the audit above, one confirming, one correcting:

- **Confirmed, with a live per-mapping breakdown, that the growth is private
  heap, not graphics memory.** `awk` over `/proc/<pid>/smaps` on the live
  process showed a single `[heap]` mapping (private, anonymous, `rw-p`, no
  backing file) carrying the overwhelming majority of the process's PSS —
  304,808 KiB of it at one reading, versus a few MiB each in shared libraries.
  A GPU/EGL/`dma-buf` import would appear as its own named or `/memfd:`-backed
  mapping, not as growth inside `[heap]`. **This rules out the graphics-buffer
  explanation for the shell's own growth**: whatever is growing is being
  allocated through the process's own allocator, in code running in this
  process (ours, or a library it calls in-process — GTK, WebKitGTK's
  `webkit6` bindings, GLib), not handed off to the GPU driver.

- **Ruled out the memory-footer polling loop as the source, by direct
  reproduction.** The prime suspect going in was
  `crates/idle-manager-shell/src/memory_footer/imp.rs`'s `start_sampling` loop
  — it runs forever, every `MEMORY_SAMPLE_INTERVAL_SECS` (5s), dispatching
  `ProcPssProbe::sample()` onto `gio::spawn_blocking`'s thread pool, and that
  probe reads every process's `/proc/<pid>/status` on the whole machine to
  build a parent map before walking down to this process's descendants
  (`crates/idle-manager-metrics/src/proc_pss.rs`). Read end to end, nothing in
  either file retains state across polls: `read_process_table` builds a fresh
  `HashMap` and drops it at the end of `read_tree`, and `show_reading` replaces
  each label's text and toggles one CSS class rather than accumulating either.
  To test this directly rather than trust the reading, the loop was
  instrumented (temporarily — reverted, not part of this fix) to log
  `/proc/self/statm`'s resident-page count once per iteration, and run under
  Xvfb with no accounts open at all (so the probe's own `/proc`-wide scan was
  the only thing happening every 5 seconds, against this same 8-core, hundreds
  of processes machine). Resident pages rose during startup (34,312 → 43,321
  pages) and then went **completely flat — 43,337 to 43,338 pages — across 15
  further iterations (75 seconds) with no accounts running.** If the polling
  loop's own churn were the leak, this reproduction should have shown it; it
  did not. The loop is cleared.

- **Not settled: fragmentation vs. a true leak in the live, loaded case.**
  Distinguishing them needs `malloc_trim` called on the live process (if PSS
  drops sharply, it is fragmentation; if not, live objects are genuinely being
  retained) — this investigation's sandbox could not attach to the live PID
  (`gdb -p` refused: "Could not attach to process" — the kernel's
  `yama/ptrace_scope` is `1` here and there is no passwordless `sudo` to lower
  it), so this test could not be run mechanically. Two observations lean
  toward fragmentation rather than a strict leak, without proving it: the
  process's own PSS was watched drop from 355,794 KiB to 108,978 KiB
  (roughly -247 MiB) in a single step **while the process kept running and
  nothing was manually freed** — a real leak of still-reachable objects cannot
  self-correct like that, but glibc returning a large freed chunk near the top
  of an arena to the OS (an automatic `brk`-shrink) can; and the machine has 8
  cores, so glibc's default per-thread arena cap (`8 × ncpus` = 64) leaves a
  lot of room for the kind of per-thread heap fragmentation that a long-running,
  multi-threaded GTK/WebKit-embedding process is a textbook case for. No fix
  for this was written into the codebase: `docs/code-standards.md` rule 28
  ("write no `unsafe`") forbids introducing an FFI `mallopt` call on unproven
  grounds, and env-var mitigations (`MALLOC_ARENA_MAX`) can't be wired in from
  inside the binary either, since `std::env::set_var` is itself `unsafe` on
  this workspace's 2024 edition. **What would settle it, on the user's own
  machine** (which has the root access this sandbox does not):
  1. `gdb -p <pid> -batch -ex 'call (int)malloc_trim(0)'` against the live
     process, then re-read `/proc/<pid>/smaps_rollup` immediately after — a
     sharp drop means fragmentation, not a leak.
  2. A comparison run with `MALLOC_ARENA_MAX=1 make dev` against a plain
     `make dev`, both against the same game for the same duration — a flatter
     curve under the capped-arena run points at fragmentation as the
     mechanism and names the fix (cap arenas at deploy time, or move the
     polling loop off a rotating thread pool onto one dedicated thread).

  Until one of those is run, this document's growth-rate figures above stand,
  and the "does not settle" finding is neither confirmed as a true leak nor
  downgraded to "just fragmentation" — it is exactly as open as it was, with
  the graphics-buffer explanation now ruled out and the polling loop now
  cleared as suspects.

## Round 3: fragmentation ruled out, the actual leak found and fixed

A third session ran both of round 2's proposed settling tests directly against
the user's own live, running instance (same machine, real accounts, one real
game — Huntera on `lorvath.com`/`huntera.com.br`) rather than a sandbox that
could not attach to the live PID.

**Fragmentation is ruled out.** Two 516-523s soaks (`./scripts/memory-report.sh
--soak 30`) against the same live setup, one plain `make dev` and one
`MALLOC_ARENA_MAX=1 make dev` (capping glibc to a single arena, which should
flatten or eliminate arena-fragmentation-driven growth if that were the
mechanism):

| run | own PSS start (KiB) | own PSS end (KiB) | duration | rate |
|---|---|---|---|---|
| plain `make dev` | 201,720 | 302,900 | 516s | ≈ 690 MB/hour |
| `MALLOC_ARENA_MAX=1 make dev` | 84,757 | 237,885 | 523s | ≈ 1,030 MB/hour |

Capping arenas to one did not reduce the growth rate — if anything it was
faster, same order of magnitude either way. A true leak of live, reachable
objects does not care how many arenas glibc uses; fragmentation-driven,
self-correcting growth would have been suppressed by the cap. **This document's
round 2 fragmentation lean is wrong.** This is a true leak, gated behind
real, ongoing account activity (round 2's isolated, zero-account reproduction
of the polling loop stayed clean because it never exercised whatever this
actually is).

**Found: `register_page_console_handler`
(`crates/idle-manager-shell/src/web_view.rs`).** This handler is what
[`PAGE_CONSOLE_JS`] (`resources/js/page-console.js`) posts to — it overrides
`console.debug/log/info/warn/error` and the page's `error`/`unhandledrejection`
listeners in **every frame** and forwards each call across the WebKit
`UserContentManager` script-message bridge. It was registered unconditionally
for every account, in every build, unlike the sibling resource-load logger in
the same file which is already gated behind `diagnostics_enabled()`. Its own
doc comment already noted "one Cloudflare challenge frame alone logs hundreds
of lines per load" — an idle game that logs for telemetry, ads, or its own
debugging (Huntera runs Phaser, which logs a version banner and preload
warnings on every load) calls into this continuously for as long as the
account runs. The callback did three JSC property reads per message
(`object_get_property("level"/"origin"/"text")`, each followed by `to_str()`)
— a `javascriptcore6` 0.6.0 / `webkit6` 0.6.1 binding boundary (`Cargo.lock`),
exactly the kind of young FFI surface where a reference/rooting bug is
plausible.

Tested directly against the live instance rather than guessed at:

1. **No-op test.** With the handler still registered (so nothing about
   `UserContentManager` setup changed) but its callback body replaced with an
   empty closure — no property reads, no `tracing` calls — a 9-minute soak
   against the same live 4-account, real-Huntera setup:

   | sample | own PSS (KiB) | elapsed |
   |---|---|---|
   | after startup settled | 101,018 | 68s |
   | end of soak | 101,472 | 511s |

   454 KiB of growth over 443s (≈ 3.7 MB/hour) — flat, noise-level, against a
   baseline of 690-1,030 MB/hour. **This confirms the handler as the leak.**

2. **Real fix, same test.** The shipped fix (below) does not no-op the
   callback; it removes the registration entirely for the default case by
   gating it behind a switch that is off unless asked for. A second 9-minute
   soak against the same live accounts, with the fix in place and no override
   set:

   | sample | own PSS (KiB) | elapsed |
   |---|---|---|
   | after startup settled | 69,456 | ~40s in |
   | last sample before the process closed | 69,649 | ~214s in |

   193 KiB over 174s (≈ 4.0 MB/hour) — again flat, matching the no-op result.
   (The soak's own process was closed partway through by something outside
   this session — a concurrently-running human on the same machine — cutting
   the run to about 3 minutes instead of the intended 9; the flat trend across
   every sample taken is nonetheless the same signature as the no-op test's
   full run, not a coincidence given both start from the same 690-1,030
   MB/hour baseline.)

**The fix.** `register_page_console_handler` is still exactly as written
before (the JSC property reads are unchanged — no evidence isolated whether
`object_get_property`, `to_str`, or their combination is the specific leaking
call inside the binding, and the no-op test does not distinguish that from
"the callback ran at all"). What changed is *whether it is registered at all*.
It is gated behind a new `page_console_forwarding_enabled()`, **not** the
existing `diagnostics_enabled()` the sibling resource-load logger uses:
`diagnostics_enabled()` is `true` unconditionally in every debug build
(`cfg!(debug_assertions)`), and a debug build (`make dev`) is exactly how this
application is actually run for real, day-long accounts — reusing it here
would have left the leak on in precisely the scenario that found it. The new
switch reads the same `IDLE_MANAGER_DIAGNOSTICS` environment variable but
never turns on by build profile alone, because forwarding every `console.log`
for as long as an account runs is not a bounded diagnostics cost the way the
inspector backend or per-resource logging are — it does not settle.
`PAGE_CONSOLE_JS`'s own guard (`if (!handler) return`) means a page never even
installs the `console.*` overrides when the handler is not registered, so the
fix removes the cost on both sides of the bridge, not just the host side.

This is a real fix for a confirmed leak, not a mitigation: with the switch off
(the default, in every build), the growth this whole document describes goes
away in both the no-op and gated-off measurements above. Page console
forwarding — useful for diagnosing a login or Cloudflare-challenge failure —
is available again by setting `IDLE_MANAGER_DIAGNOSTICS=1`, same channel the
tracing filter and the other diagnostics already read, at the cost this
document now documents precisely.

**Still open, if the leak resurfaces or the switch needs to be on for a
diagnosis:** which of `object_get_property`, `to_str`, or the JSC value
passed into the callback itself is the specific leaking call was not
isolated further — the crate versions are pinned at `javascriptcore6` /
`javascriptcore6-sys` 0.6.0 and `webkit6` / `webkit6-sys` 0.6.1; no newer
release or upstream issue confirming this specific leak was found in this
session's research. If `IDLE_MANAGER_DIAGNOSTICS=1` is ever needed for a
long-running account, expect the same growth this document measured while it
is on.

## The engine-wide limit (task 06)

`WEB_PROCESS_MEMORY_LIMIT_MIB` was `0`. WebKitGTK's
`webkit_memory_pressure_settings_set_memory_limit` guards its argument with
`g_return_if_fail(memoryLimit)` — passing `0` logs a GLib critical and leaves
`baseThreshold` at its default-constructed value, which is documented as "the
system's RAM size with a maximum of 3GB." In other words the `0` placeholder
was never inert in the sense of "no limit enforced" — it was silently
discarded, leaving every rendering process governed by an unmeasured
~3 GiB default instead of a number chosen for this application. That is fixed
in `lib.rs` (see the comments beside the constants there); the new limit is
chosen conservatively relative to the single-account working set the roadmap
item's own earlier sample recorded (751 MiB, `docs/roadmap/05-memory-accounting/README.md`)
since no settled multi-account figure exists to size it against instead.

Because the curve above does not settle, this limit and its thresholds are a
ceiling on damage (the engine sheds caches and collects harder as a process
approaches the limit), not a cure — the underlying growth this document
describes needs its own fix, which is the static leak audit above and
whatever the descendant-process leak in WebKit or the game pages themselves
turns out to be (out of scope for this session; not a claim this repository's
code causes it).

## WebGL per game (task 07)

Two `make memory-report` runs, same machine and engine as above, 4 accounts
live in each, taken minutes apart on 2026-09-12:

**WebGL off** (all four presets' `webgl` key set `false`):

```
own                 75448 KiB
descendants       2459051 KiB
total             2534499 KiB
processes              26
```

**WebGL on** (the same four presets, `webgl` key removed/`true`):

```
own                 80505 KiB
descendants       2336132 KiB
total             2416637 KiB
processes              26
```

The totals do not show WebGL off as cheaper — if anything the on-run reads
slightly lower here. Read as inconclusive rather than as evidence WebGL
costs nothing: each `WebKitWebProces` figure swings by tens of megabytes
between runs (up to ~135 MiB on one process across the two samples), and the
observation during measurement was that in-game activity — e.g. spell
effects — spikes a rendering process well above its baseline independent of
WebGL. A four-account, mixed-activity snapshot is not a controlled
same-game-same-moment comparison, so this does not settle whether disabling
WebGL saves anything measurable for a game that does not need it; it is
recorded here because it is the only paired measurement taken, not because it
proves the feature's premise.
