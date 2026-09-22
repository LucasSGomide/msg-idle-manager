# 05 — The keys on Windows

**Roadmap:** [14](../../roadmap/14-keyboard-navigation/README.md) · **Scope:** front-end · **Depends on:** 04

## Context

This application runs on Linux and on Windows, and on each it embeds a
different browser engine to show the games: WebKit on Linux, Microsoft's
WebView2 on Windows. The previous slice put "next account" on `Shift`+`Tab`
and "next workspace" on `Ctrl`+`Tab`, caught by the window's own key listener
before a game page can see them. On Windows that listener never fires while a
game has the keyboard, because WebView2 is a native child window that takes
its keys straight from the operating system. The reload and zoom keys already
solved this for `Ctrl` chords: WebView2 offers an "accelerator key" callback
that fires for chords a page would otherwise swallow, and the application
subscribes to it and hands each key to the same shortcut function the Linux
listener calls.

This slice widens that path to the two new keys. The callback fires for
`Shift`+`Tab` as well as `Ctrl`+`Tab`, because Tab does not map to a
character, but the subscription today ignores anything without `Ctrl` held.
It now reads both `Ctrl` and `Shift`, builds the modifier set the shortcut
table expects, and maps the Tab virtual key, which the key table did not know.
Two constraints shape the rest. The callback runs while the browser process is
paused waiting for an answer, and switching workspace refocuses a different
embedded engine, so the shortcut is not run inside the callback: the callback
only decides which shortcut it is, marks the key handled so the page never
sees a key-down, and schedules the act on the main loop's next idle turn. And
Windows reports auto-repeat itself, as a "was already down" flag on the event,
so a held key is dropped there rather than through the latch the Linux
listener uses.

The reload and zoom keys take the same idle hop for uniformity; nothing about
them needs to run synchronously. The whole slice is three files on the Windows
side of the engine seam and compiles from Linux through the cross-check the
build already has; the behaviour itself is proven in the Windows virtual
machine with a physical keyboard, which also settles the item's second open
question, whether reading the Shift key's state inside the callback behaves
as reading Ctrl's does.

## User experience

- **Entry** — `Shift`+`Tab` and `Ctrl`+`Tab` on Windows, while a WebView2
  game page has the keyboard.
- **Flow** — Identical to Linux: next account steps and turns pages, next
  workspace switches; holding either key does not repeat.
- **States** — The page never sees a `keydown` for either chord; the same
  no-op states as on Linux (one account, one workspace, selection mode).
- **Pattern** — The reload and zoom keys and the two new ones share one path
  through `handle_shortcut_key`'s halves; nothing engine-specific decides
  what a key does.

## Technical details

- **Architecture** — in
  `crates/idle-manager-shell/src/web_engine/webview2/ffi.rs`,
  `watch_accelerator_keys` stops returning early when `Ctrl` is not held: it
  reads `GetKeyState` for `VK_CONTROL` and `VK_SHIFT`, builds a
  `gdk::ModifierType` with `CONTROL_MASK` and/or `SHIFT_MASK`, reads
  `args.PhysicalKeyStatus()` and returns early when `WasKeyDown` is set (the
  auto-repeat drop on this engine), then calls `on_key(virtual_key,
  modifiers)`; the closure's type becomes `Fn(u32, gdk::ModifierType) -> bool
  + 'static`. A `true` still marks the event handled (`FR.23.5`).
- **Architecture** — `web_engine/virtual_key.rs` adds `vk::TAB = 0x09`
  mapping to `gdk::Key::Tab` in `gdk_key_for_virtual_key`, with a test beside
  the existing ones.
- **Architecture** — the closure in `webview2/host/imp.rs` (the
  `watch_accelerator_keys` call in the view-ready path) becomes: map the key
  with `gdk_key_for_virtual_key`, call `shortcut::shortcut_for(key,
  modifiers)`, and on `Some(shortcut)` schedule
  `glib::idle_add_local_once` that upgrades a `glib::WeakRef<Window>` and
  runs `window.imp().run_shortcut(shortcut)`, returning `true`; `None` returns
  `false`. The idle hop is the constraint `AcceleratorKeyPressed` imposes —
  the callback runs with the browser process blocked and a switch refocuses
  another `EngineHost` (rule 10). `Reload` and `Zoom` take the same hop.
- **Code standards** — the two constraints (the idle hop and `WasKeyDown` in
  place of the GTK latch) are commented where they bite (rule 18). The new
  `unsafe` blocks in `ffi.rs` — `GetKeyState(VK_SHIFT)` and
  `PhysicalKeyStatus` — carry a `// SAFETY:` line each, in the file that is
  the one allowed exception to rule 28.
- **Code standards** — `make windows-check` type-checks and lints the Windows
  build from Linux; `virtual_key.rs`'s test runs on both targets (rule 25).

## Acceptance criteria

- [x] `(unit)` `gdk_key_for_virtual_key(0x09)` is `Some(gdk::Key::Tab)`
- [x] `(integration)` `make verify` passes, including `make windows-check`
- [x] `(manual)` in the Windows VM with a physical keyboard and a game page
      focused, `Shift`+`Tab` walks four accounts in `2` exactly as on Linux
      and `Ctrl`+`Tab` switches workspace, skipping an empty one
- [x] `(manual)` in the VM, a page with a focused text field receives no
      `keydown` for either chord (checked with a page that logs key events),
      and `Tab` alone still moves the field focus
- [x] `(manual)` in the VM, holding `Ctrl`+`Tab` for two seconds switches
      exactly once; the second Blocker (`GetKeyState(VK_SHIFT)` inside the
      callback) is recorded as confirmed in the runbook
- [x] `(manual)` in the VM, `F5` and `Ctrl`+`+` still reload and zoom the
      focused game after the idle hop

## References

- [Roadmap item](../../roadmap/14-keyboard-navigation/README.md) — Front-end
  "Windows"; Technical References, the first; Blockers, the second
- [Wireframes](../../roadmap/14-keyboard-navigation/wireframes/) — no screen
- [`docs/requirements.md`](../../requirements.md) — `FR.23.3`, `FR.23.5`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12
- [`docs/code-standards.md`](../../code-standards.md) — rules 18, 25, 28
- [`docs/naming.md`](../../naming.md) — rule 2
- [`docs/windows-vm.md`](../../windows-vm.md) — the VM's daily loop and what
  it can prove

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The `(manual)` steps run in
the VM from `scripts/windows-vm/compose.yml` with a physical keyboard, as item
12's runbook does.
