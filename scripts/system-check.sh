#!/usr/bin/env bash
# Reports the C libraries cargo cannot install for you.
#
# gtk4-sys and webkit6-sys resolve their linker flags through pkg-config at
# build time, so a missing package surfaces as a build-script failure hundreds
# of lines into a compile. Checking first turns that into one line.
#
# Versions are checked for the same reason. The floors are not ours to pick:
# webkit6 0.6 names gtk::Accessible, which gtk4 0.11 gates behind its v4_10
# feature, so the app is built with that feature on and cannot link against an
# older GTK; webkit6-sys 0.6 declares 2.40 as its own floor. Too old a library
# passes an existence check and then fails deep in a build script.
set -uo pipefail

# <pkg-config module> <minimum version> <Debian package providing it>
modules=(
  "gtk4          4.10 libgtk-4-dev"
  "webkitgtk-6.0 2.40 libwebkitgtk-6.0-dev"
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

if [[ ${#missing[@]} -eq 0 && ${#outdated[@]} -eq 0 ]]; then
  echo "system-check: GTK 4 and WebKitGTK development files present"
  exit 0
fi

if [[ ${#missing[@]} -gt 0 ]]; then
  echo "system-check: missing ${missing[*]}" >&2
  echo "  sudo apt install ${missing[*]}" >&2
fi

for entry in "${outdated[@]}"; do
  echo "system-check: $entry — upgrade the distribution package" >&2
done

exit 1
