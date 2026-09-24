# 01 — Releasing both builds from GitHub and updating them from inside the app

**Depends on:** R07, R12 · **Verdict:** viable, not yet spiked · **Estimate:** 13

## Context

Idle Manager has no release process. There is no git remote, no CI, no tag, and
the workspace version in the root `Cargo.toml` has been `0.1.0` since the first
commit. Two local commands exist. `make release` builds the Linux binary.
`make windows-package` cross-builds the Windows program with `cargo xwin` and
zips it with its GTK files into `dist/idle-manager-<version>-windows-x64.zip`
(`FR.3.4`). Nothing packages the Linux build: a Linux user builds from source.

The owner wants one flow from "the work on `main` is ready" to "every copy has
updated itself". That means bumping the version, writing release notes,
building and publishing both systems on GitHub, and having a running Idle
Manager notice the new version and install it. This doc compares the tools for
each step, recommends a pipeline, and lists what `/msg-pre-roadmap` still has
to decide.

**How to read the tables.** Every tool claim names the version looked at and
was checked on **2026-09-23**. The last column says how far to trust it:

- **verified (spike)** — run on this machine against this repository; the
  commands and results are in [Spikes run](#spikes-run).
- **verified (source)** — read in the tool's source code, a package archive or
  a build manifest.
- **claimed** — what the tool's documentation says, not tried here.

## 1. Versioning

A *version bump* is the edit that turns `0.1.0` into the next number. The
history already follows *conventional commits*: each subject starts with a type
such as `feat(shell):` or `fix(shell):`. A tool can read those types and pick
the bump: a `feat` means a new feature, so the middle number goes up; a `fix`
means the last number goes up.

Two things about this repository make most tools awkward:

- One shared version. The six crates all say `version.workspace = true`, and the
  only real number is `[workspace.package] version` in the root `Cargo.toml`.
- Nothing goes to crates.io. Every crate is `publish = false`.

Several tools below work through a *release PR*: after each push to `main`, a
bot opens (or updates) a pull request that contains only the version bump and
the new release notes. Merging that pull request is the "ship it" button.

| Tool | Version | What it does for us | What it costs | Verified or claimed |
| --- | --- | --- | --- | --- |
| [`git-cliff`](https://git-cliff.org/docs/) | 2.14.2 (2026-09-18) | `git cliff --bumped-version` reads the commits since the last tag and prints the next version. On this history it printed `v0.2.0`. It does not edit `Cargo.toml`; a Make target has to write the number and refresh `Cargo.lock`. | One static binary, or one Action (`orhun/git-cliff-action`). One `cliff.toml`. | verified (spike) |
| [`release-plz`](https://release-plz.dev/docs/config) | 0.3.169 (2026-09-19) | Rust-specific release-PR bot. It has `git_only = true` for crates never published. With `version_group` on all six crates, a shared `git_tag_name = "v{{ version }}"` and `features_always_increment_minor`, it wrote `0.2.0` into the root `Cargo.toml` and `Cargo.lock`. | It credits each commit to the crate folder the commit touched. With default settings the binary crate looked unchanged and the five libraries looked unreleased, and nothing was written. Even with the working setup, the app's changelog came out **empty**, because almost every `feat` touches `idle-manager-shell`, not the binary crate. | verified (spike) |
| [`release-please`](https://github.com/googleapis/release-please) | 17.11.2 (2026-08-24) | Google's release-PR bot, run as a GitHub Action. It generates its own notes. | Its Rust updater edits only `[package] version`, and it throws `is not a package manifest (might be a cargo workspace)` on a root `Cargo.toml` like ours ([`src/updaters/rust/cargo-toml.ts`](https://github.com/googleapis/release-please/blob/main/src/updaters/rust/cargo-toml.ts)). The workaround is `release-type: simple` plus a generic marker comment on the version line. | verified (source) for the limit; the workaround is claimed |
| [`semantic-release`](https://semantic-release.gitbook.io/) | 25.0.9 (2026-08-05) | Fully automatic, with no release PR: every qualifying push to `main` releases. | A Node toolchain in CI. Cargo support comes only from the community plugin [`semantic-release-cargo`](https://www.npmjs.com/package/semantic-release-cargo), 2.4.2, last published 2025-11-12. There is no moment to edit the notes. | claimed |
| [`cargo-release`](https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md) | 1.1.6 (2026-09-16) | Understands `shared-version`, commits, tags and pushes. | You pass the bump level yourself (`cargo release minor`); it does not read commit types. It writes no notes and creates no GitHub Release. | claimed |
| [`dist` (cargo-dist)](https://axodotdev.github.io/cargo-dist/book/) | 0.32.0 (2026-05-22) | Not a versioning tool. It reacts to a tag you push. See [section 3](#3-building-and-publishing). | — | claimed |

**Recommendation: `git-cliff`, called from two Make targets.**
`make release-notes` would print the notes for the unreleased commits.
`make release-prepare` would compute the next version with
`git cliff --bumped-version`, write it into `[workspace.package]`, run
`cargo update --workspace` so the lockfile matches, and prepend the new section
to `CHANGELOG.md`. A small `prepare-release.yml` workflow, started by hand from
GitHub's Actions tab, runs that target and opens a release PR with the result.
The owner edits the notes in the PR and merges it; the merge creates the tag.
This is the only candidate verified end to end on this history. It uses one
tool for both the number and the notes, and it keeps every command a Make
target. `release-plz` is the fallback, but its crate-per-folder model works
against a one-app workspace.

## 2. Release notes

*Release notes* are the text on the GitHub Release page, and later in the
app's "what's new", that tells a user what changed.

| Tool | Version | What it does for us | What it costs | Verified or claimed |
| --- | --- | --- | --- | --- |
| `git-cliff` | 2.14.2 | `commit_parsers` group `feat` under "New" and `fix` under "Fixed", and `skip = true` drops `docs`, `chore`, `test`, `refactor` and `ci`. `filter_unconventional` drops the `Merge feat/…` subjects. Over the 40 simulated commits (19 `docs`, 1 `chore`, 14 `feat`, 6 `fix`), no `docs` or `chore` line reached the output. | One `cliff.toml`. | verified (spike) |
| `release-plz` | 0.3.169 | Uses git-cliff inside, with the same `commit_parsers`. | Produced an empty changelog for the app (section 1). | verified (spike) |
| `release-please` | 17.11.2 | `changelog-sections` with `hidden: true` hides a commit type. | Only in its own release-PR flow. | claimed ([customizing](https://github.com/googleapis/release-please/blob/main/docs/customizing.md)) |
| GitHub's generated notes | — | Builds notes from merged pull-request titles. | This repository merges branches locally and has no pull requests, so there would be nothing to list. | claimed |

The spike surfaced something the tools can't fix: **the commit subjects are
written for developers**. Real lines from the generated notes include "Wire the
pager's arrows and readout into the window" and "Revert four sidebar details
per owner feedback". They are accurate but mean little to a player.

**Recommendation:** commit a `cliff.toml` that keeps only `feat`, `fix` and
`perf`. Treat its output as a draft that the owner rewrites in the release PR
before merging. The edited section becomes both `CHANGELOG.md` and the GitHub
Release body.

## 3. Building and publishing

A *runner* is the computer GitHub lends a workflow for one run.

**The Linux runner can build both systems.** Checked against the package
archives on 2026-09-23 (Launchpad API, Debian `madison`):

| System | GTK 4 | WebKitGTK 6.0 | Meets 4.10 / 2.42? |
| --- | --- | --- | --- |
| Ubuntu 24.04 (`ubuntu-latest`) | 4.14.5 | 2.52.6 (`libwebkitgtk-6.0-dev` published) | yes |
| Ubuntu 26.04 (this machine, runner `ubuntu-26.04`) | 4.22.4 | 2.52.6 | yes |
| Ubuntu 22.04 | 4.6.9 | no 6.0 package | no |
| Debian 13 | 4.18.6 | 2.52.6 | yes |
| Debian 12 | 4.8.3 | 2.50.6 | no (GTK) |

The [Ubuntu 24.04 runner image](https://github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md)
(version 20260907) ships `rustup` with Rust 1.98.1 (our pinned version), Node
22.23.2 for the roadmap engine, clang 18 and the .NET SDKs 8–10. Today's cross
build needs no clang at all: this machine has no `clang`, `clang-cl` or
`lld-link` on `PATH`. `cargo xwin` downloads its own linker and Microsoft's
headers into `~/.cache/cargo-xwin`, which is 1.1 GB here. The gvsbuild GTK
download is about 300 MiB and unpacks to 1.2 GB. The resulting zip is 39 MB and
the Linux binary 2.9 MB (measured in `dist/` and `target/release/`). **A Windows
runner is not needed to build.** It would only be useful for a smoke test that
launches the `.exe`.

One catch, found in a spike: any Rust HTTPS client built on `rustls` pulls in
`ring`. `ring` compiles C code, so the Windows cross build then **needs
`clang-cl`** (`failed to find tool "clang-cl"` from `ring v0.17.14`). `ring` is
not in today's `Cargo.lock`. `make bootstrap` would have to install clang
locally. On the runner, clang 18 is present, but whether `clang-cl` is on
`PATH` is not verified.

| Option | Version | What it does for us | What it costs | Verified or claimed |
| --- | --- | --- | --- | --- |
| Hand-written workflow calling Make targets | — | `ci.yml` runs `make bootstrap && make verify` on every push. `release.yml`, triggered by a `v*` tag, runs `make verify`, `make release`, `make windows-package` and a new Linux package target, then `gh release create` with the artifacts, checksums and notes. It follows "every command is a Make target" (CLAUDE.md). | We write and maintain two YAML files. Caching the xwin folder and the gvsbuild zip, keyed on `GVSBUILD_VERSION` and the `cargo-xwin` version, avoids about 1.4 GB of downloads per run. | Package floors verified (source); the cross build is verified locally, not on a runner |
| `dist` | 0.32.0 | Generates the whole release workflow, checksums, shell and PowerShell installers, and an updater. | By default it builds Windows on a Windows runner (`github-custom-runners` can override that). It knows nothing about the GTK DLLs or the Visual C++ runtime, so `include` and `github-build-setup` would re-create `scripts/windows-package.sh` in its own config. Its generated YAML is not a Make target. | claimed ([config](https://axodotdev.github.io/cargo-dist/book/reference/config.html)) |

**What it costs in minutes: nothing, while the repository is public.** GitHub's
billing page says "The use of standard GitHub-hosted runners is free: In public
repositories" ([billing](https://docs.github.com/en/billing/concepts/product-billing/github-actions)).
For comparison, a private repository pays $0.006/min for a 2-core Linux runner
and $0.010/min for a 2-core Windows runner. A full release run was not timed
here. It will be dominated by two optimised builds and, on a cold cache, the
downloads above.

**Recommendation:** a hand-written `release.yml` on `ubuntu-24.04` that only
calls Make targets. Build the Linux binary on 24.04, the oldest supported
system, rather than on 26.04, so it runs on every system in the table that
meets the floors.

## 4. The Linux package

A *runtime* here is a shared bundle of libraries that many apps run on top of,
downloaded once. *Bundling* means shipping a library inside our own download
instead of using the copy the system already has. The hard part is WebKitGTK:
it is large, and it starts helper programs (`WebKitWebProcess`,
`WebKitNetworkProcess`) from a folder path compiled into the library.

| Format | Version | GTK 4 / WebKitGTK come from | Size and floors | How it updates | Verified or claimed |
| --- | --- | --- | --- | --- | --- |
| Plain tarball (`.tar.gz`) | — | The system | About 1.3 MB (the 2.9 MB binary, gzipped). Floors unchanged: runs on Ubuntu 24.04+ and Debian 13+, not Ubuntu 22.04 or Debian 12. | The in-app updater renames a new binary over the running one, which is allowed on Linux ([self-replace](https://docs.rs/self-replace/latest/self_replace/)). | Size measured; floors verified (source) |
| `.deb` + our own apt repository | — | The system, through `Depends:` | Small. Same floors. Debian and Ubuntu only. | `apt upgrade`. The app cannot install it without root; it can only tell the user. We would host a GPG-signed repository on GitHub Pages. | claimed |
| AppImage, bundled by hand | — | Bundled | 70 MB and up for WebKit apps ([tauri#6918](https://github.com/orgs/tauri-apps/discussions/6918)). Floors no longer depend on the system. | `zsync`/AppImageUpdate, or Velopack. | claimed. The helper path must be patched inside the library binary ([tauri#2940](https://github.com/tauri-apps/tauri/pull/2940)). That is documented for `webkit2gtk-4.1` only; nothing was found for `webkitgtk-6.0`. Bundled graphics libraries can clash with the host's Mesa ([example](https://github.com/jpbhatt21/integrated-mod-manager/pull/40)), and the `GSK_RENDERER=opengl` re-exec relies on the host's GL. |
| AppImage made by Velopack (`vpk pack`) | 1.2.158 | Whatever folder we give it. Given only our binary, the system's. | Small when thin. Same floors as the tarball. | Velopack downloads to `/var/tmp` and replaces the `.AppImage` file, using `pkexec` if it needs permission. | claimed ([Linux docs](https://docs.velopack.io/packaging/operating-systems/linux)). Whether its AppImage runtime needs `libfuse2`, which Ubuntu 24.04 does not install by default, is **not verified**. |
| Flatpak on Flathub | GNOME runtime 50 | The `org.gnome.Platform` runtime. Its build manifest lists `sdk/webkitgtk-6.0.bst` (WebKitGTK 2.54.0) beside GTK ([`sdk-platform.bst`](https://gitlab.gnome.org/GNOME/gnome-build-meta/-/blob/gnome-50/elements/sdk-platform.bst), lines 53–54). | The app is small. The runtime is a large, one-time, shared download (not measured). Floors are met on **any** distribution with Flatpak, including Ubuntu 22.04 and Debian 12. | `flatpak update` or the software centre. The app itself can detect and request its own update through the portal's `CreateUpdateMonitor` → `UpdateAvailable` → `Update`, and the new version runs after a restart ([portal XML](https://github.com/flatpak/flatpak/blob/main/data/org.freedesktop.portal.Flatpak.xml)). | Runtime contents and portal API verified (source); the rest claimed |
| Flatpak from our own repository on GitHub Pages | — | Same runtime | Same | `flatpak update` against our remote | claimed |

What Flathub costs, per its [requirements](https://docs.flathub.org/docs/for-app-authors/requirements):

- a licence in the metainfo file, a desktop file, and an SVG or 256 px icon;
- a build entirely from source with no network access, so every crate must be
  listed in the manifest;
- an app ID whose domain the project controls. `org.idlemanager.IdleManager`
  (`crates/idle-manager/src/main.rs:42`) needs `idlemanager.org`; otherwise use
  `io.github.<owner>.IdleManager`.

The repository has none of these files today: no `LICENSE`, `*.desktop`, icon
or metainfo. Flathub builds on its own servers, which steps outside "releases
live on GitHub"; the self-hosted repository stays on GitHub. A Flatpak app also
stores its files under `~/.var/app/<id>/`, so an existing user's accounts would
not appear without a one-time migration (claimed). Whether keep-awake, reading
`/proc` for the memory footer, and the phone server work inside the sandbox is
not verified.

**Recommendation:** ship a **thin Velopack AppImage** (our binary, the presets
and a desktop file, with GTK and WebKitGTK from the system). One update library
then covers both systems (section 5), and the floors stay those
`make system-check` already enforces. First, spike it on a clean Ubuntu 24.04:
does it start without `libfuse2`? If it fails, fall back to the **tarball**
with `self_update` (section 5). Don't bundle WebKitGTK into an AppImage. Flatpak
is the right later step if users on older distributions or the software centre
start to matter; it is its own item, not part of this pipeline.

## 5. The Windows update

A running `.exe` cannot be overwritten on Windows, and neither can the roughly
70 GTK DLLs it has loaded. Every option below is a way around that. A *delta
update* downloads only the bytes that changed between two versions rather than
the whole 39 MB zip.

| Tool | Version | What it does for us | What it costs | Verified or claimed |
| --- | --- | --- | --- | --- |
| [Velopack](https://docs.velopack.io/) (`velopack` crate + `vpk` CLI) | 1.2.158 (2026-09-21) | The app calls `check_for_updates`, `download_updates`, then `wait_exit_then_apply_updates`. A separate `Update.exe` waits for the app to exit, then swaps the whole `current` folder (exe **and** DLLs) in one step, so locked files never matter. It kills the app if it hasn't exited within 60 s. `GithubSource` reads GitHub Releases. It makes delta packages. `vpk [win] pack` builds the Windows package from Linux. Its `Portable.zip` updates itself **with no installer**, so it can keep `FR.3.4`. `Setup.exe` installs per user to `%LocalAppData%\{packId}`. | Builds on Linux with the pinned toolchain, with the API as documented. **Fails `make audit` twice**: `derivative` is unmaintained ([RUSTSEC-2024-0388](https://rustsec.org/advisories/RUSTSEC-2024-0388)), and `webpki-roots` is `CDLA-Permissive-2.0`, a licence outside `deny.toml`. Both come through `ureq` → `rustls` → `ring`, which also needs `clang-cl` (section 3). `vpk` needs the .NET SDK, which the runner has. The zip gains `Update.exe`, and the program moves into a `current\` subfolder. | Build, audit and cross-build failure verified (spike); update behaviour claimed ([overview](https://docs.velopack.io/packaging/overview), [integrating](https://docs.velopack.io/integrating/overview), [cross-compiling](https://docs.velopack.io/packaging/cross-compiling), [API](https://docs.rs/velopack/latest/velopack/struct.UpdateManager.html)) |
| [`self_update`](https://docs.rs/self_update/latest/self_update/) | 1.3.0 (2026-09-02) | GitHub backend. `MoveAll` moves a whole archive's files transactionally, and the `signatures` feature verifies with `zipsign`. On Windows it swaps the exe through `self-replace` 1.5.0: move the running exe aside, spawn a copy that deletes itself. | It swaps files **while the app runs**. Its own docs warn that "file locks may cause failures if the process holds files open beyond its own executable", which is exactly the loaded GTK DLLs. A safe Windows update would need a helper that runs after exit, which is Velopack rebuilt by hand. Fine for a single Linux binary. | claimed |
| An updater of our own | — | Download the zip, verify it, unpack it into a sibling `app-0.2.0\` folder, and have a tiny launcher start the newest one. | A second executable, clean-up of old folders, and rollback, all owned by us. | — |
| [`cargo-packager-updater`](https://docs.rs/cargo-packager-updater/latest/cargo_packager_updater/) | 0.2.3 (2025-07-21) | Works without Tauri and verifies with minisign. | On Windows it installs only NSIS or WiX installers, which **conflicts with `FR.3.4`**. It hasn't been released in 14 months. | claimed |
| `tauri-plugin-updater` | 2.12.0 | — | Needs the Tauri runtime, which `docs/stack.md` rejects. **Out.** | claimed |
| [`axoupdater`](https://docs.rs/axoupdater/latest/axoupdater/) (with `dist`) | 0.10.2 (2026-08-12) | A library that updates apps installed by `dist`'s installers. | It swaps one binary through `self-replace`, and it expects `dist`'s install receipt. It has no answer for the DLL folder. | claimed |

**Recommendation: Velopack's portable zip.** It is the only candidate that
handles the loaded DLLs, keeps "unzip and run, no installer, no administrator
rights" (`FR.3.4`), and builds from the Linux runner. Its first task is a spike
in the Windows VM (`docs/windows-vm.md`). Unzip `Portable.zip` into
`C:\idle-manager`, publish a second version to a test release, and confirm:

- the update applies;
- the app restarts;
- `%APPDATA%\idle-manager` is untouched;
- the unsigned `Update.exe` is not blocked.

If the portable zip fails, the per-user `Setup.exe` needs an installer, and
`FR.3.4` would have to be superseded.

## 6. In-app behaviour

**Learning that a new version exists.** The app asks GitHub for the latest
release, through `GET /repos/{owner}/{repo}/releases/latest` or Velopack's
`GithubSource`. Unauthenticated calls are limited to 60 per hour per IP address
([rate limits](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api)).
One check at launch and one a day uses almost none of that.

**The app has no HTTP client today.** Three ways to add one, a decision for
the roadmap item:

- `ureq` + `rustls`, which Velopack brings anyway. That means `ring`,
  `clang-cl` and the two `deny.toml` exceptions above.
- The system's own TLS: `native-tls`, which uses Schannel on Windows and
  OpenSSL on Linux.
- The libraries already linked: libsoup, which WebKitGTK already loads on Linux,
  and WinHTTP through the `windows` crate we already depend on.

The update check belongs behind a port in `idle-manager-core`, the way
`WorkspaceStore` is. Which crate holds the adapter that reaches the network is
an architecture decision; `make arch-check` would gain an edge for it.

**Restarting drops every live game, so the app must never restart by itself.**
What a restart costs today:

- logins survive, because cookies persist (`FR.2.2`, `FR.2.3`);
- workspace restore brings back every account that was running (`FR.18.4`), but
  one at a time through the start queue, which waits up to 30 s per account
  (`LOAD_SETTLE_TIMEOUT_SECS`, item 07);
- a kept-awake game makes no progress while it is down;
- the phone link (item 13) drops.

So the recommended behaviour:

1. Check at launch and once a day.
2. Download in the background.
3. Tell the user: "Version 0.3.0 is ready. It installs when you quit Idle
   Manager", with a *Restart now* button and a link to the notes.
4. Apply on the next normal quit, through `wait_exit_then_apply_updates`.

The workspace has to be saved before the swap. The window's `close-request`
already flushes the pending save synchronously (item 07's `Saver::flush`), so
an update-triggered quit must go through that same close path, not a hard exit.
Velopack kills the app after 60 s; closing four web views has not been timed
against that. Design rule 9's message strip carries "one line and a dismiss
button … and nothing else", so an update notice with an action button needs a
new design rule, or a different widget.

## 7. Trust

A *checksum* (SHA-256) proves a download arrived intact. A *signature* proves
who made it: the owner signs with a private key kept as a GitHub Actions
secret, and the app checks the signature with the matching public key compiled
into it.

| Mechanism | Version | What it does for us | What it costs | Verified or claimed |
| --- | --- | --- | --- | --- |
| `SHA256SUMS` file on the release | — | Catches a corrupted download. | Nothing. It does not stop an attacker who can edit the release, because they would edit the sums too. | — |
| minisign ([`minisign-verify`](https://crates.io/crates/minisign-verify)) | 0.2.5 (2026-03-03) | Small Ed25519 signatures. The app verifies the downloaded package against a public key it carries, before applying anything. | One key pair. The secret lives in Actions secrets, and losing it means shipping a new key in a new version. | claimed |
| `zipsign` | 0.2.1 (2026-02-02) | The same idea, built into `self_update`'s `signatures` feature. | Only with `self_update`. | claimed |
| Velopack's own check | 1.2.158 | Checks each package against the hash in its release feed (the crate depends on `sha1` and `sha2`). | The feed comes from the same release, so this catches corruption, not a hijacked release. Add a minisign check between download and apply. | claimed; not read in source |
| GPG | — | Signs an apt repository. | Only needed if we choose `.deb`. | claimed |
| GitHub artifact attestations (Sigstore) | — | `actions/attest-build-provenance` records which workflow built which file. People check it with `gh attestation verify`. Public repositories use the Sigstore public instance ([docs](https://docs.github.com/en/actions/concepts/security/artifact-attestations)). | Free. Checking it inside the app would need the `sigstore` crate (0.14.0), which is heavy. It serves people, not the updater. | claimed |

**Windows SmartScreen and code signing** ([options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options),
[reputation](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/smartscreen-reputation),
both read 2026-09-23):

- **Unsigned** shows "Windows protected your PC", and the user must press
  *Run anyway*. Reputation belongs to each file's hash, so it restarts at zero
  with every version. On Windows 11, **Smart App Control** "will block
  execution of unsigned files unless the file has a positive reputation" and
  applies "to all executable files, not just those downloaded from the
  Internet". An unsigned update may be blocked outright on those machines.
- **Azure Artifact Signing** costs about $9.99/month. **Individuals must be in
  the USA or Canada.** Organizations may be in the USA, Canada, the EU or the UK.
- An **OV certificate** costs $150–300/year, with the key on a hardware token or
  cloud HSM. An **EV certificate** costs $400+/year and "no longer bypass[es]
  SmartScreen".
- **SignPath Foundation** is free for open source, but only with an
  OSI-approved licence and no commercial dual-licensing, no proprietary
  component, a build on CI, MFA for every team member, and a published
  code-signing policy ([terms](https://signpath.org/terms)). The repository has
  no licence yet.
- Signed or not, a new publisher sees warnings until reputation builds, which
  takes "several weeks and hundreds of clean installs".
- Signing from Linux needs a cross-platform tool such as JSign, because
  `signtool.exe` runs only on Windows (Velopack's cross-compiling page).

**Recommendation:** publish `SHA256SUMS` and attestations from day one, and
sign every update package with minisign so the app refuses anything the owner's
key did not sign. Windows code signing is the owner's call (see decisions). The
free route needs a licence first.

## 8. What an update must never touch

The app keeps nothing next to its executable. `crates/idle-manager-store/src/paths.rs`
resolves every path through `ProjectDirs::from("", "", "idle-manager")`:

- `config_dir()` holds `presets/`, `sessions.toml` and `phone.toml`;
- `data_dir()` holds `profiles/<id>/`, meaning cookies, storage, `state.toml`
  and zoom, plus, on Windows, `webview2/`.

On Linux these are `~/.config/idle-manager` and `~/.local/share/idle-manager`.
On Windows they sit under `%APPDATA%\idle-manager` (Roaming), per the
`directories` crate's convention (claimed).

| Candidate | What it replaces | Leaves the data alone? |
| --- | --- | --- |
| Velopack (Windows) | The `current` folder. Uninstalling a `Setup.exe` install deletes `%LocalAppData%\{packId}`. | Yes. The data lives in Roaming, not Local, and no path in `paths.rs` uses Local. Keep the pack ID from ever naming a folder the app writes to. |
| Velopack (Linux AppImage) | The `.AppImage` file | Yes |
| `self_update` / tarball | The files it is given | Yes |
| `.deb` | Package-owned files under `/usr` | Yes |
| Flatpak | The app inside the runtime | Yes, but it can't see it either: data moves to `~/.var/app/<id>/`, so the first run needs a migration |

Two effects of an update on existing data need a decision rather than a tool:

- **Rolling back is not free.** `sessions.toml` has a format version (3 since
  `FR.22.2`). An older build meeting a newer file quarantines it to
  `sessions.bad` (item 07). Going back one version can lose the saved workspace.
- **New built-in presets never reach existing users.**
  `crates/idle-manager-store/src/preset.rs:219` seeds the shipped presets only
  when the folder does not exist yet. That protects a user's edits, but an
  update that adds or fixes a game preset changes nothing for anyone who
  already ran the app.

## Recommended pipeline

Steps 1–6 are shared. The two systems part at step 7.

1. A push to `main` runs `ci.yml`: `make bootstrap && make verify` on
   `ubuntu-24.04`.
2. When `main` is ready, the owner starts *Prepare release* from the Actions
   tab. It runs `make release-prepare`: `git cliff --bumped-version` → write
   `[workspace.package] version` → `cargo update --workspace` → prepend
   `CHANGELOG.md`. Then it opens a release PR.
3. The owner rewrites the draft notes in the PR and merges it.
4. The merge tags `vX.Y.Z` and triggers `release.yml` on `ubuntu-24.04`, with
   the xwin and gvsbuild caches restored: `make verify` → `make release` →
   `make windows-package` → `vpk pack` for Linux and `vpk [win] pack` for
   Windows → minisign every package → write `SHA256SUMS` → attest.
5. `gh release create vX.Y.Z` uploads the packages, Velopack's release feed,
   the signatures, the checksums and the notes.
6. Within a day, every running copy calls GitHub, sees `vX.Y.Z`, downloads it
   in the background, and checks the minisign signature.
7. **Linux:** the app shows "ready, installs when you quit". The user quits, and
   Velopack replaces the `.AppImage`. On next launch the workspace restores
   every account through the start queue.
8. **Windows:** the same notice. The user quits, and `Update.exe` swaps
   `current\` under the unzipped folder, with no installer and no administrator
   rights. On next launch the workspace restores from `%APPDATA%\idle-manager`.

```mermaid
flowchart LR
  A[push to main] --> B[ci.yml: make verify]
  B --> C[Prepare release: git-cliff bump + notes]
  C --> D[release PR, notes edited]
  D -->|merge, tag vX.Y.Z| E[release.yml on ubuntu-24.04]
  E --> F[make release / make windows-package]
  F --> G[vpk pack Linux + Windows, minisign, SHA256SUMS]
  G --> H[GitHub Release]
  H --> I[app: check, download, verify]
  I --> J[user quits]
  J --> K[swap: AppImage / current folder]
  K --> L[relaunch, workspace restore]
```

## Spikes run

All spikes ran in the session scratchpad; nothing in the repository changed.

- **`git-cliff` 2.14.2** on a clone with `v0.1.0` tagged 40 commits back. It
  printed `v0.2.0` with `features_always_bump_minor = true`. The notes held 14
  "New" and 6 "Fixed" lines, and zero `docs` or `chore` lines. Two commits were
  reported as skipped for lacking a group.
- **`release-plz` 0.3.169** on the same clone:
  - With one tagged package it reported "already up to date" for the binary
    and "initial release" for the five libraries, and wrote nothing.
  - With all six crates in one `version_group`, a shared tag pattern and
    `features_always_increment_minor` (committed first, since it refuses a
    dirty tree), it wrote `0.2.0` into `Cargo.toml` and `Cargo.lock`.
    `CHANGELOG.md` came out 1 byte long.
- **`velopack` 1.2.158**, in a scratch crate calling `GithubSource::new`,
  `check_for_updates`, `download_updates` and `wait_exit_then_apply_updates`:
  - `cargo check` passes on Linux with Rust 1.98.1.
  - `cargo deny` with this repository's `deny.toml` rejects `webpki-roots`
    (`CDLA-Permissive-2.0`) and flags `derivative` (RUSTSEC-2024-0388).
  - `cargo xwin build --target x86_64-pc-windows-msvc` fails in `ring`'s build
    script for lack of `clang-cl`.
- **Not run:** a Docker `ubuntu:24.04` build of the whole pipeline. No Docker
  daemon was running, and starting one needs `sudo`. The floors were checked
  against the package archives instead.

## Findings

- `git-cliff` alone handles the version bump and the notes on this history,
  and skips `docs` and `chore`. `release-plz` needs a six-crate config and
  still produces empty notes. `release-please` can't edit a workspace root.
- One `ubuntu-24.04` runner can build and verify both systems, and costs
  nothing while the repository is public.
- Only Velopack handles the locked Windows DLLs without an installer. Its
  portable zip can keep `FR.3.4`, but that is not yet tried in the VM. Adding
  it costs two `deny.toml` exceptions and `clang-cl` for the cross build.
- The GNOME Flatpak runtime ships WebKitGTK 6.0, but Flathub needs a licence,
  metainfo, an icon and a data migration.

**User needs surfaced**

- "When `main` is ready, I want one action that publishes Linux and Windows
  builds on GitHub, with no building by hand."
- "I want the version number and a draft of the notes written for me, and I
  want to edit the notes before they go out."
- "As a user, the notes should say what changed for me, not list docs and
  chores."
- "As a user, I want the app to tell me a new version exists without me
  checking GitHub."
- "As a user, I don't want the app to restart on its own and drop my running
  games. I choose when."
- "After an update, my accounts, logins, workspaces, presets and zoom are where
  I left them."
- "On Windows, I want to keep unzipping and running it with no installer and no
  admin rights, and still get updates."
- "On Linux, I want a download I can run without building from source."
- "I want to be sure the update I install is the one the owner published."
- "Every push should run `make verify`, so a broken `main` is caught before a
  release."

**Decisions the research could not make**

1. Keep `FR.3.4` (portable zip, if the VM spike passes) or supersede it with a
   per-user installer.
2. The Linux format: a thin Velopack AppImage, a tarball, a `.deb`, or Flatpak.
   Flathub takes the build off GitHub and moves the data.
3. Windows code signing: ship unsigned (SmartScreen, and Smart App Control may
   block updates), apply to SignPath (needs a licence), buy an OV certificate,
   or use Azure (individuals in the USA or Canada only).
4. The project licence, which Flathub and SignPath both need (prompt 15's
   audit).
5. Update consent: check automatically? Download automatically? An opt-out
   setting? And the notice's design, since design rule 9 allows no action
   button.
6. The HTTP and TLS stack, and whether to accept the `deny.toml` exceptions and
   `clang-cl` in `make bootstrap`.
7. Which crate holds the network adapter for the update-check port.
8. Whether rollback to an older version must be supported, given
   `sessions.toml`'s format version.
9. Whether an update should deliver new or fixed built-in presets to existing
   users.
10. Version policy: `feat` bumps the minor number while on `0.x`. When is
    `1.0`? Is there a pre-release (beta) channel?
