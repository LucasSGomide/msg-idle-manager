# Every command a developer runs lives here. `make` on its own lists them.
#
# Targets are thin wrappers around cargo — the wrapper exists so the flags that
# matter (workspace-wide, all targets, warnings as errors) are not retyped and
# not forgotten. `make verify` is the whole gate, and is what CI should run.

# rustup installs into ~/.cargo/bin and asks the user's shell profile to add it
# to PATH. A fresh terminal, a non-login shell, or a profile the installer
# could not edit then has cargo on disk but not on PATH. Prepend it here so the
# targets work without `source "$HOME/.cargo/env"` first. A missing directory on
# PATH is harmless; the shim it points at is the toolchain rust-toolchain.toml
# already pins. Override the whole thing with `make CARGO=/path/to/cargo`.
#
# node (for the roadmap engine) has the same problem when it comes from nvm,
# which only wires up PATH in an interactive shell. Add the newest nvm node if
# one is installed and nothing set NODE_BIN; a system node already on PATH is
# untouched.
NODE_BIN ?= $(lastword $(sort $(wildcard $(HOME)/.nvm/versions/node/*/bin)))
#
# `clang-cl` and `lld-link` (roadmap item 16) have the same problem in another
# shape: Ubuntu's `clang`/`lld`/`llvm` packages put only versioned names on
# PATH and keep the plain ones in `/usr/lib/llvm-<N>/bin`. Add the newest of
# those, so CI and a fresh machine need no profile edit; override LLVM_BIN for
# an LLVM unpacked elsewhere. `vpk` is a .NET global tool, which lands in
# ~/.dotnet/tools and is only on PATH once the .NET profile script has run.
LLVM_BIN ?= $(lastword $(sort $(wildcard /usr/lib/llvm-*/bin)))
export PATH := $(HOME)/.cargo/bin$(if $(NODE_BIN),:$(NODE_BIN))$(if $(LLVM_BIN),:$(LLVM_BIN)):$(HOME)/.dotnet/tools:$(PATH)

CARGO ?= cargo
PACKAGE := idle-manager
RUST_LOG ?= idle_manager=debug,idle_manager_core=debug,idle_manager_shell=debug

# Roadmap item 12: cross-compiling and cross-linting the Windows build from
# Linux. The version is pinned here, not in the fetch script, so bumping it is
# a one-line, one-commit change (`docs/stack.md`'s version-policy rule).
GVSBUILD_VERSION := 2026.8.0
WINDOWS_TARGET := x86_64-pc-windows-msvc
WINDOWS_SDK_DIR := target/windows-sdk/gtk
WINDOWS_PKG_CONFIG := target/windows-sdk/pkg-config-wrapper.sh
# The Visual C++ runtime the release zip carries beside the program (task 08).
# gvsbuild's DLLs and ours all import it and none of them ship it, so without
# this a clean Windows install cannot start the program at all. Pinned by
# package id, the same way the GTK build above is pinned by version; the
# manifest keeps older ids, so bumping this stays a one-line change.
WINDOWS_CRT_PACKAGE := Microsoft.VC.14.44.17.14.CRT.Redist.X64.base
WINDOWS_CRT_DIR := target/windows-sdk/crt

.DEFAULT_GOAL := help

.PHONY: help bootstrap system-check dev run watch build release check fmt fmt-check lint \
        test doc audit arch-check windows-check windows-build windows-package linux-package verify clean \
        memory-report release-version release-notes release-prepare release-tools-test \
        release-sign release-checksums

help:  ## list every target
	@grep -hE '^[a-zA-Z0-9_-]+:.*##' $(MAKEFILE_LIST) | sort | awk 'BEGIN { FS = ":.*## " } { printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2 }'

# --- setup ------------------------------------------------------------------

bootstrap:  ## install the pinned toolchain and the cargo tools, then check the C libraries
	@command -v rustup >/dev/null || { echo "install rustup first: https://rustup.rs"; exit 1; }
	rustup show active-toolchain
	rustup target add $(WINDOWS_TARGET)
	$(CARGO) install --locked cargo-nextest cargo-deny cargo-watch cargo-xwin git-cliff
	$(CARGO) fetch
	@./scripts/system-check.sh
	GVSBUILD_VERSION=$(GVSBUILD_VERSION) WINDOWS_SDK_DIR=$(WINDOWS_SDK_DIR) ./scripts/windows-sdk-fetch.sh
	WINDOWS_CRT_DIR=$(WINDOWS_CRT_DIR) WINDOWS_CRT_PACKAGE=$(WINDOWS_CRT_PACKAGE) ./scripts/windows-crt-fetch.sh

