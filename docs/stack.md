# Stack

What the project is built from, at which version, and why. Reference material —
the rules that constrain it live in [`architecture.md`](architecture.md) and
[`code-standards.md`](code-standards.md).

Versions below were the current stable releases on 2026-09-06; the phone server's (roadmap item 13) on 2026-09-19. Bumping one is a
deliberate act: change it here, change it in the manifest, run `make verify`.

## Target

Linux desktop, GTK 4 on both X11 and Wayland, and 64-bit Windows 10 and 11
(roadmap item 12). Which system a build targets is chosen at compile time,
never at runtime, and each system's code carries no trace of the other: the
web engine is `WebKitGTK` on Linux and Microsoft Edge `WebView2` on Windows,
and the memory accounting reads `/proc` on Linux and walks the process tree
through `CreateToolhelp32Snapshot` on Windows. The Windows build is developed
and verified from Linux, cross-compiled with `cargo xwin` and checked against
the Windows 11 VM in [`docs/windows-vm.md`](windows-vm.md); nobody on the
project owns a Windows computer.

The phone path (roadmap item 13) has one prerequisite outside this repository:
a mesh network that gives the desktop and the phone addresses in
`100.64.0.0/10` — Tailscale, or any WireGuard mesh that hands out that range.
The desktop binds the first such address it finds and the phone reaches it
from any network with internet access, with nothing opened on the home
router (`FR.5.1`); without the mesh the server does not listen and the phone
dialog says so. The `[listen]` override in `phone.toml` is the escape hatch
for a trusted LAN or the Windows VM (`docs/windows-vm.md`), not a second
supported path.

## Language and toolchain

| Piece | Version | Why |
| --- | --- | --- |
| Rust | 1.98.1, pinned in `rust-toolchain.toml` | A pinned compiler means a warning that fails CI fails locally too |
| Edition | 2024 | The default since 1.85; `unsafe_op_in_unsafe_fn` and RPIT capture rules are the ones we want |
| Cargo resolver | `"3"` | Respects `rust-version`, so a dependency cannot silently raise the MSRV |

## GUI

| Crate | Version | Role |
| --- | --- | --- |
| `gtk4` | 0.11 | Windows, the grid, the sidebar, on both systems |
| `webkit6` | 0.6 | Linux only: one `WebKitWebView` and one `WebKitNetworkSession` per game session |
| `glib` / `gio` | 0.22 | Main context, async, GObject plumbing |
| `wry` | 0.57 | Windows only: builds a `WebView2` view as a native child window |
| `webview2-com` | 0.39 | Windows only: the COM calls `wry` does not wrap (`ProcessFailed`, `AcceleratorKeyPressed`, profile deletion) |
| `gdk4-win32` | 0.11, `win32` feature | Windows only: the toplevel's `HWND`, for `wry::WebViewBuilder::build_as_child` |
| `raw-window-handle` | 0.6 | Windows only: the handle type `wry` takes a window as |
| `windows` | 0.62 | Windows only: the Win32 calls `webview2-com` and `idle-manager-metrics`'s process-tree walk need directly |

These four move together: `webkit6` 0.6 is built against `gtk4` 0.11 and
`glib`/`gio` 0.22, so all four are bumped in one commit or not at all.

`gtk4` is depended on with its `v4_10` feature enabled. That is not a choice:
`webkit6` 0.6 names `gtk::Accessible` unconditionally, `gtk4` 0.11 gates that
type behind `v4_10`, and nothing in `webkit6` turns the feature on — without it
the workspace does not compile. It fixes the floors below.

The five Windows-only crates are declared in the workspace root but taken only
under `crates/idle-manager-shell/Cargo.toml`'s
`[target.'cfg(windows)'.dependencies]` (`wry` must never build on Linux — it
pulls in the GTK 3 `webkit2gtk`, which clashes with `webkit6`); `windows` is
also taken the same way in `idle-manager-metrics`, for its Windows-only memory
probe.

## Libraries

| Crate | Version | Role |
| --- | --- | --- |
| `serde` | 1.0 | Derive for the persisted record types, in `store` only |
| `toml` | 1.1 | Presets and the session file — hand-editable, which the requirements ask for |
| `directories` | 6.0 | XDG config and data paths, so nothing is hardcoded |
| `thiserror` | 2.0 | Error enums in every library crate |
| `anyhow` | 1.0 | The binary's top-level error type |
| `tracing` + `tracing-subscriber` | 0.1 / 0.3 | Structured logs, filtered by `RUST_LOG` |
| `serde_json` | 1.0 | The phone's wire messages, in `remote` only (roadmap item 13) |
| `sha1` + `base64` | 0.11 / 0.23 | The one hash and one encoding the RFC 6455 WebSocket handshake needs |
| `hmac` + `sha2` | 0.13 / 0.11 | The enrolment proof both sides compute over a shared secret, verified in constant time |
| `rand` | 0.10 | Enrolment codes, device ids, secrets and challenges |
| `jpeg-encoder` | 0.7 | Frames from raw pixels, single-digit milliseconds at 412 × 915; carries the IJG licence, allowed in `deny.toml` for it |
| `if-addrs` | 0.15 | Finds the mesh address in `100.64.0.0/10` to listen on |
| `async-channel` | 2.5 | The intent channel from the server's threads to the GTK main context; no runtime, so no `tokio` |
| `qrcode` | 0.14 | The enrolment address as a code the phone's camera reads, in `shell` only |
| `velopack` | 1.2 | `run_hooks`'s `VelopackApp::build().run()`, in `idle-manager-update` only (roadmap item 16 task 03) — the one crate allowed to depend on it (architecture rules 2, 3, 4) |

