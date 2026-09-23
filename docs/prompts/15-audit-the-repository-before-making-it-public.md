# Goal: Audit the repository and its whole git history for sensitive information and for files that should not be published, then clean both before the first push to a public GitHub repository

**Status:** executed 2026-09-23 — history rewritten on `main`; findings in the ignored `docs/public-release-audit.md`, backup in `~/idle-manager-backup-2026-09-23/`
**Rating:** —

## Context

The repository has only ever lived on this machine. There is no git remote, and
prompt `14` settles that releases will come from a **public** GitHub repository.
Pushing publishes every tracked file in every commit on every branch pushed,
not just today's tree. So before the first push, find what must not go out and
remove it, and make sure the files that stay only on this machine are ignored
so they cannot leak back in later.

What a first look already found. Confirm each one, then keep looking; this list
is not complete:

- **`docs/handover.md` is tracked despite being ignored.** `.gitignore` says it
  is "never committed" because "it names live profile paths", yet
  `git ls-files` lists it, and it appears in four commits (`1dc315e`,
  `e8fb924`, `ecba032`, `bc41efd`). A file that was committed before it was
  ignored stays tracked, so the ignore rule never took effect.
- **Every commit carries the author's personal email.** All 287 commits are
  authored with the owner's personal email address. A public history shows
  that address to anyone. GitHub offers a `noreply` address for this.
- **Refs besides `main`.** A `feat/ui-redesign` branch, a
  `backup/pre-trailer-strip` tag and a stash quarantining changes from a broken
  subagent run. Decide which refs are pushed; by default only `main` is.
- **`scripts/windows-vm/compose.yml`** sets the VM's login to `Docker`/`admin`.
  Its ports are bound to loopback, so this is probably fine, but record it and
  say why.
- **`CLAUDE.md` describes hooks and skills under `.claude/`**, and all of
  `.claude/` is ignored and stays private. Once public, `CLAUDE.md` points
  readers at files they will never have.

**What to look for, beyond that list:**

- Credentials of any kind: tokens, keys, passwords, the phone pairing secret
  (roadmap item 13) or a real enrolment record, cookies or saved sessions from a
  game account.
- Personal data: real account names, game logins, home-directory paths
  (`/home/lucas-gomide/…`), real mesh addresses in `100.64.0.0/10`, the owner's
  network or devices. Test fixtures like `192.168.1.20` are not findings.
- Files that belong only on this machine: handovers, measurement dumps,
  screenshots, recordings, logs, anything generated into `dist/` or `target/`,
  editor and OS files. Check `docs/research/`, `docs/prompts/`,
  `docs/memory-budget.md`, `docs/ui-redesign-runbook.md` and the
  `test-script.md` files, which record hand-run sessions on this machine and
  may hold paths or output from real accounts.
- Large binary blobs in history (`.git` is 20 MiB for 287 commits).
- Anything deleted from the tree but still in history: search every commit,
  not only `HEAD`. Use a real secret scanner (`gitleaks` or `trufflehog`) across
  the full history, plus targeted `git log -p -S` / `git grep $(git rev-list
  --all)` searches for the patterns above.

For each thing found, decide one of three fates:

1. **Delete from the tree and scrub from history.** It must never be public.
2. **Keep on this machine, ignored.** Untrack it (`git rm --cached`), add a
   `.gitignore` rule, and scrub it from history if it was ever committed.
3. **Publish as is.** Say why it is safe.

## Constraints

1. **Two phases, with a stop between them.** First write the findings report and
   stop. Apply nothing until the owner approves the list, item by item. The
   owner may change a proposed fate.
2. **Scrub history by rewriting it, keeping the commits.** Use
   `git filter-repo` to strip the approved files and strings from every commit,
   so the 287 commits survive without the sensitive parts. Do not squash into
   one fresh commit. Whether to also rewrite the author email to a GitHub
   `noreply` address is the owner's call: propose it in the report, do not
   assume it.
3. **Back up before any rewrite.** Before running `git filter-repo`, make a full
   copy of the repository that a rewrite cannot touch (`git bundle create …
   --all` into a folder outside the repo, or a mirror clone). Say where it is.
   `git filter-repo` refuses a non-fresh clone; if it asks for `--force`,
   stop and say so rather than forcing it without the backup in place.
4. **`.claude/` stays private.** Nothing under `.claude/` is published. Keep
   the rule in `.gitignore`. For `CLAUDE.md`'s references to it, propose a fix
   in the report — do not edit `CLAUDE.md` until the owner approves.
5. **Do not push, and do not create the remote.** This prompt stops with a
   clean local repository ready to push. Creating the GitHub repository and the
   first push are the owner's acts. End by stating the exact command that
   would push only the approved refs.
6. **The report must not become a leak itself.** It quotes secrets and personal
   data by location and kind (`file:line`, commit, "an email address"), never
   by value. It lives in an ignored path, never in a commit.
7. **Prove the clean state.** After the fixes, rerun the secret scanner and the
   targeted searches over the full rewritten history and show they come back
   empty. Run `git ls-files` and confirm every tracked file is one the report
   marked "publish". Confirm the ignored-but-kept files still exist on disk.
   Run `make verify` to show the rewrite broke nothing.
8. **Stay on `main`, and commit through `/msg-commit`.** This is a repository
   hygiene change, not an implementation, so no roadmap item or branch naming
   applies. Commit the `.gitignore` and untracking changes before the history
   rewrite, so the rewrite includes them.

## Tone

Direct, clear, avoiding jargon, explaining like a teacher addressing a
beginner. The owner has not rewritten git history before: say in a sentence
what `git filter-repo` does, why pushed history cannot be taken back, and what
the backup is for.

## Output

Phase one: the findings report at a git-ignored path (for example
`docs/public-release-audit.md`, with its ignore rule added first). One table
row per finding: what it is, where (file, line, commits), how sensitive, the
proposed fate, and why. Then the list of decisions the owner has to make — the
author email, which refs to push, the `CLAUDE.md` wording.

Phase two, after approval: the fixes applied, the backup's location, the
before-and-after of the scanner and searches, the final `git ls-files` check,
and the push command for the owner to run.
