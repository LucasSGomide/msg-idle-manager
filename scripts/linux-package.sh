#!/usr/bin/env bash
# Packs the optimised Linux program into one AppImage anybody can mark
# executable and run (roadmap item 16 task 03, `FR.3.2`, `FR.3.3`).
#
# What goes into the staged folder, and why each part has to:
#   idle-manager                the program itself, dynamically linked against
#                                the system's own GTK 4 and WebKitGTK — neither
#                                is bundled, which keeps the download small and
#                                the supported systems exactly what
#                                `docs/stack.md` already promises
#   presets/                    the game presets the shell reads at start-up
#   idle-manager.desktop         the desktop entry `vpk` builds the AppImage's
#                                launcher menu entry from
#   idle-manager.png             a plain 256 px icon (roadmap item 16 task 03;
#                                the project carried no app icon before this)
#
# `vpk [linux] pack` (the `[linux]` directive tells Velopack's CLI which
# platform to package for, the same as `[win]` in `windows-package.sh`) wraps
# the staged folder into `IdleManager.AppImage`, the `.nupkg` and
# `releases.linux.json` the update port reads from later. `--delta None`:
# there is nothing yet for a delta to apply against; task 06 turns it on once
# a previous release exists to diff. `--packTitle` and `--categories` are not
# named in this task's own technical details, but Velopack ignores the
# `Name`/`Comment`/`Categories` fields of the staged `.desktop` file and
# synthesises its own placeholder ("IdleManager 0.1.0", "Utility") when they
# are left out — passing them is the difference between a real menu entry and
# a placeholder one.
#
# `PACKAGE` and `CARGO` come from the Makefile, which is where they are
# defined; this script only ever reads them.
set -euo pipefail

: "${PACKAGE:?set by the Makefile}"
CARGO="${CARGO:-cargo}"

command -v vpk >/dev/null || { echo "linux-package: install vpk first (scripts/system-check.sh names the tool)" >&2; exit 1; }
command -v jq >/dev/null || { echo "linux-package: install jq first" >&2; exit 1; }

# The version comes from Cargo.toml through `cargo metadata`, never retyped
# here (naming rule 1): the package's name in `dist/` and the version it
# reports at run time cannot drift apart.
version=$($CARGO metadata --format-version 1 --no-deps \
  | jq -r --arg package "$PACKAGE" '.packages[] | select(.name == $package) | .version')
[[ -n "$version" ]] || { echo "linux-package: could not read $PACKAGE's version" >&2; exit 1; }

pack_id="IdleManager"
staging="dist/.staging/linux"
output_dir="dist/releases/linux"
exe="target/release/$PACKAGE"

[[ -f "$exe" ]] || { echo "linux-package: $exe is missing; run make release" >&2; exit 1; }

# A fresh staging folder every run: a file dropped from the package must
# actually leave the AppImage too, and a stale leftover would hide that.
rm -rf "dist/.staging"
mkdir -p "$staging/presets"

cp "$exe" "$staging/"
cp presets/*.toml "$staging/presets/"
cp release/idle-manager.desktop "$staging/"
cp release/idle-manager.png "$staging/"

rm -rf "$output_dir"
mkdir -p "$output_dir"
vpk "[linux]" pack \
  --packId "$pack_id" \
  --packVersion "$version" \
  --packDir "$staging" \
  --mainExe "$PACKAGE" \
  --icon "$staging/idle-manager.png" \
  --packTitle "Idle Manager" \
  --categories Game \
  --delta None \
  --outputDir "$output_dir"
rm -rf "dist/.staging"
# Only the feed the updater reads ships: `RELEASES-linux` is the legacy format
# and `assets.linux.json` is `vpk upload`'s bookkeeping, and the release
# publishes every file left in this folder.
rm -f "$output_dir/RELEASES-linux" "$output_dir/assets.linux.json"

appimage="$output_dir/$pack_id.AppImage"
nupkg="$output_dir/$pack_id-$version-linux-full.nupkg"
feed="$output_dir/releases.linux.json"
for artefact in "$appimage" "$nupkg" "$feed"; do
  [[ -f "$artefact" ]] || { echo "linux-package: $artefact did not make it into $output_dir" >&2; exit 1; }
done

# Proves the AppImage did not bundle its own copy of GTK or WebKitGTK: the
# extracted binary must resolve both against the *system's* shared libraries,
# never a copy this script's own staging step never put there in the first
# place (roadmap item 16 task 03's own criterion).
extract_dir=$(mktemp -d)
trap 'rm -rf "$extract_dir"' EXIT
(cd "$extract_dir" && "$OLDPWD/$appimage" --appimage-extract >/dev/null)
extracted_exe="$extract_dir/squashfs-root/usr/bin/$PACKAGE"
[[ -f "$extracted_exe" ]] || { echo "linux-package: $extracted_exe did not come out of the AppImage" >&2; exit 1; }

resolved=$(ldd "$extracted_exe")
for library in libgtk-4.so.1 libwebkitgtk-6.0.so.4; do
  line=$(grep "$library" <<<"$resolved" || true)
  [[ -n "$line" ]] || { echo "linux-package: $extracted_exe does not link $library at all" >&2; exit 1; }
  case "$line" in
    *"$extract_dir"*)
      echo "linux-package: $library resolved from inside the AppImage, not the system" >&2
      exit 1
      ;;
  esac
done

echo "linux-package: wrote $appimage, $nupkg and $feed ($(du -h "$appimage" | cut -f1)); $PACKAGE links GTK and WebKitGTK from the system"
