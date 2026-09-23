# Goal: Research how to automate versioning, release notes, publishing to GitHub and in-app updates for the Linux and Windows builds, as input for `/msg-pre-roadmap`

**Status:** not executed
**Rating:** —

## Context

Idle Manager has no release process. There is no git remote, no CI, no tag, and
the workspace version in the root `Cargo.toml` has sat at `0.1.0` since the
first commit. What exists is local: `make release` compiles the optimised Linux
binary from the committed lockfile, and `make windows-package` cross-builds the
Windows program from Linux with `cargo xwin` and zips it into
`dist/idle-manager-<version>-windows-x64.zip` (roadmap item 12, `FR.3.4`). There
is no Linux package of any kind — a Linux user today builds from source with
`make bootstrap`. Nobody on the project owns a Windows computer; the Windows
build is checked in the VM described in `docs/windows-vm.md`.

The owner wants one flow that goes from "the work on `main` is ready" to "every
user's copy updates itself": the version is bumped, release notes are written,
both builds are produced and published to GitHub, and a running Idle Manager
learns a newer version exists and installs it. The owner does not yet know
which tools do this, so finding and comparing them is the job. The result feeds
`/msg-pre-roadmap`, which will turn it into user needs and requirements; this
prompt is research only and stops at the exploration doc.

**What the research has to answer.** Take these as the branches to cover, not
an exhaustive list:

- **Versioning.** How the next version is chosen and written into the single
  workspace version. The history already uses conventional commits
  (`feat(shell): …`, `fix(shell): …`, `docs(roadmap): …`), so tools that derive
  the bump from commit types are worth a close look — for example
  `release-plz`, `cargo-release`, `release-please`, `semantic-release`,
  `cargo-dist`. Say which ones understand a Cargo workspace with one shared
  version and crates that are never published to crates.io.
- **Release notes.** How they are produced from the history — `git-cliff` and
  whatever the versioning tools above generate — and whether `docs(…)` and
  `chore(…)` commits can be kept out of user-facing notes.
- **Building and publishing.** A GitHub Actions workflow that runs `make verify`
  and builds both targets, then creates a GitHub Release with the artifacts.
  Establish whether the Windows cross-build (`cargo xwin`, the pinned gvsbuild
  GTK zip, the app-local CRT DLLs) runs on a Linux runner as it does locally,
  or whether a Windows runner is needed, and what each costs in minutes.
- **The Linux package.** Which format a Linux user installs, and how that format
  updates: AppImage (with its own update mechanism), Flatpak (through Flathub or
  a self-hosted repository), a `.deb`/apt repository, or a plain tarball. The
  hard part is WebKitGTK 6.0 and GTK 4 — say for each format whether they are
  bundled, taken from a runtime, or required from the system, and what that does
  to download size and the `make system-check` floors (GTK 4.10, WebKitGTK
  2.42).
- **The Windows update.** How a portable zip replaces itself: a self-update
  crate such as `self_update`, a framework such as Velopack, or an updater of
  our own that downloads, verifies and swaps the executable and its DLLs. A
  running `.exe` cannot overwrite itself on Windows; say how each option gets
  around that.
- **In-app behaviour.** How the application learns there is a new version (the
  GitHub Releases API, a manifest file), whether it asks or updates silently,
  when the swap happens given that restarting drops every live game, and how
  the choice interacts with workspace restore (roadmap item 07) bringing the
  accounts back after a restart.
- **Trust.** How a downloaded update is verified before it runs — checksums,
  signatures (minisign, GPG, Sigstore), and what Windows SmartScreen does with
  an unsigned executable. Record what code signing costs; do not assume it is
  or is not wanted.
- **What an update must never touch.** The owner's accounts, presets, sessions
  and per-account profiles live under the XDG config and data folders on Linux
  and their Windows equivalents. Confirm each candidate leaves them alone.

## Constraints

1. **Research only.** Write the exploration doc and its row in
   `docs/explorations/README.md`, nothing else. No code, no workflow file, no
   `docs/requirements.md` rows, no roadmap item. A throwaway spike in the
   scratchpad is allowed if it settles a question; record what it proved.
2. **Respect `FR.3.4`'s promise, but flag conflicts rather than hide them.** The
   Windows build is a zip that needs no installer and no administrator rights.
   A tool that keeps that promise is preferred. A tool that needs an installer
   is not ruled out: record the conflict as a trade-off, plainly, so
   `/msg-pre-roadmap` can decide whether to supersede `FR.3.4`.
3. **Hosting is GitHub, in a public repository.** Releases live on GitHub
   Releases and the automation runs on GitHub Actions. Do not compare other
   hosts.
4. **Fit the project as it is.** Every developer command is a Makefile target
   and `make verify` is the gate CI runs (CLAUDE.md). The compiler is pinned
   and builds are `--locked` (`docs/stack.md`, "Version policy"). Tauri is
   already rejected (`docs/stack.md`, "Considered and not used"), so its updater
   counts only if it can be used on its own, without the Tauri runtime.
5. **Cite and date every claim about a tool.** Name the version looked at and
   link its documentation. Separate what was verified — read in the source,
   tried in a spike — from what the tool's documentation merely claims.
6. **Follow the exploration shape.** Number the doc with the next free
   exploration number, give it a **Verdict** (`viable, verified` ·
   `viable, not yet spiked` · `blocked` · `ruled out`), and end it with
   `## Findings`. File names follow `docs/naming.md`.
7. **Run `make roadmap-sync`, then `make roadmap-check`,** after adding the doc.
8. **No branch needed.** Documentation and planning edits only, per CLAUDE.md.

## Tone

Direct, clear, avoiding jargon, explaining like a teacher addressing a
beginner. The owner has never set up a release pipeline: when a term like
"release PR", "delta update" or "runtime" first appears, say what it means in
a few words.

## Output

`docs/explorations/NN-release-automation-and-auto-update.md`, with:

- one section per branch above, each comparing its candidates in a short table
  (tool, version, what it does for us, what it costs, verified or claimed) and
  ending in a recommendation;
- a recommended end-to-end pipeline, drawn as the ordered steps from merging
  to `main` to a user running the new version, on each system;
- `## Findings`, closing with two lists written for `/msg-pre-roadmap`: the
  user needs the research surfaced, phrased as the owner or a user would say
  them, and the decisions the research could not make on its own.

Plus a row in `docs/explorations/README.md`, and a short note back naming the
recommended pipeline and the open decisions.
