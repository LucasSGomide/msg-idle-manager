# Goal: Stop a single zombie descendant from blanking the memory footer permanently

**Status:** executed 2026-09-19 — branch `fix/pss-zombie-descendant`, uncommitted, in the worktree `.claude/worktrees/fix-pss-zombie`
**Rating:** —
**Run:** parallel with 07 — 07 touches `docs/requirements.md` only, this
touches `crates/idle-manager-metrics/` only.

## Context

The application "stops being able to read memory usage" after running for a
while. The cause is known and was reproduced on `gomide` on 2026-09-19 against
a live zombie (`xdg-terminal-ex`, PID 306036):

| File | A zombie's behaviour |
| --- | --- |
| `/proc/<pid>/status` | Readable, carries `Name:` and `PPid:` → **`ProcessEntry::parse` succeeds, so the zombie enters the table and the walk** |
| `/proc/<pid>/smaps_rollup` | `ESRCH` — "No such process", *not* `ENOENT` |
| `/proc/<pid>/smaps` | Readable, **0 bytes** |

Trace it through `crates/idle-manager-metrics/src/proc_pss.rs::read_pss_kib`:

1. The rollup's `ESRCH` is swallowed by `if let Ok(text) = …` and correctly
   falls through to `smaps`. Fine so far.
2. `fs::read_to_string(&smaps_path)` on a 0-byte file returns **`Ok("")`** — a
   successful read, so neither the `NotFound` arm nor the `Refused` arm is
   reached.
3. `sum_pss_kib("")` returns `None`, because it treats an empty parse as a
   parse failure rather than a zero — deliberate, and right for its own case.
4. `.ok_or(ProcPssError::Malformed { … reason: "no Pss: line" })` therefore
   fires.
5. `read_tree` calls `self.read_pss_kib(pid)?` and the `?` propagates, so
   **the entire reading fails** — `MemoryProbeError::Unreadable`, and the shell
   shows its unavailable footer.

It stays broken until the kernel reaps the zombie. WebKitGTK starts each
renderer inside nested `bwrap` sandboxes with `xdg-dbus-proxy` helpers, which
is exactly the kind of grandchild that lingers unreaped in a GTK application
that installs no `SIGCHLD` handler.

The code already anticipates this scenario — `read_pss_kib`'s doc comment says
*"`Ok(None)` means the process vanished between the walk and this read"* — but
it only handles the **fully reaped** form, where both files are `ENOENT`. The
**zombie** form, dead but still in the process table, takes the `Malformed`
path instead.

One consequence worth stating: a silent instrument is not evidence of a healthy
system. Item 05's leak finding (`docs/memory-budget.md`: descendants growing
~7–11 GiB/hour, not settling) and the more recent impression that memory is
fine cannot both be judged while the probe fails this way. Fixing the probe is
the cheapest route to settling item 05's still-open 4-hour soak criterion.

## Constraints

1. **Two fixes, both wanted.** The surgical one: an *empty* `smaps` /
   `smaps_rollup` means "vanished", so return `Ok(None)` and skip, leaving
   `Malformed` to mean what its doc comment claims — non-empty content with no
   `Pss:` line. The structural one: `read_tree` is all-or-nothing across a
   process tree whose membership churns constantly, so a per-process read
   failure should log and skip, and only a failure to read `own` should fail
   the whole sample.
2. **Keep the two error cases distinct.** `ProcPssError::Refused` and
   `::Malformed` exist because the shell shows one footer for both while the
   log tells them apart (architecture rule 11, code standards rule 15). The fix
   must not collapse them, and must not turn a genuinely malformed file into a
   silent skip.
3. **Log every skip at debug with a reason**, matching the existing
   `tracing::debug!(pid, reason = "exited during the walk", "process skipped")`
   so the log still explains a total that looks too small.
4. **Check the base branch first.** `crates/idle-manager-metrics/src/tree.rs`
   and `src/process_tree.rs` are currently **untracked**, added by the in-flight
   item 12 task 07 work on `feat/12-windows-delete-and-package`, and
   `proc_pss.rs` already imports `crate::tree`. Decide deliberately whether to
   branch from `main` (where the walk may still be inline) or from the item 12
   branch, and say which — do not discover the conflict halfway.
5. **Look at the Windows probe too, but do not fix it blind.**
   `process_tree.rs` is the `CreateToolhelp32Snapshot` walk and is mid-
   implementation. Check whether it carries the same all-or-nothing shape, and
   report it; only change it if it is settled enough to change safely.
6. **Test against a captured `/proc` fixture, not a real zombie.**
   `crates/idle-manager-metrics/tests/walking-the-process-tree.rs` already
   drives `ProcPssProbe::under`. Add at minimum: a zombie case (a fixture pid
   with a valid `status` holding `Name:`/`PPid:`, an empty `smaps`, and no
   `smaps_rollup`) asserting the sample **succeeds** and the zombie is excluded
   from `process_count`; and a regression case (a non-empty `smaps` with no
   `Pss:` line) asserting `Malformed` still fires, so the surgical fix does not
   erase the distinction constraint 2 protects.
7. **Stay inside the adapter.** This is `idle-manager-metrics` only.
   `MemoryProbeError` in the core almost certainly needs no change; if you
   believe it does, say why before changing it (architecture rules 1, 5, 6).
8. **Branch before the first edit.** This touches `crates/`, so the CLAUDE.md
   branch-first rule and its `PreToolUse` hook apply. It is a bug fix, not a
   roadmap item, so there is no item number for the branch name — something
   like `fix/pss-zombie-descendant`. The retire hook will not fire and does not
   need to.
9. **No requirements entry and no roadmap item.** This restores behaviour
   `FR.19.1` already specifies rather than adding any, so the planning gate
   does not apply. Item 05's open 4-hour soak criterion is *unblocked* by this
   work — mention that, do not tick it.
10. **`make verify` before handing back**, and say plainly what it reported.

## Output

Rust in `crates/idle-manager-metrics/` — `src/proc_pss.rs` primarily, plus the
new cases in `tests/walking-the-process-tree.rs`. A note on whether
`src/process_tree.rs` shares the defect.