system-check:  ## report the C libraries cargo cannot install for you
	@./scripts/system-check.sh

# --- running ----------------------------------------------------------------

dev: system-check  ## run the app with debug logging and full backtraces
	RUST_LOG=$(RUST_LOG) RUST_BACKTRACE=1 $(CARGO) run --package $(PACKAGE)

run: system-check  ## run the app the way a user would
	$(CARGO) run --package $(PACKAGE)

watch: system-check  ## rebuild and rerun on every save
	RUST_LOG=$(RUST_LOG) $(CARGO) watch --clear --exec 'run --package $(PACKAGE)'

# --- building ---------------------------------------------------------------

build: system-check  ## compile the whole workspace, unoptimised
	$(CARGO) build --workspace

release: system-check  ## compile the optimised binary from the committed lockfile
	$(CARGO) build --workspace --release --locked

check: system-check  ## type-check everything, including tests, without producing binaries
	$(CARGO) check --workspace --all-targets

clean:  ## delete every build artefact
	$(CARGO) clean

# --- quality ----------------------------------------------------------------

fmt:  ## format every crate
	$(CARGO) fmt --all

fmt-check:  ## fail on anything unformatted
	$(CARGO) fmt --all --check

lint: system-check  ## clippy over the workspace, warnings are errors
	$(CARGO) clippy --workspace --all-targets -- --deny warnings

# --no-tests=warn so an empty workspace, or a crate that has no test yet, does
# not fail the gate. nextest exits non-zero on an empty run by default.
test: system-check  ## run the test suite, doc tests included
	$(CARGO) nextest run --workspace --no-tests=warn
	$(CARGO) test --workspace --doc
	$(MAKE) release-tools-test

audit:  ## check dependencies for advisories, licences and duplicate versions
	$(CARGO) deny check

arch-check:  ## fail if a crate depends on a layer it must not
	./scripts/arch-check.sh

# --- releasing ----------------------------------------------------------

# The version is never typed in by hand: every commit already says what kind
# of change it carries (`feat`, `fix`, or one that moves nothing), so asking a
# person to also pick the number is asking them to agree with history that has
# already answered (docs/code-standards.md rule 16). `cliff.toml` is the one
# place the mapping from commit type to version bump is written down, and
# these three targets are the only things allowed to read it.

release-version:  ## print the version the next release would ship, or nothing if there is nothing to release
	@bumped=$$(git cliff --bumped-version 2>/dev/null | sed 's/^v//'); \
	current=$$(git describe --tags --abbrev=0 --match 'v[0-9]*' 2>/dev/null | sed 's/^v//'); \
	if [ -n "$$bumped" ] && [ "$$bumped" != "$$current" ]; then echo "$$bumped"; fi

release-notes:  ## print the release notes for the commits since the last tag
	@git cliff --unreleased --strip all

release-prepare:  ## write the version into Cargo.toml/Cargo.lock and prepend CHANGELOG.md
	./scripts/release-prepare.sh

release-tools-test:  ## pin release-version/-notes/-prepare's behaviour against a throwaway clone
	./scripts/tests/release-prepare-test.sh

# Roadmap item 16 task 04: the owner's private key never touches the
# repository or a log — the workflow writes the `MINISIGN_SECRET_KEY` Actions
# secret to a file named by `MINISIGN_SECRET_KEY_FILE` and deletes it once
# this target has run. A missing variable fails loudly rather than silently
# skipping every package.
release-sign:  ## sign every file under dist/releases/*/ with minisign, skipping ones already signed
	@./scripts/system-check.sh minisign
	@[ -n "$$MINISIGN_SECRET_KEY_FILE" ] || { echo "release-sign: set MINISIGN_SECRET_KEY_FILE to the secret key's path" >&2; exit 1; }
	for f in dist/releases/*/*; do \
		case "$$f" in \
			*.minisig) continue ;; \
		esac; \
		if [ -f "$$f.minisig" ]; then \
			echo "release-sign: $$f is already signed"; \
			continue; \
		fi; \
		minisign -S -s "$$MINISIGN_SECRET_KEY_FILE" -m "$$f"; \
	done

release-checksums:  ## write dist/releases/SHA256SUMS over every packaged asset
	cd dist/releases && sha256sum */* > SHA256SUMS

doc: system-check  ## build the API docs and open them
	$(CARGO) doc --workspace --no-deps --open

verify: fmt-check lint test audit arch-check windows-check roadmap-check  ## everything CI runs

# --- windows (roadmap item 12) -----------------------------------------------

