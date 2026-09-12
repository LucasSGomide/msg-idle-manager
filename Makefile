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
export PATH := $(HOME)/.cargo/bin$(if $(NODE_BIN),:$(NODE_BIN)):$(PATH)

CARGO ?= cargo
PACKAGE := idle-manager
RUST_LOG ?= idle_manager=debug,idle_manager_core=debug,idle_manager_shell=debug

.DEFAULT_GOAL := help

.PHONY: help bootstrap system-check dev run watch build release check fmt fmt-check lint \
        test doc audit arch-check verify clean memory-report

help:  ## list every target
	@grep -hE '^[a-zA-Z0-9_-]+:.*##' $(MAKEFILE_LIST) | sort | awk 'BEGIN { FS = ":.*## " } { printf "  \033[36m%-14s\033[0m %s\n", $$1, $$2 }'

# --- setup ------------------------------------------------------------------

bootstrap:  ## install the pinned toolchain and the cargo tools, then check the C libraries
	@command -v rustup >/dev/null || { echo "install rustup first: https://rustup.rs"; exit 1; }
	rustup show active-toolchain
	$(CARGO) install --locked cargo-nextest cargo-deny cargo-watch
	$(CARGO) fetch
	@./scripts/system-check.sh

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

audit:  ## check dependencies for advisories, licences and duplicate versions
	$(CARGO) deny check

arch-check:  ## fail if a crate depends on a layer it must not
	./scripts/arch-check.sh

doc: system-check  ## build the API docs and open them
	$(CARGO) doc --workspace --no-deps --open

verify: fmt-check lint test audit arch-check roadmap-check  ## everything CI runs

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
