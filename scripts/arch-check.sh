#!/usr/bin/env bash
# Fails when a crate reaches a layer it is not allowed to reach.
#
# Cargo already makes dependency cycles impossible. What it has no opinion on is
# direction: nothing stops the domain crate from picking up GTK, or the shell
# from reaching around the composition root into persistence. This checks that.
#
# Each rule is one crate followed by the crates it must never pull in, directly
# or transitively. See docs/architecture.md for why each edge is forbidden.
set -euo pipefail

rules=(
  "idle-manager-core     gtk4 gdk4 glib gio webkit6 serde toml"
  "idle-manager-store    gtk4 gdk4 webkit6"
  "idle-manager-metrics  gtk4 gdk4 webkit6"
  "idle-manager-shell    idle-manager-store idle-manager-metrics"
)

status=0
for rule in "${rules[@]}"; do
  read -r crate forbidden <<<"$rule"
  reached=$(cargo tree --package "$crate" --edges normal --prefix none | awk '{print $1}' | sort -u)

  for dependency in $forbidden; do
    if grep -qx "$dependency" <<<"$reached"; then
      echo "arch-check: $crate must not depend on $dependency (docs/architecture.md)" >&2
      status=1
    fi
  done
done

[[ $status -eq 0 ]] && echo "arch-check: layer boundaries hold"
exit $status
