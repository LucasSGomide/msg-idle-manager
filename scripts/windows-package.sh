#!/usr/bin/env bash
# Stages the optimised Windows program and the parts of GTK it needs at run
# time, then hands the staged folder to Velopack's packer (roadmap item 16
# task 03; before this, `FR.2.1`, `FR.3.4` were met with a plain zip — see
# item 12 task 08 — Velopack now owns both the packaging and, from task 06
# onward, the in-app update).
#
# What goes into the staged folder, and why each part has to:
#   idle-manager.exe                          the program itself
#   *.dll                                     the GTK stack gvsbuild built
#   vcruntime140.dll, msvcp140.dll, …         the Visual C++ runtime they all
#                                             import and none of them ship
#                                             (scripts/windows-crt-fetch.sh)
#   share/glib-2.0/schemas/gschemas.compiled  GSettings aborts without it
#   share/icons/{Adwaita,hicolor}             or every icon draws as a square
#   lib/gdk-pixbuf-2.0/                       the image loaders, cache included
#
# The WebView2 runtime is deliberately *not* staged: Windows already ships it,
# and bundling a second copy would be both large and wrong (item 12 task 08's
# own context). `WebView2Loader.dll` is only carried if the executable
# actually imports it — see the check below, which records the answer either
# way.
#
# `vpk [win] pack` (the `[win]` directive, not a placeholder — it tells
# Velopack's CLI to cross-package for Windows from this Linux host) turns the
# staged folder into `IdleManager-win-Portable.zip` (a small `Update.exe`
# beside a `current/` folder holding the program — unzip anywhere, double
# click, still no installer) plus the `.nupkg` and `releases.win.json` a later
# task's update port reads. It also writes `IdleManager-win-Setup.exe`, an
# installer this project does not want (no admin rights, no install step);
# deleting it rather than passing `--noInst` keeps the command exactly the one
# this task's own technical details name, and costs one `rm`.
# `--runtime win-x64` names the architecture explicitly — the cross-build only
# ever targets `x86_64-pc-windows-msvc`, and leaving it out makes `vpk`
# guess `x86` and warn on every run. `--delta None` is this vpk release's
# spelling of what task 03 wrote as `--noDelta`: 1.2.158 renamed the flag to a
# mode selector between when the task was written and when it shipped; there
# is nothing here for a delta to apply against yet (`--noDelta` itself no
# longer parses), and a real delta channel is task 06's job once a previous
# release exists to diff against.
#
# The packages land in `dist/releases/win/`, which `scripts/windows-vm/compose.yml`
# bind-mounts into the VM as drive `Z:`, so the file that gets handed out is
# the same file that gets tested (`docs/windows-vm.md`).
#
# `WINDOWS_SDK_DIR`, `WINDOWS_TARGET`, `PACKAGE` and `CARGO` come from the
# Makefile, which is where they are defined; this script only ever reads them.
set -euo pipefail

: "${WINDOWS_SDK_DIR:?set by the Makefile}"
: "${WINDOWS_TARGET:?set by the Makefile}"
: "${WINDOWS_CRT_DIR:?set by the Makefile}"
: "${PACKAGE:?set by the Makefile}"
CARGO="${CARGO:-cargo}"

command -v vpk >/dev/null || { echo "windows-package: install vpk first (scripts/system-check.sh names the tool)" >&2; exit 1; }
command -v jq >/dev/null || { echo "windows-package: install jq first" >&2; exit 1; }
command -v unzip >/dev/null || { echo "windows-package: install unzip first" >&2; exit 1; }

# The version comes from Cargo.toml through `cargo metadata`, never retyped
# here (naming rule 1): the package's name in `dist/` and the version it
# reports at run time cannot drift apart.
version=$($CARGO metadata --format-version 1 --no-deps \
  | jq -r --arg package "$PACKAGE" '.packages[] | select(.name == $package) | .version')
[[ -n "$version" ]] || { echo "windows-package: could not read $PACKAGE's version" >&2; exit 1; }

# The Velopack pack id (naming rule 5's crate/directory convention does not
# apply here — this is the id the update client and the release feed key on,
# and it is `IdleManager` on both systems, matching the `.desktop` file and
# the AppImage's own pack id in `scripts/linux-package.sh`).
pack_id="IdleManager"
staging="dist/.staging/win"
output_dir="dist/releases/win"
exe="target/$WINDOWS_TARGET/release/$PACKAGE.exe"

[[ -f "$exe" ]] || { echo "windows-package: $exe is missing; run make windows-build PROFILE=release" >&2; exit 1; }

# A fresh staging folder every run: a file dropped from the package (a DLL that
# left gvsbuild, say) must actually leave the package too, and a stale
# leftover would hide that.
rm -rf "dist/.staging"
mkdir -p "$staging/share/glib-2.0/schemas" "$staging/share/icons"

