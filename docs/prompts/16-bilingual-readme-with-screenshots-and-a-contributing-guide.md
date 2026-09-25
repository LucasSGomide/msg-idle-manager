# Goal: Reorganise the project documentation — an English README with a Portuguese twin, screenshots, and a bilingual contributing guide that teaches the msg workflow well enough for an outsider to build a feature with it

**Status:** executed on 2026-09-24 — the contributing guides and the
`.claude/` publication were dropped on 2026-09-25 before anything was pushed;
only the READMEs and the screenshots stayed, and a licence was added
**Rating:** 7

## Context

The repository is now public at `github.com/LucasSGomide/msg-idle-manager`
and the first release (v0.1.0) is out. `README.md` today is 43 lines: a
one-paragraph description, then a **Download** section (the release table,
SmartScreen and AppImage notes, in-app updating). It has no picture, no feature
tour, no pointer to the docs tree, and nothing about contributing. `CLAUDE.md`
opens by saying the `.claude/` folder is "private to the maintainer and not
part of this repository", which is true today and stops being true with this
prompt.

**Research first, then write.** Before touching a file, look at how a handful
of well-regarded GitHub projects do the three things this prompt needs, and
let that shape the result:

- a README kept in two languages — where the language switch sits (the
  usual pattern is a one-line `English | Português (Brasil)` row under the
  title), whether the translated file is `README.pt-BR.md` at the root or
  under a folder, and how they keep the two in step;
- screenshots in a README — one hero image versus a short gallery, captions,
  how wide the images are shown, where the files live (`docs/screenshots/`,
  `.github/`, `assets/`);
- a `CONTRIBUTING.md` that walks a newcomer through a workflow step by step,
  especially projects whose contribution process is driven by an AI coding
  agent with repository-local skills, hooks or slash commands.

Record what you took from each project in a short **Research** note inside the
PR description or commit body — three to six lines, project name and the
pattern borrowed. Borrow shapes, never text.

**What the README has to become.** The English `README.md` is the front door:

1. Title, the language-switch row, one paragraph saying what Idle Manager is
   and who it is for (someone who keeps several browser idle games running at
   once and is tired of losing them in tabs).
2. The screenshots (below), each with a one-line caption.
3. A feature tour, one short paragraph or bullet per feature, drawn from the
   **done** roadmap items in `docs/roadmap/README.md` — isolated accounts in
   one splittable window (item 01), the session sidebar (02), parking and
   unparking (03), keep-awake for hidden games (04), memory accounting (05),
   presets (06), workspace restore (07), operating an account from a phone
   (13), releases and in-app update (16). Read each item's `## As built` or
   summary; say what the user gets, not how it is built.
4. The existing **Download** section, kept as is (it was written for v0.1.0
   and is accurate); tighten only if the research shows a clearly better
   shape.
5. **Contributing** — two or three sentences and a link to `CONTRIBUTING.md`.
6. A **Documentation** map: one line each for `docs/stack.md`,
   `docs/architecture.md`, `docs/code-standards.md`, `docs/naming.md`,
   `docs/design.md`, `docs/requirements.md`, `docs/roadmap/README.md`,
   `CHANGELOG.md`, `release/README.md`.
7. Licence line if the repo has one; if it has none, say so in the report and
   do not invent one.

`README.pt-BR.md` is the same document in Brazilian Portuguese, section for
section, with the switch row pointing back. Same for `CONTRIBUTING.md` and
`CONTRIBUTING.pt-BR.md`.

**The screenshots** are already taken. The owner dropped three PNGs
(1913 × 1163, 450 KB to 1.3 MB) at the repository root, untracked:

| File today | Shows |
| --- | --- |
| `msg-idle-manager-4-screens-splt.png` | the main window: sidebar with three workspaces, two games running side by side, a parked account with its **Start** button, the memory footer |
| `msg-idle-manager-shortcuts.png` | the **Shortcuts** dialog over the same window |
| `msg-idle-manager-phone-connection.png` | the **Phone** dialog with its QR code, over a single game in the phone layout, page 1/3 |

Move them to `docs/screenshots/` under kebab-case names that say what they
show (`docs/naming.md`), shrink them without visible loss (`oxipng` or
`pngquant`; a README image over ~400 KB is a slow page for no gain), and
reference them from both READMEs with a caption each.

**The contributing guide** is the bold part. A contributor who clones the
repo, has Claude Code installed, and has never seen the msg workflow must be
able to read `CONTRIBUTING.md` and take a feature from idea to merged pull
request. It teaches, in order:

1. **Setting up** — `make bootstrap`, `make dev`, `make verify`, and what
   `make` alone lists. Where the five crates are and the one dependency rule
   (`docs/architecture.md`), in a sentence each.
2. **How the planning docs work** — `project.yml` as the manifest; the four
   folders (`roadmap/`, `tasks/`, `explorations/`, `ditched/`) and what lives
   in each; that numbers are permanent IDs; that requirements come first in
   `docs/requirements.md`; that only `make roadmap-sync` writes tables and
   people write prose and checkboxes.
