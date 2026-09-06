# Stack

What the project is built from, at which version, and why. Reference material —
the rules that constrain it live in [`architecture.md`](architecture.md) and
[`code-standards.md`](code-standards.md).

Versions below were the current stable releases on 2026-09-06. Bumping one is a
deliberate act: change it here, change it in the manifest, run `make verify`.

## Target

Linux desktop, GTK 4 on both X11 and Wayland. Nothing else is supported, and
nothing in the code should pretend otherwise — the memory accounting reads
`/proc` and the web engine is WebKitGTK.

## Language and toolchain

| Piece | Version | Why |
| --- | --- | --- |
| Rust | 1.98.1, pinned in `rust-toolchain.toml` | A pinned compiler means a warning that fails CI fails locally too |
| Edition | 2024 | The default since 1.85; `unsafe_op_in_unsafe_fn` and RPIT capture rules are the ones we want |
| Cargo resolver | `"3"` | Respects `rust-version`, so a dependency cannot silently raise the MSRV |

## GUI

| Crate | Version | Role |
| --- | --- | --- |
| `gtk4` | 0.11 | Windows, the grid, the sidebar |
| `webkit6` | 0.6 | One `WebKitWebView` and one `WebKitNetworkSession` per game session |
| `glib` / `gio` | 0.22 | Main context, async, GObject plumbing |

These four move together: `webkit6` 0.6 is built against `gtk4` 0.11 and
`glib`/`gio` 0.22, so all four are bumped in one commit or not at all.

`gtk4` is depended on with its `v4_10` feature enabled. That is not a choice:
`webkit6` 0.6 names `gtk::Accessible` unconditionally, `gtk4` 0.11 gates that
type behind `v4_10`, and nothing in `webkit6` turns the feature on — without it
the workspace does not compile. It fixes the floors below.

## Libraries

| Crate | Version | Role |
| --- | --- | --- |
| `serde` | 1.0 | Derive for the persisted record types, in `store` only |
| `toml` | 1.1 | Presets and the session file — hand-editable, which the requirements ask for |
| `directories` | 6.0 | XDG config and data paths, so nothing is hardcoded |
| `thiserror` | 2.0 | Error enums in every library crate |
| `anyhow` | 1.0 | The binary's top-level error type |
| `tracing` + `tracing-subscriber` | 0.1 / 0.3 | Structured logs, filtered by `RUST_LOG` |

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

## Bootstrap

```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh   # once, if rustup is missing
make bootstrap                                                   # toolchain, cargo tools, package list
make dev                                                         # run it
```

`make bootstrap` reads `rust-toolchain.toml`, so the pinned compiler is what gets
installed. `make verify` is the full gate and is what CI should run.

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
