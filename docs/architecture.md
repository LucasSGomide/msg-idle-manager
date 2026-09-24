# Architecture rules

Rules for anything architecture-shaped. `project.yml` points every
`**Architecture**` bullet in a roadmap item at this file.

Each rule is one imperative and one line of why. A rule with no why is a
preference, and the next person will not know whether to keep it.

Numbered, because roadmap items cite them by number — renumbering breaks the
citations, so append rather than reorder.

The stack these rules assume is in [`stack.md`](stack.md); how the code inside a
crate is written is in [`code-standards.md`](code-standards.md).

## The shape

One binary, seven crates, one direction of dependency.

```
                        ┌─────────────────────────────┐
                        │        idle-manager         │  binary
                        │ builds adapters, runs the   │  composition root
                        │ GTK application             │
                        └──────────────┬──────────────┘
        ┌──────────────┬───────────────┼───────────────┬──────────────┐
        ▼              ▼               ▼               ▼              │
┌──────────────┐ ┌──────────────┐ ┌──────────────┐ ┌──────────────┐   │
│ …-shell      │ │ …-remote     │ │ …-store      │ │ …-metrics    │   │
│ GTK 4,       │ │ the phone    │ │ XDG, presets │ │ /proc PSS on │   │
│ WebKitGTK or │ │ server: HTTP │ │ session and  │ │ Linux,       │   │
│ WebView2     │ │ + WebSocket  │ │ phone files  │ │ process tree │   │
│ widgets      │ │ on std       │ │              │ │ on Windows   │   │
└──────┬───────┘ └──────┬───────┘ └──────┬───────┘ └──────┬───────┘   │
       └────────────────┴────────┬───────┴────────────────┴───────────┘
                                 ▼
                   ┌──────────────────────────────┐
                   │      idle-manager-core       │
                   │ sessions, layout, backoff,   │
                   │ the remote vocabulary, the   │
                   │ traits the rest fulfils      │
                   │ no GTK · no I/O · no serde   │
                   └──────────────────────────────┘
```

Arrows are "depends on". There is no arrow back up, and none sideways.

Two of the four adapters above *drive* the application — the shell from the
desktop's own window, the remote crate from a phone on the network — and both
do it the same way: they turn what the user did into a domain intent and
render the state the domain answers with (rule 8). The other two are driven:
the store and metrics answer when asked.

A seventh crate hangs off the binary alone, fitting neither pattern:

```
                        ┌─────────────────────────────┐
                        │        idle-manager         │
                        └──────────────┬──────────────┘
                                       ▼
                      ┌──────────────────────────────┐
                      │      idle-manager-update      │
                      │ Velopack's install/update     │
                      │ hooks, called before anything │
                      │ else exists (roadmap item 16) │
                      └──────────────────────────────┘
```

`idle-manager-update` neither drives nor is driven: it is the hook Velopack's
own helper restarts the program into mid-swap, run as the very first statement
of `main`, before the renderer choice, before tracing, before any store is
opened (code standards rule 18). It is the one crate in the workspace allowed
to depend on Velopack, the same way `idle-manager-metrics` is the one crate
allowed to depend on `/proc` (rules 2–4).

## Rules

1. **Keep `idle-manager-core` free of GTK, WebKit, serde, the filesystem and
   `/proc`.** Its tests must run in milliseconds on a machine with no display
   server, and every one of those dependencies would end that.

2. **Let adapters depend on the core, never on each other.** `store` reaching
   into `metrics` would make two independently replaceable things one thing, and
   the compiler would stop being able to tell you which.

3. **Let only the binary depend on an adapter.** The composition root is the one
   place allowed to know that persistence is TOML and that memory comes from
   `/proc`; everywhere else works against a trait.

4. **Enforce rules 1–3 with `make arch-check`, not with review.** A forbidden
   edge is a build failure in `scripts/arch-check.sh`, so nobody has to notice it
   in a diff.

5. **Define a port on the core side and its adapter on the outside.** The trait
   describes what the domain needs — `MemoryProbe`; the implementation names the
   technology that supplies it — `ProcPssProbe`. That way the domain reads as a
   description of the problem, not of the machinery.

6. **Introduce a port only when the domain needs something it must not know how
   to do, or when a test needs a fake.** A trait with one implementation and no
   test double is indirection with no reader benefit.

7. **Give the file format its own types in `store`, mapped to and from domain
   types.** The session file is a contract with files users already have on disk;
   deriving `Serialize` on a domain type silently turns every domain refactor
   into a breaking format change.

8. **Flow state in one direction: intent → transition → state → render.** The
   shell sends what the user did, the core decides what that means, the shell
   redraws from the result. A widget that mutates domain state directly makes the
   state machine unreadable from the core alone.

9. **Keep the core synchronous and pure — no async runtime, no threads, no
   clock.** Pass elapsed time and randomness in as arguments so the restart
   backoff and every other policy can be tested without waiting for it.