3. **The skills, in the order a feature goes through them** — each with what
   it produces and when to run it: `/msg-pre-roadmap` (needs and requirements),
   `/msg-roadmap-plan-item` (the item folder), `/msg-wireframes`,
   `/msg-sequence-diagrams` and `/msg-api-contracts` (only when the item has a
   screen, a new route, an endpoint), `/msg-roadmap-task-breakdown` and
   `/msg-roadmap-task-review` (the slices), `/msg-commit` (semantic commits),
   `/msg-roadmap-sync` (statuses, retiring a landed breakdown). `/msg-grill-me`,
   `/msg-brainstorm`, `/msg-write-prompt` and `/msg-setup` get a line each as
   tools you reach for, not steps. Say plainly which are conversations the
   skill runs with you and which just write files.
4. **The rules the hooks enforce** — branch-first for code edits and the
   branch name carrying the item number; acceptance before landing (every box
   ticked, the task's section in `test-script.md`); the retire marker. Say what
   each hook blocks and what the message looks like, so a blocked contributor
   knows why. Say the hooks need `jq` and a POSIX shell.
5. **Without Claude Code** — one short section: the skills are Markdown
   instructions, so a contributor without the tool follows the same steps by
   hand, and `make roadmap-sync` / `make roadmap-check` work either way.
6. **Landing** — `make verify` green, a pull request against `main`, what the
   PR description should carry (the item number, the ticked slices).

A worked example runs through the whole guide: one small invented feature,
named at the top and followed through every step, so the reader sees the same
thing shaped as a requirement, an item, a breakdown, a branch name, a commit,
a PR. Keep it to a feature that fits in one or two slices.

**Publishing the skills and hooks.** This prompt commits `.claude/skills/`,
`.claude/hooks/` and `.claude/settings.json` so a contributor gets the same
slash commands and guards. `.gitignore` line 1 ignores all of `.claude`;
replace it with rules that ignore only `.claude/settings.local.json`,
`.claude/worktrees/` and `.claude/scheduled_tasks.lock`. Then fix `CLAUDE.md`'s
opening paragraph — it must say the skills and hooks ship with the repo and
point at `CONTRIBUTING.md` for the tour. Update the sentence in
`docs/prompts/README.md` if it still says the prompts are private or
untracked.

## Constraints

1. **Scrub the screenshots before they are committed.** The phone screenshot
   shows a live enrolment URL with a Tailscale address (`100.x.x.x`) and a
   token, and the QR code encodes the same URL. Pixelate or paint over both the
   URL text and the QR code, or ask the owner to retake it with
   `phone.toml` bound to `127.0.0.1` — never commit the address or the token.
   The sidebar labels name two people; show the owner the scrubbed images and
   let them decide whether those labels stay, before anything is committed.
   Prompt `15` set the rule: a real `100.64.0.0/10` address is personal data.
2. **Read every skill and hook before describing it.** The guide describes what
   `.claude/skills/*/SKILL.md` and `.claude/hooks/*.sh` actually do, not what
   the skill names suggest. Where a skill's behaviour and `CLAUDE.md` disagree,
   say so in the report rather than papering over it.
3. **Secrets check on what gets published.** Before committing `.claude/`,
   grep it for paths under `/home/`, mesh addresses, tokens and account names.
   `.claude/settings.local.json` stays ignored (it names other projects'
   skills) and so does `.claude/worktrees/`.
4. **The Portuguese is Brazilian Portuguese, written by hand, not machine
   output pasted in.** Use the pt-BR string table from prompt `09`'s design
   brief for UI terms so the README calls things what the app will call them
   (workspace, parked, keep-awake, preset). Where a term has no settled
   translation yet, keep the English word in `code` and say so once.
5. **No code changes.** This prompt touches Markdown, PNGs, `.gitignore` and
   the `.claude/` folder only. No `src/`, no `Makefile`, no `Cargo.toml`.
   The branch-first rule therefore does not apply; work on a `docs/readme-and-
   contributing` branch anyway so the change goes up as one pull request.
6. **Links must resolve.** Every relative link in the four Markdown files
   points at a file that exists in the tree at the commit; check them with a
   script, not by eye. Both READMEs and both contributing guides are
   section-for-section twins — same headings, same images, same links.
7. **`make verify` and `make roadmap-check` still pass** at the end. Moving
   files must not break a path named in `project.yml`.
8. **Commit through `/msg-commit`**, in this order: the `.gitignore` change and
   the `.claude/` files; the screenshots; the READMEs; the contributing
   guides; `CLAUDE.md`. Do not push.

## Tone

Direct, clear, avoiding jargon, explaining like a teacher addressing a
beginner. The README speaks to a player who has never built software; the
contributing guide speaks to a developer who has never used a planning
workflow driven by an AI agent and may be sceptical of it — show the value
(requirements traced, a reviewer can check a breakdown, the gate refuses a
half-accepted slice) without selling.

## Output

- `README.md` (English) and `README.pt-BR.md` (pt-BR), each with the
  language-switch row.
- `CONTRIBUTING.md` and `CONTRIBUTING.pt-BR.md`, twins.
- `docs/screenshots/` with the three scrubbed, shrunk PNGs.
- `.gitignore`, `CLAUDE.md` and `docs/prompts/README.md` updated;
  `.claude/skills/`, `.claude/hooks/` and `.claude/settings.json` tracked.
- A closing report: the research note, the owner decisions still open (the
  sidebar names, the licence), and the `make verify` result.
