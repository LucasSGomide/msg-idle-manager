#!/usr/bin/env bash
# Downloads the Microsoft Visual C++ runtime DLLs that `make windows-package`
# ships beside the program (roadmap item 12 task 08).
#
# Why the zip has to carry them: 66 of the 67 GTK DLLs gvsbuild builds, and the
# program itself, import `vcruntime140.dll` / `msvcp140.dll`, and gvsbuild
# ships neither. Without them a clean Windows install answers a double-click
# with "VCRUNTIME140.dll was not found" — which would break task 08's whole
# promise of no installer and no administrator rights. Microsoft's own
# redistribution terms allow exactly this app-local deployment: the DLLs sit
# beside the executable, not in the system directory, so nothing is installed
# and nothing else on the machine is touched.
#
# Where they come from: the Visual Studio release channel's own manifest. The
# package is a `.vsix`, which is a plain zip, so `unzip` is the only tool this
# needs — the alternative, `VC_redist.x64.exe`, is a self-extracting installer
# that would need `7z` or `cabextract` on every developer machine.
#
# Idempotent: the DLLs already sitting in `WINDOWS_CRT_DIR` are taken as proof
# the download happened, so a repeat `make bootstrap` is a no-op.
# `WINDOWS_CRT_DIR` and `WINDOWS_CRT_PACKAGE` come from the Makefile, which is
# where the version is pinned (`docs/stack.md`'s version-policy rule) — this
# script only ever reads them.
set -euo pipefail

: "${WINDOWS_CRT_DIR:?set by the Makefile}"
: "${WINDOWS_CRT_PACKAGE:?set by the Makefile}"

command -v jq >/dev/null || { echo "windows-crt-fetch: install jq first" >&2; exit 1; }

marker="$WINDOWS_CRT_DIR/vcruntime140.dll"
if [[ -f "$marker" ]]; then
  echo "windows-crt-fetch: $marker already present; skipping the download"
  exit 0
fi

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

echo "windows-crt-fetch: reading the Visual Studio release channel..."
curl --proto '=https' --tlsv1.2 -fsSL https://aka.ms/vs/17/release/channel -o "$work/channel.json"

manifest_url=$(jq -r '
  .channelItems[]
  | select(.id == "Microsoft.VisualStudio.Manifests.VisualStudio")
  | .payloads[0].url
' "$work/channel.json")
[[ -n "$manifest_url" ]] || { echo "windows-crt-fetch: the channel names no Visual Studio manifest" >&2; exit 1; }

# The manifest is ~18 MiB of JSON listing every package in the release. Only
# one line of it matters here, but there is no narrower published index.
echo "windows-crt-fetch: downloading the package manifest (about 18 MiB)..."
curl --proto '=https' --tlsv1.2 -fsSL "$manifest_url" -o "$work/vs.vsman"

payload=$(jq -r --arg package "$WINDOWS_CRT_PACKAGE" '
  .packages[]
  | select(.id | ascii_downcase == ($package | ascii_downcase))
  | .payloads[0]
  | "\(.url)\t\(.sha256 // "")"
' "$work/vs.vsman" | head -1)
[[ -n "$payload" ]] || {
  echo "windows-crt-fetch: $WINDOWS_CRT_PACKAGE is not in the current manifest." >&2
  echo "windows-crt-fetch: pick a version the manifest still carries and update WINDOWS_CRT_PACKAGE in the Makefile." >&2
  exit 1
}

url=${payload%%$'\t'*}
sha=${payload##*$'\t'}

echo "windows-crt-fetch: downloading $WINDOWS_CRT_PACKAGE..."
curl --proto '=https' --tlsv1.2 -fsSL "$url" -o "$work/crt.vsix"

# The manifest publishes each payload's digest; a binary from the network that
# will be shipped to someone else's machine is checked against it before it is
# ever unpacked.
if [[ -n "$sha" ]]; then
  echo "$sha  $work/crt.vsix" | sha256sum --check --quiet \
    || { echo "windows-crt-fetch: the download does not match the manifest's sha256" >&2; exit 1; }
  echo "windows-crt-fetch: sha256 matches the manifest"
else
  echo "windows-crt-fetch: the manifest published no sha256 for this payload" >&2
fi

# Only the redistributable folder. The same vsix also carries a
# `debug_nonredist` tree, which Microsoft's terms specifically do not allow
# redistributing — hence the narrow path rather than a blanket extract.
mkdir -p "$WINDOWS_CRT_DIR"
unzip -qo -j "$work/crt.vsix" 'Contents/VC/Redist/MSVC/*/x64/Microsoft.VC143.CRT/*.dll' -d "$WINDOWS_CRT_DIR"

for required in vcruntime140.dll vcruntime140_1.dll msvcp140.dll; do
  [[ -f "$WINDOWS_CRT_DIR/$required" ]] \
    || { echo "windows-crt-fetch: $required is missing from the unpacked package" >&2; exit 1; }
done

echo "windows-crt-fetch: unpacked $(find "$WINDOWS_CRT_DIR" -name '*.dll' | wc -l) runtime DLLs into $WINDOWS_CRT_DIR"
