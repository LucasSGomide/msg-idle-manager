# Goal: Run the pre-roadmap process for Brazilian Portuguese — record the user needs and functional requirements for a translation mechanism whose first language is pt-BR

**Status:** not executed
**Rating:** —
**Run:** parallel with 11 — this prompt writes only `docs/requirements.md`; 11 writes code and `docs/design.md`. No shared files. Neither waits on the other.

## Context

Idle Manager speaks English and nothing else. Every string in the application
is an English literal in Rust source or in a `.ui` file, and there is no
`gettext`, no catalogue, no `po/` folder and no language setting anywhere in the
tree. The owner is Brazilian and wants to use the application in Brazilian
Portuguese.

Run `/msg-pre-roadmap` on that idea and land its output in
`docs/requirements.md`. **Requirements only.** Do not open a roadmap item, do
not write a breakdown, do not touch code. `/msg-roadmap-plan-item` comes after
this and will refuse an idea with nothing recorded behind it — recording it is
the whole job here.

**What already exists to build on.** Prompt `09`'s design brief — the artifact
at `https://claude.ai/artifact/JneJT2xQA5PnCsXGjTRDW4`, unpacked with the recipe
in prompt `11` — carries a complete English/pt-BR string table for the whole
desktop window, section 6. It is a designer's translation of every string the
redesigned window shows, and it is the closest thing to a first catalogue this
project has. Read it: it tells you how many strings there are, how much longer
pt-BR runs, and which ones carry placeholders. Design rule 28, which prompt `11`
appends, already forbids fixed widths for exactly this reason.

**Four decisions are already made** and go in as given, not as open questions:

- The requirements cover **how the application carries languages at all** — how
  a string is marked for translation, where translations live, what happens to a
  string with no translation — with **pt-BR as the first and only shipped
  language**. Adding a third language later should cost a file, not a rewrite.
- **The phone page is in scope.** `idle-manager-remote` serves HTML to the
  phone, and a Brazilian owner reads it on the same phone. Its handful of
  strings — the liveness words, `Park`, `Start`, `Reconnecting…`, the
  un-enrolled sentence — follow whatever the desktop is set to. Design rule 17
  already binds that page to this file's rules; language is one more.
- **The owner picks the language in the application, and English is the
  default.** The machine's own locale is not consulted. A fresh install is
  English; pt-BR is a choice the owner makes and the application remembers.
- The redesign of prompt `11` is assumed landed or landing. Write the
  requirements against the redesigned window, not today's.

**What the grill still has to close.** Take these as the branches to walk, not
as an exhaustive list:

- Where the language choice lives and how it is reached — a main-menu item, a
  preferences window this application does not yet have, something else — and
  which file remembers it. The application already writes `sessions.toml` and a
  per-account `state.toml`; a third file needs a reason.
- Whether changing the language takes effect at once or at the next launch, and
  what happens to a window full of live games either way.
- What is **not** translated: account names, workspace names and game names are
  the owner's own words and stay as typed. Say so explicitly, or someone will
  translate `Ungrouped` and wonder why a renamed workspace did not follow.
- How numbers and units read — the memory footer's `1 204 MiB`, the pager's
  `1/2`, the phone viewport's `412 × 915`. pt-BR groups digits differently from
  English, and rule 11 fixed the format in English.
- What a missing translation does: fall back to English silently, or be caught
  before shipping.
- How a translation is proved. This box has no window manager and headless
  verification is limited; a language that can only be checked by changing the
  machine's locale cannot be tested here at all, which is one more argument for
  the in-app picker and worth recording as a requirement rather than an
  afterthought.
- Whether the shortcut chords change. They are letters — `Ctrl`+`P` for park,
  `Ctrl`+`S` for start — that mean something in English and something else in
  Portuguese. Decide whether the keys stay put and only their descriptions move.
- Whether the application's own name, window title and any file path it writes
  stay English.

## Constraints

1. **Follow `/msg-pre-roadmap` as it is written.** Brainstorm, close the gaps,
   research if the skill calls for it, then write requirements. Do not shortcut
   to writing table rows.
2. **`docs/requirements.md` is append-only.** Never edit or delete an existing
   row. Where a new requirement changes an old one, add a row that says it
   supersedes or refines it by code, the way `FR.3.5` supersedes `FR.3.4` and
   `FR.22.2` refines `FR.3.2`.
3. **Pick the Module and Feature names deliberately.** The file's existing
   modules are `Session Management` and `Remote Access`; a translation that
   covers both the GTK window and the phone page may not sit under either. Say
   why the name you choose is the right one.
4. **No roadmap doc, no task breakdown, no code.** `docs/requirements.md` is the
   only file this prompt writes.
5. **Run `make roadmap-check` before finishing**, and `make roadmap-sync` if the
   check asks for it.
6. **Dates are absolute.** Use the real date in the `Addition Date` column, not
   "today".

## Tone

Match `docs/requirements.md` as it already reads: full sentences, plain words, a
requirement stated as a fact about the finished application rather than as a
task. Say what must be true and why the reader should believe it matters. No
jargon the file has not already used.

## Output

New rows appended to `docs/requirements.md` — one `UN.n` per user need, its
`FR.n.m` rows beneath it — and a short note back saying which needs were
recorded, which of the open branches above the grill closed, and what it left
for `/msg-roadmap-plan-item` to settle.
