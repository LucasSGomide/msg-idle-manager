# 01 — Verifying every push on GitHub

**Roadmap:** [16](../../roadmap/16-release-pipeline-and-in-app-update/README.md) · **Scope:** back-end · **Depends on:** —

## Context

This application is a desktop program that keeps several browser idle games
running at once, on Linux and on Windows. It already has one command that runs
the whole quality gate on a developer's machine: formatting, lints, tests, a
dependency audit, a check that no crate reaches a layer it must not, and a
check that the planning tables are current. Today that command only runs when
someone remembers to run it. Nothing runs it when work is pushed.

This slice makes GitHub run it. A workflow file tells GitHub Actions to start
a job on an Ubuntu 24.04 machine on every push to the main branch and on every
pull request that targets it. That job installs the handful of system
libraries the build needs, restores caches so the large Windows tool downloads
happen once rather than on every run, installs the pinned toolchain the way a
developer does, and runs the same gate command. A red run means the push broke
something, and it is seen before anything is released.

Ubuntu 24.04 is chosen on purpose: it is the oldest Linux the application
supports, so a build that passes there runs everywhere the project promises.
The workflow file itself stays thin. Every real step is a Make target, so a
developer can repeat any step locally with the same command and get the same
result, and the workflow can never drift into doing something the Makefile does
not.

It is its own slice because every later slice in this item is proven by the
gate this one puts on GitHub, and because it touches only the workflow file,
so it can be written beside the versioning slice without either waiting.

## Technical details

- **Architecture** — add `.github/workflows/ci.yml`: one job `verify` on
  `ubuntu-24.04`, `on: push: branches: [main]` and `on: pull_request:
  branches: [main]`, `permissions: contents: read`; steps: `actions/checkout`,
  `sudo apt-get install -y build-essential pkg-config libgtk-4-dev
  libwebkitgtk-6.0-dev clang lld llvm jq zip unzip curl`, cache restores,
  `make bootstrap`, `make verify`. Nothing in the file runs a command that is
  not a `make` target except checkout, apt and cache.
- **Architecture** — cache `target/windows-sdk/gtk` keyed on the Makefile's
  `GVSBUILD_VERSION` (read with `grep` in a step), `target/windows-sdk/crt`
  keyed on `WINDOWS_CRT_PACKAGE`, `~/.cache/cargo-xwin` keyed on the
  `cargo-xwin` version `make bootstrap` installs, and `~/.cargo/registry` plus
  `~/.cargo/git` keyed on `Cargo.lock`; a hit on all of them makes a routine
  run download none of the ~1.4 GB.
- **Architecture** — `make bootstrap` is not changed here; if the runner's
  preinstalled `rustup` needs the pinned toolchain, `rustup show
  active-toolchain` in bootstrap already installs it from
  `rust-toolchain.toml`.
- **Code standards** — the workflow is documented at its top with one comment
  block saying why 24.04 and why every step is a target (rule 16: why, not
  what); `docs/stack.md`'s "Bootstrap" section gains one paragraph naming the
  workflow as what CI runs.

## Acceptance criteria

- [ ] `(integration)` `make verify` passes locally on the branch before the
      workflow is pushed
- [ ] `(e2e)` the first run of `ci.yml` on a pull request against `main` is
      green, and its log shows `make verify` ending with
      `arch-check: layer boundaries hold` and `roadmap tables are up to date`
- [ ] `(e2e)` a second run with an unchanged `Cargo.lock` and Makefile
      restores all four caches (`Cache restored from key …` for each) and
      downloads neither the gvsbuild zip nor the CRT package
- [ ] `(e2e)` a push that breaks formatting on a throwaway branch makes the
      workflow red at the `fmt-check` step, proving the gate blocks rather than
      reports

## References

- [Roadmap item](../../roadmap/16-release-pipeline-and-in-app-update/README.md)
  — Back-end "Verifying every push"; Technical References on the Ubuntu 24.04
  runner image
- [`docs/requirements.md`](../../requirements.md) — Distribution `FR.1.1`,
  `FR.1.2`, `FR.1.3`
- [`docs/architecture.md`](../../architecture.md) — rule 4
- [`docs/code-standards.md`](../../code-standards.md) — rules 16, 29
- [`docs/stack.md`](../../stack.md) — "System packages", "Bootstrap"

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