## Development tooling

| Tool | Role | Entry point |
| --- | --- | --- |
| `rustfmt` | Formatting, non-negotiable | `make fmt` |
| `clippy` | Lints, warnings are errors | `make lint` |
| `cargo-nextest` | Test runner, one process per test | `make test` |
| `cargo-deny` | Advisories, licences, duplicate versions | `make audit` |
| `cargo-watch` | Rebuild and rerun on save | `make watch` |
| `rstest` | Table-driven cases without a macro of our own | dev-dependency |
| `insta` | Snapshots of the on-disk file format | dev-dependency, `store` |
| `cargo-xwin` | Supplies the MSVC CRT and Windows SDK import libraries for the cross build | `make windows-build` |
| `jq`, `zip`, `unzip`, `curl` | Read the Visual Studio manifest and assemble the release zip | `make windows-package` |
| `git-cliff` | Reads `cliff.toml` and the commit history to pick the next version and write the release notes | `make release-version`, `make release-notes`, `make release-prepare` |
| `.NET SDK` | Runs `vpk`, Velopack's CLI, which is a .NET tool (roadmap item 16 task 03) | `dotnet tool install -g vpk` |
| `vpk` | Packs the staged folder into the portable zip (Windows) or the AppImage (Linux) and writes the release feed both read from later | `make windows-package`, `make linux-package` |

## System packages

WebKitGTK and GTK are C libraries; cargo cannot install them, and `gtk4-sys`
resolves them through `pkg-config` inside a build script — so a missing one
surfaces hundreds of lines into a compile.

```
sudo apt install build-essential pkg-config libgtk-4-dev libwebkitgtk-6.0-dev
```

| Library | Minimum | Set by |
| --- | --- | --- |
| GTK 4 | 4.10 | The `v4_10` feature `webkit6` forces on `gtk4` |
| WebKitGTK 6.0 | 2.42 | The `v2_42` feature `idle-manager-shell` turns on for the settings feature list |

`webkit6-sys` 0.6 itself declares only 2.40. The floor is 2.42 because
`idle-manager-shell` enables `webkit6`'s `v2_42` feature for
`Settings::all_features` and `Settings::set_feature_enabled`, which item 04 needs
to turn off a hidden page's timer throttling per account and which libwebkit2gtk
added in 2.42.

`make system-check` names in a single line the ones you are missing *or* that
are too old, and every target that compiles the workspace runs it first. A
library that is present but below its floor is the case worth checking for: it
passes an existence test and then fails deep inside a build script.

**Cross-compiling and cross-linting Windows from Linux.** `make windows-check`
and `make windows-build` type-check, lint and cross-compile against
`x86_64-pc-windows-msvc`, but linking still needs the Windows import libraries
`pkg-config` would normally resolve against a system install. `make bootstrap`
downloads and unpacks the pinned `GTK4_Gvsbuild_<version>_x64.zip` release into
`target/windows-sdk/gtk/` and generates a small `pkg-config --define-prefix`
wrapper script (`target/windows-sdk/pkg-config-wrapper.sh`) — the `.pc` files
gvsbuild ships bake in the Windows build machine's own path, and
`--define-prefix` is what makes `pkg-config` compute the real prefix from each
file's own location instead. `PKG_CONFIG_ALLOW_CROSS`, `PKG_CONFIG_PATH` and
`PKG_CONFIG` (pointed at the wrapper) are set by the two `make` targets, never
needed by hand.