cp "$exe" "$staging/"
cp "$WINDOWS_SDK_DIR"/bin/*.dll "$staging/"

# The Visual C++ runtime, beside the program rather than installed into the
# system: app-local deployment, which is what keeps the zip runnable on a
# clean Windows with no redistributable installer and no administrator rights.
# Every DLL in the package folder goes in, not just the three that show up as
# direct imports, because those three have their own dependencies among the
# rest (`msvcp140.dll` reaches `msvcp140_atomic_wait.dll`, for instance).
[[ -d "$WINDOWS_CRT_DIR" ]] || { echo "windows-package: $WINDOWS_CRT_DIR is missing; run scripts/windows-crt-fetch.sh" >&2; exit 1; }
cp "$WINDOWS_CRT_DIR"/*.dll "$staging/"
cp "$WINDOWS_SDK_DIR/share/glib-2.0/schemas/gschemas.compiled" "$staging/share/glib-2.0/schemas/"
cp -r "$WINDOWS_SDK_DIR/share/icons/Adwaita" "$staging/share/icons/"
cp -r "$WINDOWS_SDK_DIR/share/icons/hicolor" "$staging/share/icons/"
mkdir -p "$staging/lib"
cp -r "$WINDOWS_SDK_DIR/lib/gdk-pixbuf-2.0" "$staging/lib/"

# `loaders.cache` ships as gvsbuild wrote it, because gvsbuild writes the loader
# paths *relative* to the program's own folder (`lib\gdk-pixbuf-2.0\...`), not
# as absolute paths off the build machine. Unzipping anywhere therefore works
# with no regeneration step. Checked here rather than assumed: an absolute
# `C:\` path in there would mean silently broken image loading on every machine
# but the one that built it.
cache="$staging/lib/gdk-pixbuf-2.0/2.10.0/loaders.cache"
if [[ -f "$cache" ]] && grep -q '^"[A-Za-z]:' "$cache"; then
  echo "windows-package: loaders.cache holds absolute paths; image loaders would not resolve" >&2
  exit 1
fi

# The three the whole package would fail to start without. Checked by name
# rather than trusted, because "the zip is there" and "the zip can start" are
# different claims and only this one is cheap to make from Linux.
for required in vcruntime140.dll vcruntime140_1.dll msvcp140.dll; do
  [[ -f "$staging/$required" ]] \
    || { echo "windows-package: $required did not make it into the package" >&2; exit 1; }
done

# Does the program import the WebView2 loader as a DLL, or is it linked in?
# task 08 asks for the answer to be checked rather than guessed, because
# getting it wrong either bloats the zip or ships one that cannot start.
objdump=""
for candidate in llvm-objdump x86_64-w64-mingw32-objdump objdump; do
  command -v "$candidate" >/dev/null && { objdump="$candidate"; break; }
done

if [[ -z "$objdump" ]]; then
  echo "windows-package: no objdump found; not checking for a WebView2Loader.dll import"
else
  if [[ "$objdump" == "llvm-objdump" ]]; then
    imports=$("$objdump" --private-headers "$exe")
  else
    imports=$("$objdump" -p "$exe")
  fi

  if grep -qi "WebView2Loader.dll" <<<"$imports"; then
    loader=$(find target/"$WINDOWS_TARGET"/release -name WebView2Loader.dll -print -quit)
    [[ -n "$loader" ]] || { echo "windows-package: the exe imports WebView2Loader.dll but none was found to ship" >&2; exit 1; }
    cp "$loader" "$staging/"
    echo "windows-package: WebView2Loader.dll imported; carried into the package"
  else
    echo "windows-package: WebView2Loader.dll is not imported (linked statically); not carried"
  fi
fi

rm -rf "$output_dir"
mkdir -p "$output_dir"
vpk "[win]" pack \
  --packId "$pack_id" \
  --packVersion "$version" \
  --packDir "$staging" \
  --mainExe "$PACKAGE.exe" \
  --runtime win-x64 \
  --delta None \
  --outputDir "$output_dir"
rm -rf "dist/.staging"

# The setup bundle is Velopack's default output, not something this project
# asked for (no installer, no administrator rights — item 12 task 08's own
# promise); deleting it here is cheaper than teaching `vpk` a flag that would
# also need to survive whatever the CLI calls it next release.
rm -f "$output_dir/$pack_id-win-Setup.exe"

portable="$output_dir/$pack_id-win-Portable.zip"
nupkg="$output_dir/$pack_id-$version-full.nupkg"
feed="$output_dir/releases.win.json"
for artefact in "$portable" "$nupkg" "$feed"; do
  [[ -f "$artefact" ]] || { echo "windows-package: $artefact did not make it into $output_dir" >&2; exit 1; }
done

# Read into a variable rather than piping `unzip -l` straight into `grep -q`:
# `grep -q` closes its end of the pipe on the first match, `unzip` then dies of
# `SIGPIPE`, and `pipefail` turns that dead-pipe write into a false failure
# even though the match was found.
listing=$(unzip -l "$portable")

if grep -q 'Update\.exe$' <<<"$listing"; then
  echo "windows-package: $portable carries Update.exe"
else
  echo "windows-package: $portable is missing Update.exe" >&2
  exit 1
fi

if grep -q "current/$PACKAGE.exe\$" <<<"$listing"; then
  echo "windows-package: $portable carries current/$PACKAGE.exe"
else
  echo "windows-package: $portable is missing current/$PACKAGE.exe" >&2
  exit 1
fi

echo "windows-package: wrote $portable, $nupkg and $feed ($(du -h "$portable" | cut -f1))"
