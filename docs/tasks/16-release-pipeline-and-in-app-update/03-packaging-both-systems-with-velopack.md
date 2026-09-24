# 03 — Packaging both systems with Velopack

**Roadmap:** [16](../../roadmap/16-release-pipeline-and-in-app-update/README.md) · **Scope:** back-end · **Depends on:** 02

## Context

A Windows user today gets a zip: unzip it anywhere, double-click the program.
A Linux user gets nothing and builds from source. Neither copy can ever update
itself. On Windows that is a hard problem, because a running program cannot
overwrite itself or the seventy library files it has loaded; something has to
do the swap after the program has exited.

This slice adopts Velopack, a packaging and update tool, for both systems, and
makes the two downloads it produces. On Windows the same staged folder the
project already assembles (the program, the GTK libraries, the Visual C++
runtime, the icon theme, the image loaders) is handed to Velopack's packer,
which writes a portable zip holding a small helper called `Update.exe` and a
`current` folder with the program inside. Unzip and run, still no installer and
no administrator rights; later, the helper is what swaps the folder while the
program is closed. On Linux the packer wraps the optimised program, the game
presets, a desktop entry and an icon into one AppImage, a single file the user
marks executable and runs. GTK and WebKitGTK are not bundled into it; the file
uses the system's own copies, which keeps the download tiny and the supported
systems exactly what the project already promises.

Velopack's helper starts the program with special arguments during a swap and
expects it to exit at once, so the program's very first statement hands those
arguments to a hook. That hook lives in a new seventh crate, the one place in
the workspace allowed to know Velopack exists. The boundary checks learn about
the crate so nothing else can reach it. Velopack also brings dependencies the
project's audit rejects today, one licence and one unmaintained crate, and it
makes the Windows cross-build need the `clang-cl` compiler; this slice records
both exceptions with their reasons and names the compiler as a required tool.

It is its own slice because signing, the update port and the publish workflow
all build on the packages and the crate it creates.

## Technical details

- **Architecture** — create `crates/idle-manager-update/` (`Cargo.toml`
  depending on `idle-manager-core`, `velopack`, `thiserror`, `tracing`;
  `lib.rs` with the crate doc and `pub fn run_hooks()` calling
  `velopack::VelopackApp::build().run()`); declare `velopack = "1.2"` in the
  workspace root; `crates/idle-manager/src/main.rs` calls
  `idle_manager_update::run_hooks()` as the first statement of `main`, before
  the renderer choice and the tracing subscriber, with a comment saying why
  (code standards rule 18).
- **Architecture** — `scripts/arch-check.sh`: add `idle-manager-update` to the
  forbidden lists of `idle-manager-core`, `idle-manager-store`,
  `idle-manager-metrics`, `idle-manager-shell` and `idle-manager-remote`, and a
  new rule `idle-manager-update gtk4 gdk4 glib gio webkit6 wry webview2-com
  gdk4-win32 idle-manager-shell idle-manager-store idle-manager-metrics
  idle-manager-remote` (rules 2, 3, 4).
- **Architecture** — `scripts/windows-package.sh` keeps building the staging
  folder and then, instead of `zip`, runs `vpk [win] pack --packId IdleManager
  --packVersion "$version" --packDir "$staging" --mainExe idle-manager.exe
  --noDelta --outputDir dist/releases/win`; the script asserts
  `dist/releases/win/IdleManager-win-Portable.zip`, the `.nupkg` and
  `releases.win.json` exist and deletes `IdleManager-win-Setup.exe`.
- **Architecture** — new `make linux-package`: `make release`, then
  `scripts/linux-package.sh` stages `target/release/idle-manager`, `presets/`,
  `release/idle-manager.desktop` and `release/idle-manager.png` (a plain
  256 px PNG, committed) into `dist/.staging/linux` and runs `vpk [linux] pack
  --packId IdleManager --packVersion "$version" --packDir … --mainExe
  idle-manager --icon release/idle-manager.png --noDelta --outputDir
  dist/releases/linux`, asserting `IdleManager.AppImage`, the `.nupkg` and
  `releases.linux.json`.
