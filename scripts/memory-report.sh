#!/usr/bin/env bash
# Reports what the running idle-manager costs in memory, per process and summed.
#
# This is the reference the on-screen footer is checked against: it is the
# simpler of the two readers and the one whose source a person can take in at a
# glance, so when the two disagree this one is right (roadmap item 05, task 03).
#
# Two modes, two questions:
#   memory-report.sh                  what is it costing right now, per process
#   memory-report.sh --soak 30 --out mem.tsv
#                                     and is that figure going anywhere
#
# Finding the processes is the part that has to be right. WebKitGTK starts each
# rendering process inside two nested bwrap sandboxes, so the process holding
# most of the memory is a grandchild of the application, not a direct child
# (FR.19.1). This reads the whole process table once, builds a parent -> child
# map, and walks down from the application's own pid. A match on command name
# would collect another user's browser; a match on direct children would miss
# five sixths of the memory.
set -uo pipefail

# --- named constants, with their units (code standards rule 5) ---------------

# The executable name the application is located by when no pid is given.
readonly APP_EXECUTABLE_NAME="idle-manager"
# How often soak mode takes a sample, in seconds, when --soak is given no value.
readonly DEFAULT_SOAK_INTERVAL_SECS=30
# Every memory figure this script prints or writes is in kibibytes, the unit
# /proc/<pid>/smaps_rollup reports. Conversion to MiB is left to the reader.
readonly MEMORY_UNIT="KiB"

usage() {
  cat >&2 <<EOF
usage: memory-report.sh [-p PID] [--soak [SECS]] [--out FILE]

  -p, --pid PID     account the tree rooted at PID instead of searching for
                    the "$APP_EXECUTABLE_NAME" executable (use when a debug and
                    a release build are running at once)
      --soak [SECS] sample every SECS seconds (default $DEFAULT_SOAK_INTERVAL_SECS)
                    and append one tab-separated row per sample; runs until
                    interrupted
      --out FILE    where soak mode appends its rows (required with --soak)
EOF
  exit 2
}

# --- argument parsing -------------------------------------------------------

pid=""
soak_interval=""
soak_out=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    -p|--pid)
      pid="${2:-}"; [[ -n "$pid" ]] || usage; shift 2 ;;
    --soak)
      if [[ "${2:-}" =~ ^[0-9]+$ ]]; then
        soak_interval="$2"; shift 2
      else
        soak_interval="$DEFAULT_SOAK_INTERVAL_SECS"; shift
      fi ;;
    --out)
      soak_out="${2:-}"; [[ -n "$soak_out" ]] || usage; shift 2 ;;
    -h|--help)
      usage ;;
    *)
      echo "memory-report.sh: unexpected argument '$1'" >&2; usage ;;
  esac
done

if [[ -n "$soak_interval" && -z "$soak_out" ]]; then
  echo "memory-report.sh: --soak needs --out FILE to append to" >&2
  exit 2
fi

# --- locating the application ----------------------------------------------

# Echoes the application's own pid, or nothing when it is not running.
find_app_pid() {
  if [[ -n "$pid" ]]; then
    [[ -d "/proc/$pid" ]] && echo "$pid"
    return
  fi
  # The comm field is truncated to 15 bytes by the kernel, which "idle-manager"
  # fits inside, so an exact match on it avoids catching this script or an
  # editor with the path open.
  local candidate
  for candidate in /proc/[0-9]*; do
    [[ -r "$candidate/comm" ]] || continue
    if [[ "$(cat "$candidate/comm" 2>/dev/null)" == "$APP_EXECUTABLE_NAME" ]]; then
      basename "$candidate"
      return
    fi
  done
}

# --- the process-tree walk -----------------------------------------------

# Fills the associative array CHILDREN with "ppid -> space-separated child pids"
# and COMM with "pid -> command name", in one pass over the process table.
declare -A CHILDREN
declare -A COMM

read_process_table() {
  local dir entry_pid name ppid
  for dir in /proc/[0-9]*; do
    entry_pid="${dir#/proc/}"
    # A process that exits between the glob and this read is normal, not fatal:
    # skip it and carry on (task 03 acceptance).
    [[ -r "$dir/status" ]] || continue
    name="$(sed -n 's/^Name:\t//p' "$dir/status" 2>/dev/null)" || continue
    ppid="$(sed -n 's/^PPid:\t//p' "$dir/status" 2>/dev/null)" || continue
    [[ -n "$ppid" ]] || continue
    COMM[$entry_pid]="$name"
    CHILDREN[$ppid]="${CHILDREN[$ppid]:-} $entry_pid"
  done
}

