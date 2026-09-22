# 03 — The new keys on Windows

**Roadmap:** [15](../../roadmap/15-window-shortcuts-and-focus/README.md) · **Scope:** front-end · **Depends on:** 02

## Context

The application runs on Linux and on Windows, and the two use different web
engines. On Linux a keypress reaches the window through GTK's own key
controller; on Windows a key pressed while a game page has focus reaches the
application through an event the Edge engine raises before the page sees it.
Item 12 built that second path for the reload and zoom keys, and item 14
widened it: it now reports whether `Ctrl` and `Shift` are held, throws away
the repeats a held key produces, marks the event as handled so the page never
sees it, and defers the act to the next turn of the main loop because the
event fires with the browser process blocked.

All of that is engine plumbing, and none of it needs touching again. What the
Windows path still cannot do is recognise the six new keys, because it works
from Windows' own numeric key codes and only the codes for the keys bound so
far have ever been translated. This slice adds the six missing translations —
`B`, `P`, `S`, `1`, `2` and `4` — and nothing else.

It is a slice of its own because it is the only Windows-side change in the
item and because it cannot be fully proved on this machine: the translation
table is pure and unit tested here, but whether the chords really arrive from
a physical keyboard is a check in the Windows VM, which task 05 runs and
records.

## User experience

- **Entry** — the same eight chords, pressed on a Windows machine.
- **Flow** — identical to Linux, by construction: the same table decides what
  a key means and the same function acts on it.
- **States** — a page never receives a `keydown` for a chord; a held chord
  runs once; a chord that does not apply is silent. All three are already the
  Windows path's behaviour and are inherited, not re-implemented.
- **Pattern** — one table, two engines. No second key path is added
  (`FR.26.5`).

## Technical details

- **Architecture** — `web_engine/virtual_key.rs`: `mod vk` gains `KEY_B`
  (`0x42`), `KEY_P` (`0x50`), `KEY_S` (`0x53`), `KEY_1` (`0x31`), `KEY_2`
  (`0x32`) and `KEY_4` (`0x34`), each documented with its `winuser.h` name as
  the existing constants are, and `gdk_key_for_virtual_key` maps them to
  `gdk::Key::b`, `p`, `s`, `_1`, `_2` and `_4`. The module stays
  dependency-free and outside the `cfg(windows)` gate, so a Linux `cargo test`
  runs its tests (rule 14).
- **Architecture** — nothing in `web_engine/webview2/ffi.rs` or
  `web_engine/webview2/host/imp.rs` changes. `watch_accelerator_keys` already
  builds a `gdk::ModifierType` from `GetKeyState(VK_CONTROL)` and
  `GetKeyState(VK_SHIFT)`, already returns early on
  `PhysicalKeyStatus.WasKeyDown`, and the host's closure already runs whatever
  `shortcut_for` returns from `glib::idle_add_local_once` and reports the
  event handled (`FR.26.7`, rules 10, 12).
- **Code standards** — the unshifted-virtual-key asymmetry is recorded where it
  bites: Win32 reports `VK_P` whether or not `Shift` is held, so
  `Ctrl`+`Shift`+`P` arrives as `gdk::Key::p` with `SHIFT_MASK` set, while GDK
  on Linux delivers `gdk::Key::P`. Task 02's table matches both cases and
  discriminates on the modifier bit precisely so this asymmetry is invisible
  above the mapping; a comment in `virtual_key.rs` says so, rather than leaving
  the next reader to rediscover it (rule 18).
- **Code standards** — the existing `an_unbound_virtual_key_maps_to_none` test
  uses `0x41` (`A`), which stays unbound; it is left as it is (rule 21 — do not
  rewrite a passing test to accommodate a change that does not affect it).

## Acceptance criteria

- [x] `(unit)` `gdk_key_for_virtual_key` maps `0x42`, `0x50`, `0x53`, `0x31`,
      `0x32` and `0x34` to `gdk::Key::b`, `p`, `s`, `_1`, `_2` and `_4`
- [x] `(unit)` `0x33` (`VK_3`) and `0x41` (`VK_A`) still map to `None`
- [x] `(unit)` the mapped keyvals, run through `shortcut_for` with
      `CONTROL_MASK` — and with `CONTROL_MASK | SHIFT_MASK` for `p` and `s` —
      produce exactly the eight shortcuts the Linux keyvals produce, which is
      the property that makes one table serve both engines
- [x] `(integration)` `make verify` passes, including `make windows-check`
- [x] `(manual)` recorded in task 05's Windows VM run, not here

## References

- [Roadmap item](../../roadmap/15-window-shortcuts-and-focus/README.md) —
  Front-end "Windows"; Technical References, the third
- [`docs/requirements.md`](../../requirements.md) — `FR.26.7`, `FR.26.5`;
  `FR.23.5` for the path being reused
- [`docs/architecture.md`](../../architecture.md) — rules 10, 12, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 18, 21, 25
- [`docs/naming.md`](../../naming.md) — rule 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