Cross-compiling itself stayed pure Rust and prebuilt import libraries through
roadmap item 12 — nothing needed a C compiler, so nothing named one. Roadmap
item 16 task 03 changes that: `idle-manager-update`'s `velopack` dependency
reaches `ureq` → `rustls` → `ring`, and `ring`'s build script compiles C
straight into the target, `x86_64-pc-windows-msvc` included. `cargo xwin`
already downloads the MSVC headers that C needs; what it does not supply is
the compiler itself, so `clang-cl` (Clang's MSVC-compatible driver) and
`lld-link` (LLD's MSVC-compatible linker) must be on `PATH` —
`scripts/system-check.sh clang-cl lld-link` is what `make windows-check` and
`make windows-build` check for first. `cargo xwin` looks for a plain `clang`
on `PATH` and symlinks its own `clang-cl`; `lld-link` it gets for free from
`rust-lld`, already bundled in the pinned Rust toolchain, unless a `lld-link`
already on `PATH` shadows it; only `llvm-lib`, the archiver, has no such
fallback and must resolve from a real LLVM install. `apt install clang lld
llvm` (verified 2026-09-24, roadmap item 16 task 03's own Blocker) is *not*
enough by itself: Ubuntu's packages place only the *versioned* names
(`clang-cl-21`, `lld-link-21`, `llvm-lib-21`, …) in `/usr/bin`; the
unversioned names above live in `/usr/lib/llvm-<N>/bin`, which is not on
`PATH` by default, so that directory has to be added — `export
PATH="$(echo /usr/lib/llvm-*/bin):$PATH"` after the `apt install`, which is
exactly what `scripts/system-check.sh`'s own missing-tool message prints.
This is also why `windows-check` runs `cargo xwin clippy` rather than plain
`cargo clippy --target x86_64-pc-windows-msvc`: only the `xwin` subcommand
points the C compiler at the downloaded MSVC sysroot, and without it `ring`'s
build script cannot find `assert.h` or the rest of the C runtime headers.

**The Visual C++ runtime the release zip carries.** Every DLL gvsbuild builds
— 66 of the 67 in the package — and the program itself import
`vcruntime140.dll` and `msvcp140.dll`, and gvsbuild ships neither, so a clean
Windows install answers a double-click with "VCRUNTIME140.dll was not found".
`make bootstrap` therefore also downloads the pinned
`WINDOWS_CRT_PACKAGE` (`Microsoft.VC.14.44.17.14.CRT.Redist.X64.base`,
Makefile) from the Visual Studio release channel's own manifest into
`target/windows-sdk/crt/`, and `make windows-package` puts those DLLs beside
the program in the zip. Three deliberate choices in that:

- **App-local, never installed.** The DLLs sit next to `idle-manager.exe`
  rather than in the system directory, which is the deployment Microsoft's
  redistribution terms allow without an installer — and the only one that
  keeps item 12 task 08's promise of no installer and no administrator
  rights.
- **The `.vsix` from the manifest, not `VC_redist.x64.exe`.** A `.vsix` is a
  plain zip, so `unzip` is the only tool needed; the redistributable installer
  is self-extracting and would put `7z` or `cabextract` on every developer
  machine. The manifest also publishes each payload's `sha256`, which
  `scripts/windows-crt-fetch.sh` checks before unpacking a binary that will be
  handed to someone else.
- **Pinned by package id**, the same way the GTK build is pinned by version.
  The manifest keeps older ids (14.29 through 14.44 at the time of writing),
  so bumping it stays a one-line, one-commit change.

## Bootstrap

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # once, if rustup is missing
make bootstrap                                                   # toolchain, cargo tools, package list
make dev                                                         # run it
```

`make bootstrap` reads `rust-toolchain.toml`, so the pinned compiler is what gets
installed. It also adds the `x86_64-pc-windows-msvc` rustup target, installs
`cargo-xwin`, and runs the gvsbuild download above — the first run downloads
about 300 MiB, and every run after that is a no-op. `make verify` is the full
gate and is what CI should run; it includes `windows-check`.

`.github/workflows/ci.yml` is that CI: one job on `ubuntu-24.04` — the oldest
Linux the application supports — running on every push to `main` and every
pull request targeting it, installing the same system packages named above
plus `clang lld llvm jq zip unzip curl`, then `make bootstrap` and
`make verify`. Four caches (the GTK SDK, the CRT package, `cargo-xwin`'s own
download cache, and the cargo registry) keep a routine run from re-fetching
the ~1.4 GB a cold `make bootstrap` would otherwise download every time.

The Windows loop, once a change needs trying on the Windows 11 VM: `make
windows-package` (roadmap item 12 task 08) writes
`dist/idle-manager-<version>-windows-x64.zip`, then run it in the VM per
[`docs/windows-vm.md`](windows-vm.md).

## Version policy

- The compiler is pinned; `Cargo.lock` is committed; `make release` builds
  `--locked`. A build that works today works next month.
- Application dependencies are pinned to a minor version (`"0.11"`, `"1.0"`), not
  to a patch. Patches are security fixes and should arrive without a commit.
- `make audit` fails on a yanked crate, an advisory, or a licence outside the
  allow-list in `deny.toml`. Widening that list is a deliberate decision.

## Considered and not used

| Not used | Why |
| --- | --- |
| Relm4 | Its component model owns widget lifetime, and the requirements need manual control of when a web view is destroyed and where an off-grid view is mapped |
| Tauri, Electron | The point of the project is low memory; a second runtime beside WebKit is the opposite |
| `tokio` | GTK already has a main loop. `glib`'s executor runs futures on it without a second scheduler |
| Blueprint | Nicer than `.ui` XML, but it is a build tool `make bootstrap` cannot install from crates.io |
| `mod.rs` module files | Rust 2018 style puts the module in `foo.rs` beside `foo/`, which keeps editor tabs distinguishable |
