# 08 — The Windows release zip, and measuring its memory

**Roadmap:** [12](../../roadmap/12-windows-support/README.md) · **Scope:** full-stack · **Depends on:** 05, 06, 07

## Context

By this point the application works on Windows the same way it works on Linux.
This last slice does two things.

First, it makes the Windows program easy to hand to someone. One command on the
Linux development machine builds the optimised Windows program and packs it
into a zip. The zip holds the program plus the parts of the GTK toolkit it needs
at run time: the toolkit's libraries, its compiled settings, the icon themes and
the image loaders. Someone who unzips it anywhere on Windows and double-clicks
the program gets the working application, with no installer, no administrator
rights and no GTK install. The Microsoft web engine is not put in the zip,
because Windows already ships it. The zip lands in the folder the Windows
virtual machine sees as a drive, so the same file that gets handed out is the
one tested.

Second, it checks the promise the project exists for: that the program is
cheap to run. In the Windows virtual machine, four live game accounts are
compared against the same four games open as tabs in Microsoft Edge, using the
same memory measure for both. On Linux, the existing measurement is repeated to
show that the Windows work did not make the Linux program cost more. Both
results go into the project's memory-budget document. If Windows loses the
comparison, the first remedy is to ask the engine to use less memory for
accounts that are out of sight, and the result is recorded again.

The virtual machine has no graphics card, so it draws everything in software.
That makes graphics-acceleration and idle-processor figures meaningless there,
and this slice does not measure them. A developer with a real Windows computer
may look at them informally, and that does not decide whether this slice is
done.

This work comes last because the package must contain the complete program,
and the measurement must be of the real thing.

## User experience

- **Entry** — On Windows the user unzips
  `idle-manager-<version>-windows-x64.zip` and double-clicks `idle-manager.exe`.
- **Flow** — First launch on a Windows install with no GTK opens the normal
  window with the shipped game presets available in the add-game window,
  exactly as on Linux.
- **States** — Icons in the sidebar, dialogs and grip render (no missing-icon
  squares); no console window opens beside a release build.
- **Pattern** — Every screen is unchanged from the Linux build (design rules
  1–13).

## Technical details

- **Architecture** — `scripts/windows-package.sh`, run by `make
  windows-package`, calls `make windows-build PROFILE=release`. It then
  assembles `dist/idle-manager-<version>-windows-x64.zip` from the build and the
  unpacked gvsbuild tree in `target/windows-sdk/gtk/`:
  - `idle-manager.exe`;
  - `bin/*.dll`;
  - `share/glib-2.0/schemas/gschemas.compiled`;
  - the `Adwaita` and `hicolor` icon themes;
  - `lib/gdk-pixbuf-2.0/` as gvsbuild ships it;
  - `WebView2Loader.dll`, only if the executable imports it. Check with
    `x86_64-w64-mingw32-objdump -p` or `llvm-objdump --private-headers` and
    record the answer. **Answered while implementing: it does not** — the
    loader is linked statically, so the zip carries none. The package script
    re-checks it on every run and says which way it went.

  The version is read from `Cargo.toml` with `cargo metadata` (naming rule 1).
  `dist/` is already gitignored.
- **Architecture — added while implementing: the Visual C++ runtime.** The
  plan above was incomplete, and the zip built from it could not start on a
  clean Windows. 66 of the 67 DLLs gvsbuild ships, and the program itself,
  import `vcruntime140.dll` / `msvcp140.dll`, and gvsbuild ships neither — the
  missing piece only shows up when every import in the finished package is
  walked, which is now a step of the package script rather than a thing to
  remember. `scripts/windows-crt-fetch.sh` (run by `make bootstrap` and again
  by `make windows-package`) downloads the pinned `WINDOWS_CRT_PACKAGE` from
  the Visual Studio release channel's manifest, checks its published `sha256`,
  and unpacks the redistributable DLLs — never the `debug_nonredist` tree
  beside them, which Microsoft's terms do not allow redistributing — into
  `target/windows-sdk/crt/`. `windows-package.sh` copies them beside the
  program and fails if the three the package cannot start without are not
  there. See `docs/stack.md` for why app-local rather than an installer, and
  why the `.vsix` rather than `VC_redist.x64.exe`.
