# Goal: Fix retire-breakdown-post.sh so it recognises the folder-per-item roadmap layout

**Status:** not executed
**Rating:** —

## Context

`retire-breakdown-post.sh` is the PostToolUse hook that stamps `**Landed:**` /
`**Merged:**` onto a roadmap item's doc after this session runs `but land <ref>`
or `git merge <ref>` with the item number in the branch name. That marker is the
only trigger that retires the item's task breakdown.

The hook silently no-ops for any project that lays each roadmap item out as a
folder — `docs/roadmap/NN-slug/README.md` — instead of a flat `docs/roadmap/NN-slug.md`
file. In `try_stamp()` the doc is located by globbing only flat files:

```bash
doc=""
for cand in "$root/$roadmap_rel/$pad-"*.md "$root/$roadmap_rel/$num_nopad-"*.md; do
  [[ -f "$cand" ]] && { doc="$cand"; break; }
done
[[ -n "$doc" ]] || return 0
slug=$(basename -- "$doc" .md)
```

With a folder layout none of those globs match, `doc` stays empty, `try_stamp`
returns 0, and the hook does nothing — no output, no error. The rest of the
toolchain (`scripts/roadmap-sync.mjs`, `make roadmap-check`) already handles the
folder layout correctly; only this hook is out of step, so a folder-layout
project has to hand-stamp every marker and never knows the hook was meant to do
it.

Observed 2026-09-06 in a folder-layout project: `but land feat/01-web-view-reload`
ran to completion, no marker appeared on `docs/roadmap/01-*/README.md`; a manual
`bash -x` trace of the hook showed `doc=` empty immediately followed by
`return 0`.

## Constraints

1. One script, both layouts. Detect and support flat `NN-slug.md` **and** folder
   `NN-slug/README.md`. Existing flat-layout projects must behave exactly as they
   do today — same doc picked, same stamp, same stderr.
2. When the doc is a folder `README.md`, `slug` is the folder name, never
   `README`. `slug` feeds the `[[ -d "$root/$tasks_rel/$slug" ]]` guard and the
   stderr hint, so a wrong slug means the hook skips a real retirement or points
   at the wrong path.
3. Keep the hook's timidity. It must still require `**Status:** done`, no
   existing `Landed:` / `Merged:` field, the `docs/tasks/<slug>/` folder present,
   and the shipped ref an ancestor of the target branch (or the ref gone, which
   is what a successful `but land` leaves). Add no new path to a false stamp — a
   wrong stamp tells the next sync to delete a folder that should still be there.
4. No new dependencies: bash + jq + git only. `set -euo pipefail` stays.
   Unmatched globs are already guarded by `[[ -f "$cand" ]]` / add the same guard
   for the new candidates.
5. Deterministic candidate order — zero-padded before unpadded, and within each,
   flat file before folder README. First hit wins, as now.
6. Fix the sibling hooks and scripts in the same pass if they carry the same
   assumption. Grep `acceptance-criteria-gate.sh`, `branch-guard-*.sh`, the
   Makefile targets and `roadmap-sync.mjs` for a hard-coded `docs/roadmap/*.md`
   or `NN-*.md` shape; fix any that would break on a folder layout, or note in
   the PR that they were checked and are fine.
7. If the repo has a hook test suite or fixtures, add a folder-layout retirement
   case (a `done` item with a folder doc, a shipped ref, a present task folder →
   marker stamped). If there is no such suite, say so in the PR rather than
   inventing a framework.

## Output

A minimal patch to `retire-breakdown-post.sh` where it is vendored in this repo
(the `msg init` hook template — likely under `templates/` or `hooks/`), plus any
sibling-hook fixes from constraint 6 and test fixtures from constraint 7.
Comment new lines in the script's existing voice. Then a short PR description:
the bug, the two-layout fix, what else was checked.

## Examples

The lookup block after the fix should try, in order:

```
$roadmap_rel/$pad-*.md
$roadmap_rel/$pad-*/README.md
$roadmap_rel/$num_nopad-*.md
$roadmap_rel/$num_nopad-*/README.md
```

and set `slug` to `basename "$doc" .md` for a flat hit, or
`basename "$(dirname "$doc")"` when `$doc` ends in `/README.md`.
