#!/usr/bin/env bash
# Fails when a crate reaches a layer it is not allowed to reach.
#
# Cargo already makes dependency cycles impossible. What it has no opinion on is
# direction: nothing stops the domain crate from picking up GTK, or the shell
# from reaching around the composition root into persistence. This checks that.
#
# Each rule is one crate followed by the crates it must never pull in, directly
# or transitively. See docs/architecture.md for why each edge is forbidden.
#
# `cargo tree` only walks the host target (Linux here), so the Windows-only
# `wry`/`webview2-com`/`gdk4-win32`/`windows` edges (roadmap item 12) never show
# up in that pass at all — a second pass re-runs every rule with
# `--target x86_64-pc-windows-msvc` so a Windows-only edge is caught too.
set -euo pipefail

rules=(
  "idle-manager-core     gtk4 gdk4 glib gio webkit6 serde toml wry webview2-com gdk4-win32 windows"
  "idle-manager-store    gtk4 gdk4 webkit6 wry webview2-com gdk4-win32 windows idle-manager-remote"
  "idle-manager-metrics  gtk4 gdk4 webkit6 wry webview2-com gdk4-win32 idle-manager-remote"
  "idle-manager-shell    idle-manager-store idle-manager-metrics idle-manager-remote"
  "idle-manager-remote   gtk4 gdk4 glib gio webkit6 wry webview2-com gdk4-win32 idle-manager-shell idle-manager-store idle-manager-metrics"
)

check_pass() {
  local target_args=("$@")
  local status=0
  for rule in "${rules[@]}"; do
    read -r crate forbidden <<<"$rule"
    local reached
    reached=$(cargo tree --package "$crate" --edges normal --prefix none "${target_args[@]}" | awk '{print $1}' | sort -u)

    for dependency in $forbidden; do
      if grep -qx "$dependency" <<<"$reached"; then
        echo "arch-check: $crate must not depend on $dependency${target_args[*]:+ (${target_args[*]})} (docs/architecture.md)" >&2
        status=1
      fi
    done
  done
  return $status
}

status=0
check_pass || status=1
check_pass --target x86_64-pc-windows-msvc || status=1

[[ $status -eq 0 ]] && echo "arch-check: layer boundaries hold"
exit $status