- **Architecture** — `crates/idle-manager/src/main.rs` adds
  `#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem =
  "windows")]`, so a release build opens no console window. Debug builds keep
  the console for logs.
- **Architecture** — `docs/memory-budget.md` gains a "Windows 11 VM" section.
  It records:
  - the VM's cores and RAM (from `scripts/windows-vm/compose.yml`), the
    Windows build, the WebView2 runtime version, the GTK version, and that
    rendering is software;
  - four accounts of the same games the Linux section used, idle for ten
    minutes, then three footer readings a couple of minutes apart;
  - Edge in the same VM with the same four games as tabs, read three times
    from Task Manager's private working set summed over its processes.

  It also gains a "Linux after item 12" row repeating `make memory-report`
  under the Linux section's conditions.
- **Architecture** — if the Windows figure exceeds Edge's, set
  `set_memory_usage_level(Low)` on off-grid views through `ffi.rs`, re-measure,
  and record both rows. If it still exceeds Edge's, record that on the roadmap
  item's Blockers and stop without shipping.
- **Architecture** — not measured, per ditched record 01: hardware acceleration,
  idle processor use, and the choice of `GSK_RENDERER`. GTK's default renderer
  is kept on Windows.
- **Code standards** — `docs/stack.md` "Bootstrap" gains the Windows loop:
  `make windows-package`, then run it in the VM per `docs/windows-vm.md`.
  `make verify` passes before review (rule 29).

## Acceptance criteria

- [x] `(integration)` `make verify` passes on Linux, including `windows-check`
- [x] `(integration)` `make windows-package` on Linux produces
      `dist/idle-manager-<version>-windows-x64.zip` containing
      `idle-manager.exe`, `gtk-4-1.dll` and `gschemas.compiled`
- [ ] `(manual)` in the Windows VM, unzipping the file from `Z:` to
      `C:\idle-manager` and double-clicking `idle-manager.exe` opens the window
      with icons drawn and no console window
- [ ] `(manual)` in the Windows VM, adding a game from a shipped preset in the
      unzipped release starts that game in a place
- [ ] `(manual)` in the Windows VM, the footer figure for four live accounts is
      no higher than Edge's private working set with the same four games as tabs,
      and both are recorded in `docs/memory-budget.md`
- [ ] `(manual)` on Linux, `make memory-report` with four accounts falls within
      the noise of the figures already in `docs/memory-budget.md`, recorded as
      "Linux after item 12"

## References

- [Roadmap item](../../roadmap/12-windows-support/README.md) — Back-end
  tooling paragraph, Front-end "Measurements", Blocker 6
- [Wireframes](../../roadmap/12-windows-support/wireframes/) — none change
- [Ditched 01](../../ditched/01-windows-gpu-and-cpu-measurements.md) — what
  is deliberately not measured
- [`docs/requirements.md`](../../requirements.md) — Platform Support `FR.2.1`,
  `FR.2.5`, `FR.3.4`, `FR.3.5`
- [`docs/windows-vm.md`](../../windows-vm.md)
- [`docs/memory-budget.md`](../../memory-budget.md)
- [`docs/stack.md`](../../stack.md)
- [`docs/architecture.md`](../../architecture.md) — rule 3
- [`docs/code-standards.md`](../../code-standards.md) — rule 29
- [`docs/naming.md`](../../naming.md) — rule 1
- [`docs/design.md`](../../design.md)

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The `(manual)` steps run in
the Windows VM ([`docs/windows-vm.md`](../../windows-vm.md)), apart from the
Linux measurement.
