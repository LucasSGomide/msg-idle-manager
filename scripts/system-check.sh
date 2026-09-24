#!/usr/bin/env bash
# Reports the C libraries and cross-packaging tools cargo cannot install for
# you.
#
# gtk4-sys and webkit6-sys resolve their linker flags through pkg-config at
# build time, so a missing package surfaces as a build-script failure hundreds
# of lines into a compile. Checking first turns that into one line.
#
# Versions are checked for the same reason. The floors are not ours to pick:
# webkit6 0.6 names gtk::Accessible, which gtk4 0.11 gates behind its v4_10
# feature, so the app is built with that feature on and cannot link against an
# older GTK; webkit6-sys 0.6 declares 2.40 as its own floor, and the shell
# raises that to 2.42 by enabling webkit6's v2_42 feature for the settings
# feature list keep-awake flips per session. Too old a library passes an
# existence check and then fails deep in a build script.
#
# The GTK check above runs unconditionally: every build needs it. The tools
# below run only when named as arguments, so an ordinary `make build` or
# `make dev` never fails over a tool only the Windows cross-build or the
# packaging targets need (roadmap item 16 task 03):
#
#   clang-cl, lld-link  the C compiler and linker `cargo xwin` points at
#                        `ring` needs both — `velopack`'s dependency on `ureq`
#                        reaches it — and cross-compiling C to
#                        `x86_64-pc-windows-msvc` needs a real `clang-cl`,
#                        which `cargo clippy --target` alone never sets up
#                        (`make windows-check`, `make windows-build`)
#   vpk                  Velopack's own CLI, the packer both `windows-package`
#                        and `linux-package` hand the staged folder to
#
# `./scripts/system-check.sh clang-cl lld-link vpk` names exactly the tools a
# caller needs; the Makefile passes the ones each target actually uses.
set -uo pipefail

# <tool name> <one-line install instruction>
#
# `apt install clang lld llvm` alone is not enough (verified 2026-09-24,
# roadmap item 16 task 03's own Blocker): Ubuntu's packages put only the
# *versioned* names (`clang-cl-21`, `lld-link-21`, …) in `/usr/bin`; the
# unversioned names `cargo xwin` actually looks for live in
# `/usr/lib/llvm-<N>/bin`, which is not on `PATH` by default.
tool_install_line() {
  case "$1" in
    clang-cl | lld-link) echo "apt install clang lld llvm && export PATH=\"\$(echo /usr/lib/llvm-*/bin):\$PATH\"" ;;
    vpk) echo "dotnet tool install -g vpk" ;;
    *) echo "install $1" ;;
  esac
}

# <pkg-config module> <minimum version> <Debian package providing it>
modules=(
  "gtk4          4.10 libgtk-4-dev"
  "webkitgtk-6.0 2.42 libwebkitgtk-6.0-dev"
)

apt_install="sudo apt install build-essential pkg-config libgtk-4-dev libwebkitgtk-6.0-dev"

if ! command -v pkg-config >/dev/null; then
  echo "system-check: pkg-config is missing, so nothing else can be probed" >&2
  echo "  $apt_install" >&2
  exit 1
fi

missing=()
outdated=()
for entry in "${modules[@]}"; do
  read -r module minimum package <<<"$entry"
  if ! pkg-config --exists "$module"; then
    missing+=("$package")
  elif ! pkg-config --atleast-version="$minimum" "$module"; then
    outdated+=("$module $(pkg-config --modversion "$module") < $minimum")
  fi
done

status=0

if [[ ${#missing[@]} -eq 0 && ${#outdated[@]} -eq 0 ]]; then
  echo "system-check: GTK 4 and WebKitGTK development files present"
else
  status=1
  if [[ ${#missing[@]} -gt 0 ]]; then
    echo "system-check: missing ${missing[*]}" >&2
    echo "  sudo apt install ${missing[*]}" >&2
  fi
  for entry in "${outdated[@]}"; do
    echo "system-check: $entry — upgrade the distribution package" >&2
  done
fi

# Only the tools this call was told to look for (see the header comment):
# `command -v` finds a cached shim just as well as a `PATH` entry, so this
# passes whether the tool was installed by apt, unpacked by hand, or restored
# from `cargo-xwin`'s own cache.
for tool in "$@"; do
  if command -v "$tool" >/dev/null; then
    echo "system-check: $tool present"
  else
    echo "system-check: missing $tool" >&2
    echo "  $(tool_install_line "$tool")" >&2
    status=1
  fi
done

exit $status