- **Architecture** — `deny.toml`: `CDLA-Permissive-2.0` in `licenses.allow`
  and `RUSTSEC-2024-0388` in `advisories.ignore`, each with a comment naming
  `velopack` → `ureq` → `rustls` and the Velopack version, and saying a bump
  that drops the edge is when the entry goes.
- **Architecture** — `scripts/system-check.sh` names `clang-cl`, `lld-link`
  and `vpk` as tools with one install line each (`apt install clang lld llvm`;
  `dotnet tool install -g vpk`), failing only when a target that needs them
  runs; `docs/stack.md` lists `velopack`, `vpk` and the `.NET SDK` with
  reasons, and "Cross-compiling" says why `clang-cl` is now needed;
  `docs/architecture.md`'s diagram, folder tree and "Where a change goes" table
  gain the seventh crate.
- **Code standards** — the two package scripts document each staged file and
  each `vpk` flag with its reason at the top (rule 16); `run_hooks` carries a
  `///` contract (rule 17).

## Acceptance criteria

- [x] `(integration)` `make windows-package` writes
      `dist/releases/win/IdleManager-win-Portable.zip` whose listing contains
      `Update.exe` and `current/idle-manager.exe`, plus
      `IdleManager-<version>-full.nupkg` and `releases.win.json`, and no
      `Setup.exe` remains in `dist/releases/win`
- [x] `(integration)` `make linux-package` writes
      `dist/releases/linux/IdleManager.AppImage`, the `.nupkg` and
      `releases.linux.json`, and `ldd` on the extracted binary resolves
      `libgtk-4` and `libwebkitgtk-6.0` from the system, not from the image
- [ ] `(e2e)` the AppImage starts on a clean `ubuntu:24.04` container with
      only `libgtk-4-1 libwebkitgtk-6.0-4` installed, under Xvfb, and the
      window template loads (the `starting idle-manager` log line appears and
      the process is alive after 10 s); if `libfuse2` turns out to be needed,
      the criterion records it and the Blocker on the roadmap item is updated
      — not run: this session has no working Docker daemon (`docker info`
      fails to reach `/var/run/docker.sock` or the Desktop backend, and there
      is no sudo to start one), so the container could not be built. A local,
      non-containerized sanity run (`xvfb-run -a
      dist/releases/linux/IdleManager.AppImage`, and the same for the
      extracted binary and for the plain non-packaged `idle-manager`) shows
      identical behaviour for all three: the `starting idle-manager` log line
      appears, then the process exits within a fraction of a second under
      this machine's Xvfb — a pre-existing headless-display limitation of
      this sandbox, not something task 03 introduced (the plain, unpackaged
      binary built before this task's changes does the same). No FUSE-related
      error appeared running the AppImage directly (this machine has
      `fuse3`/`libfuse3`, not `libfuse2`), so there is no evidence either way
      that `libfuse2` is needed; the Blocker is left as written until the
      real container test can run.
- [x] `(integration)` `idle-manager --veloapp-install <dir>` style hook
      arguments make the binary exit 0 at once without touching
      `~/.config/idle-manager`
- [x] `(integration)` `make arch-check` passes and fails when a test edit makes
      `idle-manager-shell` depend on `idle-manager-update`
- [x] `(integration)` `make audit` passes with exactly the two new entries in
      `deny.toml`, and removing either makes it fail on the named crate
- [x] `(integration)` `make windows-check` and `make verify` pass with
      `clang-cl` installed

## References

- [Roadmap item](../../roadmap/16-release-pipeline-and-in-app-update/README.md)
  — Back-end "Packaging both systems", "The audit and the bootstrap";
  Blockers on the AppImage, `clang-cl`, the icon
- [`docs/requirements.md`](../../requirements.md) — Distribution `FR.3.2`,
  `FR.3.3`; Platform Support `FR.3.4`
- [`docs/architecture.md`](../../architecture.md) — rules 2, 3, 4
- [`docs/code-standards.md`](../../code-standards.md) — rules 16, 17, 18
- [`docs/stack.md`](../../stack.md) — "Cross-compiling", "Version policy"
- [`docs/naming.md`](../../naming.md) — rules 1, 5

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
