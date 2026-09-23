The hooks, settings and `/msg-*` skills named below live in a `.claude/` folder
that is private to the maintainer and not part of this repository. Without them,
the rules below are conventions to follow by hand; `make roadmap-sync` and
`make roadmap-check` work either way.

<!-- msg-roadmap:start -->

## Planning workflow

`project.yml` at the repo root is the manifest: it names the planning folders and
maps each area to the doc holding that area's rules. Read it before planning work.

| Folder | What lives there |
| --- | --- |
| `docs/roadmap/` | Committed work, one numbered doc per item |
| `docs/tasks/` | An open item's breakdown, one file per shippable slice |
| `docs/explorations/` | Ideas being researched, each ending in `## Findings` |
| `docs/ditched/` | Rejected ideas, kept so they are not re-proposed |

Numbers are permanent IDs. Nothing is ever renumbered or reused.

**Requirements come first**

`docs/requirements.md` is the append-only log of user needs and functional
requirements, and a roadmap item has to trace back to one. Run
`/msg-pre-roadmap` to record them there **before** `/msg-roadmap-plan-item`
opens the item — plan-item stops on an idea with nothing recorded behind it.

**Areas**

- **Design** — `docs/design.md`
- **Naming** — `docs/naming.md`

**Commands**

- `make roadmap-sync` — recompute every derived status and table from the docs
- `make roadmap-check` — fail on a stale table, a bad dependency, or a missing
  path named in `project.yml`

Run the sync after ticking an acceptance criterion, changing a status, or adding
a doc. Only the engine writes tables; prose and checkboxes are written by hand.

**Branch-first for implementation work**

Create a dedicated session branch — GitButler (`but branch new` / `but commit -b`)
if it's set up in the repo, else plain git (`git checkout -b`) — **before the
first code edit** of an implementation. Code means application source, libraries,
tests, and build/manifest/config files: `src/`, `lib/`, `app/`, `test/`,
`tests/`, `*.config.*`, `package.json` and lockfiles, `Makefile`.

Name an implementation branch with the roadmap item number — `feat/04-profiles`,
`fix/12-…`. The retire hook (below) keys off that number to know which breakdown
landed; a branch without it just means you add the retire marker by hand.

Neither tool is mandatory; either satisfies the rule. A `PreToolUse` hook (see
`.claude/settings.json`) enforces it — a Write/Edit touching code without a
session branch already created is blocked. Documentation and planning edits
(prompt files, roadmap docs, task breakdowns) don't need a branch first; create
one only when you specifically want to.

**Acceptance before landing**

Accepting a task is two acts, done together as the last step of implementing it:

1. **Tick every box** beneath the task's `## Acceptance criteria` heading, once a
   passing automated test backs each one. The engine derives item status from
   those boxes.
2. **Write the task's own section into `docs/tasks/<item>/test-script.md`** — a
   hand-run runbook, one file per roadmap item beside `README.md` and
   `openapi.json`, that proves the feature works end to end. It never replaces
   the `(unit)` / `(integration)` / `(e2e)` criteria; it sits beside them.

`test-script.md` shape: a `## Setup` and a `## Teardown` section written by the
first task to reach acceptance, then one `## MM — Task title` section appended by
each later task. Every line is a checkbox holding one concrete action and the
observable result it must produce — a command and its output, data to seed, a
request with its status and body, or a click path and what appears. "Verify the
endpoint works" is not a step. A box is ticked only after the step has actually
been run; reuse a Setup step already written rather than restating it.

Ticked checkboxes are sacred, in the criteria and in `test-script.md` alike.
Append your own section and, if you need one, a shared `## Setup` step; never
rewrite, reorder, untick, or delete another task's section.

A `PreToolUse` hook gates the ship moment — `but land`, a `git merge`, or a
`git push` that names the target branch; routine `but commit` / `git commit`
never ship. It judges **only the diff the ship carries** (the shipped ref against
the target), never a repository-wide walk, and reads each task file as it will
exist on the target *after* the ship. What it blocks:

- a numbered task file this ship **modifies and ticks at least one box in**,
  where a box beneath `## Acceptance criteria` is still unticked — a slice being
  accepted half way;
- a folder this ship accepts a slice into whose `test-script.md` is missing or
  has an unchecked step.

What it does **not** block, because it cannot tell an in-progress breakdown from
a forgotten tick: a ship that only **adds** task files (a fresh breakdown is
unticked by design), or that edits a task file without ticking any box (prose, a
new criterion). Those pass with a note on stderr. A slice legitimately landing
while later slices in its folder stay open is fine — untouched files are not in
scope.

Where the gate blocks, the honest way through is to tick the boxes for work that
is done and run `make roadmap-sync`, or move a not-yet-done criterion onto its
own slice — never tick a box for work that does not exist.

**Retiring the breakdown after it lands**

An item reaching `done` does **not** retire its `docs/tasks/<item>/` folder — the
breakdown stays put through review and merge, so a reviewer can check the
implementation against it. It is retired only after the branch actually ships,
on one explicit signal: `**Landed:** <date>` (GitButler `but land`) or
`**Merged:** <date>` (a plain `git merge` / PR) on the roadmap item's header.
The `retire-breakdown-post.sh` hook stamps that marker automatically when this
session lands or merges a branch whose name carries the item number; stamp it by
hand when the hook could not (a branch without the number, a land from a
terminal). The next `make roadmap-sync` then lists the folder for
`/msg-roadmap-sync` to retire — extract `## As built` onto the roadmap doc, then
delete the folder. Nothing else triggers it.

<!-- msg-roadmap:end -->

## Development

Four docs settle how the code is written. `project.yml` registers two of them as
planning areas, so a roadmap item cites them by number the way it cites Design
and Naming.

| Doc | What it settles |
| --- | --- |
| `docs/stack.md` | What we build with, at which version, and why |
| `docs/architecture.md` | The five crates, the dependency rule, where a change goes |
| `docs/code-standards.md` | How the code inside a crate is written |
| `docs/naming.md` | How files, crates and identifiers are named |

**Every developer command is a Makefile target** — `make` on its own lists them.
`make verify` is the whole gate: formatting, clippy, tests, dependency audit,
layer boundaries and the roadmap check. First time on a machine, `make bootstrap`
then `make dev`.

**The dependency direction is enforced, not reviewed.** `make arch-check` fails
the build when a crate reaches a layer it must not — `idle-manager-core` is pure
domain and may not touch GTK, WebKit, serde or the filesystem. Add a forbidden
edge to `scripts/arch-check.sh` whenever a new boundary is worth keeping.

**File names are kebab-case everywhere except Rust module files**, which must be
snake_case because the file name becomes a module identifier. `docs/naming.md`
rules 1–4 cover the boundary.
