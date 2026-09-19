#!/usr/bin/env bash
# Downloads and unpacks the gvsbuild GTK 4 release zip that
# `make windows-check` and `make windows-build` point `PKG_CONFIG_PATH` at.
#
# Idempotent: a `gtk4.pc` already sitting under `WINDOWS_SDK_DIR` is taken as
# proof the unpack already happened, so a repeat `make bootstrap` does not
# re-download the ~300 MiB archive. `GVSBUILD_VERSION` and `WINDOWS_SDK_DIR`
# come from the Makefile, which is where the version is pinned
# (`docs/stack.md`'s version-policy rule) — this script only ever reads them.
set -euo pipefail

: "${GVSBUILD_VERSION:?set by the Makefile}"
: "${WINDOWS_SDK_DIR:?set by the Makefile}"

pkgconfig_marker="$WINDOWS_SDK_DIR/lib/pkgconfig/gtk4.pc"
if [[ -f "$pkgconfig_marker" ]]; then
  echo "windows-sdk-fetch: $pkgconfig_marker already present; skipping the download"
  exit 0
fi

url="https://github.com/wingtk/gvsbuild/releases/download/${GVSBUILD_VERSION}/GTK4_Gvsbuild_${GVSBUILD_VERSION}_x64.zip"
archive="$(mktemp)"
trap 'rm -f "$archive"' EXIT

echo "windows-sdk-fetch: downloading GTK 4 Gvsbuild $GVSBUILD_VERSION (about 300 MiB)..."
curl --proto '=https' --tlsv1.2 -fL -o "$archive" "$url"

mkdir -p "$WINDOWS_SDK_DIR"
echo "windows-sdk-fetch: unpacking into $WINDOWS_SDK_DIR..."
unzip -q -o "$archive" -d "$WINDOWS_SDK_DIR"

if [[ ! -f "$pkgconfig_marker" ]]; then
  echo "windows-sdk-fetch: unpacked, but $pkgconfig_marker is still missing — the release layout may have changed" >&2
  exit 1
fi

echo "windows-sdk-fetch: GTK 4 Gvsbuild $GVSBUILD_VERSION ready under $WINDOWS_SDK_DIR"