# `pkg-config` trusts the prefix baked into each `.pc` file at gvsbuild's own
# build time (`C:/gtk-build/...`), which is meaningless here — `--define-prefix`
# makes it compute the prefix from the `.pc` file's own location instead. The
# `pkg-config` crate reads `PKG_CONFIG` as a full override naming the binary to
# run, so a tiny wrapper script is the way to inject that one flag (measured
# working 2026-09-17; task 02's own "Unverified" bullet on the roadmap item).
$(WINDOWS_PKG_CONFIG):
	@mkdir -p $(dir $@)
	@printf '#!/bin/sh\nexec pkg-config --define-prefix "$$@"\n' > $@
	@chmod +x $@

windows-check: $(WINDOWS_PKG_CONFIG)  ## type-check and lint the Windows build from Linux
	@./scripts/system-check.sh clang-cl lld-link
	# `cargo xwin clippy`, not plain `cargo clippy --target`: the update crate's
	# `velopack` dependency reaches `ring` (roadmap item 16 task 03), whose build
	# script compiles C and needs the MSVC headers `cargo xwin` downloads and
	# points a real cross C compiler (`clang-cl`, on `PATH`) at — plain clippy
	# never sets either up.
	PKG_CONFIG_ALLOW_CROSS=1 PKG_CONFIG_PATH=$(abspath $(WINDOWS_SDK_DIR))/lib/pkgconfig PKG_CONFIG=$(abspath $(WINDOWS_PKG_CONFIG)) \
		$(CARGO) xwin clippy --workspace --all-targets --target $(WINDOWS_TARGET) -- --deny warnings

# PROFILE=release for an optimised build, e.g. `make windows-build PROFILE=release`.
windows-build: $(WINDOWS_PKG_CONFIG)  ## cross-compile idle-manager.exe into dist/idle-manager-dev/
	@./scripts/system-check.sh clang-cl lld-link
	PKG_CONFIG_ALLOW_CROSS=1 PKG_CONFIG_PATH=$(abspath $(WINDOWS_SDK_DIR))/lib/pkgconfig PKG_CONFIG=$(abspath $(WINDOWS_PKG_CONFIG)) \
		$(CARGO) xwin build --locked --target $(WINDOWS_TARGET) -p $(PACKAGE) $(if $(filter release,$(PROFILE)),--release)
	@mkdir -p dist/idle-manager-dev
	cp target/$(WINDOWS_TARGET)/$(if $(filter release,$(PROFILE)),release,debug)/idle-manager.exe dist/idle-manager-dev/
	cp $(WINDOWS_SDK_DIR)/bin/*.dll dist/idle-manager-dev/

windows-package: $(WINDOWS_PKG_CONFIG)  ## build the release exe and pack it into a Velopack portable zip for handing out
	@./scripts/system-check.sh vpk
	$(MAKE) windows-build PROFILE=release
	WINDOWS_CRT_DIR=$(WINDOWS_CRT_DIR) WINDOWS_CRT_PACKAGE=$(WINDOWS_CRT_PACKAGE) ./scripts/windows-crt-fetch.sh
	WINDOWS_SDK_DIR=$(WINDOWS_SDK_DIR) WINDOWS_TARGET=$(WINDOWS_TARGET) PACKAGE=$(PACKAGE) \
		WINDOWS_CRT_DIR=$(WINDOWS_CRT_DIR) CARGO=$(CARGO) ./scripts/windows-package.sh

# --- linux (roadmap item 16 task 03) -----------------------------------------

linux-package:  ## build the release binary and pack it into an AppImage for handing out
	@./scripts/system-check.sh vpk
	$(MAKE) release
	PACKAGE=$(PACKAGE) CARGO=$(CARGO) ./scripts/linux-package.sh

# --- diagnosis --------------------------------------------------------------

# What the running application costs in memory, per process and summed, taken
# the same way the sidebar footer takes it (roadmap item 05). Pass arguments
# through ARGS, e.g. `make memory-report ARGS='--soak 30 --out mem.tsv'`.
memory-report:  ## report the running app's memory per process (ARGS='--soak 30 --out mem.tsv')
	./scripts/memory-report.sh $(ARGS)

# --- msg-roadmap:start
.PHONY: roadmap-sync roadmap-check

roadmap-sync:  ## recompute every derived status and table under docs/
	node scripts/roadmap-sync.mjs

roadmap-check:  ## fail on a stale table, a bad dependency, or a missing project.yml path
	node scripts/roadmap-sync.mjs --check
# --- msg-roadmap:end