10. **Do every widget call on the GTK main context.** GTK 4 is not thread-safe;
    move blocking work off with `gio::spawn_blocking` and come back with
    `glib::spawn_future_local`.

11. **Return a `thiserror` enum from a library crate and `anyhow::Result` from
    the binary.** A caller inside the workspace needs to match on the failure; a
    caller at the top of `main` only needs to print it.

12. **Put the widget's private implementation in an `imp` module beside it.**
    That is the gtk4-rs convention — `session_grid.rs` holding the public wrapper,
    `session_grid/imp.rs` holding the `ObjectSubclass` — so every upstream example
    translates without rewriting.

13. **Describe widgets in `.ui` composite templates compiled into a GResource by
    `build.rs`.** Markup keeps layout out of Rust and reloads with the binary;
    Blueprint would add a non-cargo build tool that `make bootstrap` cannot
    install from crates.io.

14. **Test the core with unit tests, the adapters with integration tests, and the
    shell with the item's `test-script.md`.** Driving GTK from a test harness
    costs more than it catches; the hand-run runbook CLAUDE.md already requires is
    the shell's coverage.

## Folder structure

```
idle-manager/
├── Cargo.toml                       workspace: members, shared deps, shared lints
├── Cargo.lock                       committed — release builds are --locked
├── Makefile                         every developer command
├── rust-toolchain.toml              the pinned compiler
├── rustfmt.toml  clippy.toml  deny.toml  .editorconfig
├── crates/
│   ├── idle-manager/                the binary
│   │   └── src/main.rs              composition root and nothing else
│   ├── idle-manager-core/
│   │   ├── src/
│   │   │   ├── lib.rs               re-exports the crate's public surface
│   │   │   ├── session.rs           identity, liveness, visibility, keep-awake
│   │   │   ├── layout.rs            layouts, slots, what off-grid means
│   │   │   ├── restart.rs           the crash backoff policy
│   │   │   ├── remote.rs            what a phone may ask, the snapshot it renders,
│   │   │   │                         the attach policy (roadmap item 13)
│   │   │   └── ports.rs             the traits adapters implement
│   │   └── tests/
│   ├── idle-manager-store/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── paths.rs             XDG config and data locations
│   │   │   ├── preset.rs            the preset catalogue
│   │   │   ├── session_file.rs      the persisted record and its mapping
│   │   │   └── phone_record.rs      the enrolled phone, owner-readable only
│   │   └── tests/
│   ├── idle-manager-metrics/
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── proc_pss.rs          smaps_rollup parsing
│   │   └── tests/fixtures/          captured /proc output
│   ├── idle-manager-remote/         the phone server (roadmap item 13): the
│   │   ├── assets/phone.html         second driving adapter, on std threads
│   │   └── src/                      and sockets, depending on the core alone
│   ├── idle-manager-update/         Velopack's install/update hooks (roadmap
│   │   └── src/lib.rs                item 16): the one crate allowed to know
│   │                                 Velopack exists
│   └── idle-manager-shell/
│       ├── build.rs                 compiles resources/ into a GResource
│       ├── resources/
│       │   ├── idle-manager.gresource.xml
│       │   ├── ui/session-grid.ui   one file per widget, kebab-case
│       │   ├── js/keep-awake.js     the document-start user script
│       │   └── js/webview2-bridge.js  Windows only: the window.ipc shim
│       └── src/
│           ├── lib.rs
│           ├── window.rs            + window/imp.rs
│           ├── session_grid.rs      + session_grid/imp.rs
│           ├── session_sidebar.rs   + session_sidebar/imp.rs
│           ├── web_view.rs          account settings and view lifecycle
│           └── web_engine.rs        the engine seam (roadmap item 12):
│                                     web_engine/webkit.rs on Linux,
│                                     web_engine/webview2.rs on Windows
├── presets/                         hand-editable game presets, kebab-case TOML
├── scripts/                         repo tooling, called from the Makefile
└── docs/                            the planning tree, per project.yml
```

## Where a change goes

| What you are adding | Where it goes |
| --- | --- |
| A rule about session state, layout or retries | `idle-manager-core` |
| Anything read from or written to disk | `idle-manager-store` |
| Anything read from `/proc`, or Windows' process tree | `idle-manager-metrics` |
| A widget, a web view, a signal handler | `idle-manager-shell` |
| Anything naming `WebKitGTK` or `WebView2` directly | `idle-manager-shell/src/web_engine/` (roadmap item 12) |
| A word the phone and the desktop exchange, or the policy that reads it | `idle-manager-core/src/remote.rs` |
| Anything that opens a socket, speaks HTTP or WebSocket, or encodes a frame | `idle-manager-remote` (roadmap item 13) |
| Anything naming `Velopack` directly, or the install/update hooks | `idle-manager-update` (roadmap item 16) |
| Knowing which adapter is used | `crates/idle-manager/src/main.rs` |
| A game's starting URL, user agent or zoom | `presets/<game>.toml` |