# Echoes the given pid and every descendant of it, one per line, breadth first.
descend_from() {
  local queue=("$1") current
  while [[ ${#queue[@]} -gt 0 ]]; do
    current="${queue[0]}"
    queue=("${queue[@]:1}")
    echo "$current"
    local child
    for child in ${CHILDREN[$current]:-}; do
      queue+=("$child")
    done
  done
}

# --- the per-process figures ---------------------------------------------

# Echoes "PSS RSS" in KiB for the given pid, or nothing when the pid is gone.
#
# PSS is the Pss line of /proc/<pid>/smaps_rollup, falling back to summing Pss
# across /proc/<pid>/smaps where the rollup is absent or unreadable — the same
# pair of sources idle-manager-metrics reads (task 02). RSS comes from the same
# rollup, or /proc/<pid>/status where it is not there.
process_figures() {
  local target="$1" rollup="/proc/$1/smaps_rollup" pss="" rss=""

  if [[ -r "$rollup" ]]; then
    pss="$(awk '/^Pss:/ {sum += $2} END {if (NR > 0) print sum+0}' "$rollup" 2>/dev/null)"
    rss="$(awk '/^Rss:/ {sum += $2} END {if (NR > 0) print sum+0}' "$rollup" 2>/dev/null)"
  fi

  if [[ -z "$pss" && -r "/proc/$target/smaps" ]]; then
    pss="$(awk '/^Pss:/ {sum += $2} END {print sum+0}' "/proc/$target/smaps" 2>/dev/null)"
  fi
  if [[ -z "$rss" && -r "/proc/$target/status" ]]; then
    rss="$(sed -n 's/^VmRSS:[[:space:]]*\([0-9]*\).*/\1/p' "/proc/$target/status" 2>/dev/null)"
  fi

  [[ -n "$pss" || -n "$rss" ]] || return
  echo "${pss:-0} ${rss:-0}"
}

# --- taking one reading -------------------------------------------------

# Sets OWN_KIB, DESC_KIB, TOTAL_KIB, PROC_COUNT and, unless $1 is "quiet",
# prints the per-process table. Returns 1 when the application is not running.
OWN_KIB=0
DESC_KIB=0
TOTAL_KIB=0
PROC_COUNT=0

take_reading() {
  local quiet="${1:-}"
  local app_pid
  app_pid="$(find_app_pid)"
  if [[ -z "$app_pid" ]]; then
    echo "memory-report.sh: no '$APP_EXECUTABLE_NAME' process found; nothing to measure" >&2
    return 1
  fi

  read_process_table

  OWN_KIB=0; DESC_KIB=0; PROC_COUNT=0
  local skipped=0
  [[ "$quiet" == "quiet" ]] || printf '%-8s %-18s %12s %12s\n' PID COMMAND "PSS/$MEMORY_UNIT" "RSS/$MEMORY_UNIT"

  local proc figures p_pss p_rss
  while read -r proc; do
    figures="$(process_figures "$proc")"
    if [[ -z "$figures" ]]; then
      skipped=$((skipped + 1))
      [[ "$quiet" == "quiet" ]] || echo "  (pid $proc exited during the walk; skipped)" >&2
      continue
    fi
    read -r p_pss p_rss <<<"$figures"
    [[ "$quiet" == "quiet" ]] || printf '%-8s %-18s %12s %12s\n' "$proc" "${COMM[$proc]:-?}" "$p_pss" "$p_rss"

    if [[ "$proc" == "$app_pid" ]]; then
      OWN_KIB="$p_pss"
    else
      DESC_KIB=$((DESC_KIB + p_pss))
    fi
    PROC_COUNT=$((PROC_COUNT + 1))
  done < <(descend_from "$app_pid")

  TOTAL_KIB=$((OWN_KIB + DESC_KIB))

  if [[ "$quiet" != "quiet" ]]; then
    echo
    printf 'own          %12s %s\n' "$OWN_KIB" "$MEMORY_UNIT"
    printf 'descendants  %12s %s\n' "$DESC_KIB" "$MEMORY_UNIT"
    printf 'total        %12s %s\n' "$TOTAL_KIB" "$MEMORY_UNIT"
    printf 'processes    %12s\n' "$PROC_COUNT"
    [[ "$skipped" -eq 0 ]] || printf 'skipped      %12s (exited mid-walk)\n' "$skipped"
  fi
}

# --- soak mode ---------------------------------------------------------

run_soak() {
  if [[ ! -e "$soak_out" ]]; then
    printf 'timestamp\town_%s\tdescendants_%s\ttotal_%s\tprocess_count\n' \
      "$MEMORY_UNIT" "$MEMORY_UNIT" "$MEMORY_UNIT" >>"$soak_out"
  fi
  echo "memory-report.sh: soaking every ${soak_interval}s into $soak_out (Ctrl-C to stop)" >&2

  while true; do
    # Re-read the table fresh each sample: pids and the tree change across a
    # page load, a park and an unpark, which is exactly what a soak measures.
    CHILDREN=(); COMM=()
    if take_reading quiet; then
      printf '%s\t%s\t%s\t%s\t%s\n' \
        "$(date -Is)" "$OWN_KIB" "$DESC_KIB" "$TOTAL_KIB" "$PROC_COUNT" >>"$soak_out"
    else
      printf '%s\t\t\t\t\n' "$(date -Is)" >>"$soak_out"
    fi
    sleep "$soak_interval"
  done
}

# --- entry point -----------------------------------------------------

if [[ -n "$soak_interval" ]]; then
  run_soak
else
  take_reading
fi
