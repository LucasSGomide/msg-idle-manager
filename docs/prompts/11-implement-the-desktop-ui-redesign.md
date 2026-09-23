# Goal: Build the whole Claude Design redesign into the GTK window — sidebar, header bar, menus and every dialog — and append the design rules it introduces

**Status:** not executed
**Rating:** —
**Run:** parallel with 12 — 12 writes only `docs/requirements.md`, this one writes code and `docs/design.md`. No shared files. Neither waits on the other.

## Context

Prompt `09` briefed Claude Design to redesign Idle Manager's main window and
dialogs. The design came back. This prompt builds it.

**Where the design lives.** One artifact:
`https://claude.ai/artifact/JneJT2xQA5PnCsXGjTRDW4`. It is a *bundled* page —
reading it with the Artifact tool returns an unpacking loader, not the design.
The design is inside, in a script tag, gzipped and base64'd. Unpack it before
reading anything else:

```python
import re, json, base64, gzip
s = open("<the saved artifact html>", encoding="utf-8").read()
tpl = json.loads(re.search(r'<script type="__bundler/template">(.*?)</script>', s, re.S).group(1))
tpl = re.sub(r'<style.*?</style>|<script.*?</script>', '', tpl, flags=re.S)
open("brief.html", "w").write(tpl)
```

`brief.html` is the full brief: seven sections, ASCII wireframes for every
screen, an icon table, a keyboard table, the design rules it adds, and a
complete English/pt-BR string table. Read all of it before writing a line. It
is the specification; this prompt is only the terms it lands under.

**What changes, in one breath.** `Add account` leaves the header bar and
becomes the first control in the sidebar. The sidebar toggle and reload move to
the header bar's start edge, above the sidebar they act on. A sidebar row shows
a state mark and a name and nothing else — every action moves into a `⋯` menu
that appears on hover or selection, on right-click, and on `Shift`+`F10`. The
six states get their own *shapes*, not just their own colours. Selection mode is
deleted and a `Move to ▸` submenu replaces it. The header title names the
focused account and its workspace, reload's tooltip names the game it reloads,
and the focused slot's outline goes to 3 px. The `1 2 4 Phone` toggles become
pictures of the arrangements. Zoom becomes readable at rest, in the main menu.

**What this is not.** It is not a roadmap item and gets no `docs/roadmap/` doc
and no `docs/tasks/` breakdown. It is a redesign of screens that already exist,
against requirements already recorded. It is also **not** the translation work:
the brief's pt-BR column is there so the layout is built to survive a 35%-longer
string, not so the application ships two languages. Translation is prompt `12`
and a roadmap item of its own.

## Constraints

1. **Build every screen the brief covers, in one branch.** Sections 2.1 through
   2.13: main window, header bar, sidebar rows and headings and the add button,
   the memory footer, the account and workspace menus, the main menu, all six
   slot states, add-account, rename, delete, the phone dialog, the shortcuts
   window, and the empty and warning states. They interlock — the `⋯`, the focus
   marks and the string table cut across all of them — so a partial landing
   leaves the window in two design languages at once.

2. **Create the branch before the first code edit.** GitButler is set up:
   `but branch new feat/ui-redesign`. There is no roadmap number to put in the
   name because there is no roadmap item, so the retire hook will not fire and
   nothing needs stamping afterwards. The ship gate judges task files; this
   branch adds none, so it passes with a note on stderr.

3. **Amend design rule 1 in place and append the new rules as 21–28.** The brief
   numbers its new rules 20–27, written before roadmap item 15 landed rule 20.
   `docs/design.md` says renumbering breaks citations, so the brief's 20 becomes
   the repo's 21 and each one after shifts by one:

   | Brief | Repo | Rule |
   | --- | --- | --- |
   | 20 | **21** | Hover-and-selection-only `⋯`, plus right-click and `Shift`+`F10` |
   | 21 | **22** | `Add account` at the top of the sidebar, never in the header bar |
   | 22 | **23** | `Move to ▸` submenu instead of a selection mode |
   | 23 | **24** | An arrangement toggle is a picture of the arrangement |
   | 24 | **25** | Controls that act on the sidebar sit at the header bar's start edge |
   | 25 | **26** | Name the focused account in the title and in every targeting tooltip; 3 px marks |
   | 26 | **27** | Show an accelerator only where that key would do the same thing |
   | 27 | **28** | No text-bearing widget gets a fixed width |

   Rule 1 is amended where it stands — shape *and* colour, not colour alone —
   keeping its number and its reasoning. Write each new rule in the file's own
   voice: one imperative, then the why, in full sentences, not a copy of the
   brief's shorthand.

4. **Deleting selection mode is part of the work, not a leftover.** `FR.23.4`
   and `FR.26.6` both say the window's chords are consumed and inert while the
   sidebar is selecting. With the mode gone those branches are dead code — remove
   them, do not leave them unreachable. `FR.27.3` also names selection mode as
   one of the three things that hold the keyboard back from a game view; that
   clause loses a term and keeps the other two. Say plainly in the final report
   which recorded requirements the redesign has made obsolete, so they can be
   superseded in `docs/requirements.md` later rather than quietly contradicted.

5. **Prove the brief's eleven open questions on Linux before building on them.**
   Section 7 lists what the designer could not verify. Six are testable here —
   the hover-only `⋯` inside a `GtkListView` row without the row re-laying out,
   right-click and `Shift`+`F10` opening a popover at the row, per-item `accel`
   attributes on the focused account's menu model only, `GtkPopoverMenu` section
   labels that update when the model changes, `Move to ▸` as a `GMenu` submenu
   with the current workspace insensitive, and the arrangement pictures staying
   crisp at 16 × 12 from nested `GtkBox`es with `currentColor` borders. Build the
   smallest thing that answers each one, then build on the answer. If a
   mechanism does not work, say so and design around it rather than shipping
   something that only looks right in the wireframe.

6. **Record the five Windows-only questions as unverified risk; do not claim
   them.** The icon names (`view-pin-symbolic`, `document-open-recent-symbolic`,
   `list-drag-handle-symbolic`, `phone-symbolic`), `@theme_selected_bg_color` in
   the shipped Windows theme, the 3 px outline staying visible beside a
   `WebView2` child, the zoom popover not stealing focus or flickering, and
   `GtkShortcutsWindow` column wrapping — none can be exercised on this machine.
   `FR.26.7` already sets the standard: where the Windows machine is not
   available, that is a blocker on record, never a claim that it works. List
   them in the final report.

7. **Nothing that verifies focus by window activation can be proved headless.**
   This box has Xvfb and no window manager, so anything gated on the window
   being the active toplevel — `FR.27.3`'s condition, the focus grab on the
   focused account's view — cannot be exercised. Verify what the harness can
   reach (widget tree, CSS classes, menu models, popover contents, tooltips,
   accessible labels, ellipsising at 200 px) and name what it cannot.

8. **Add no translation machinery.** No `gettext`, no `.po` files, no string
   catalogue, no language setting. English strings stay literals exactly as they
   are today. What the brief's pt-BR column buys is rule 28: give no
   text-bearing widget a fixed width, let labels ellipsise at the end, let
   dialogs grow to their content. Build to that and prompt `12`'s item will have
   nothing to undo.

9. **Keep every rule the brief says it keeps.** Rules 2 through 19 are unchanged
   and still bind — in particular rule 6's sidebar width check. The brief drops
   the state word from the row and adds nothing to the trailing edge, so run the
   check the rule demands: settle the width by eye with a short indented name on
   screen, and say in the report whether 200 px still earns its place.

10. **`make verify` passes before the branch is offered.** Formatting, clippy,
    tests, the dependency audit, the layer boundaries and the roadmap check. The
    layer rule is not negotiable: `idle-manager-core` stays pure domain and never
    reaches GTK. New UI files are kebab-case under
    `crates/idle-manager-shell/resources/ui/`; new Rust module files are
    snake_case.

11. **Automated tests back the behaviour, not the looks.** A state's mark, the
    menu model a row builds, which items carry an accelerator, what the title
    label reads for a given focused account, what `Move to ▸` offers and what it
    greys — all of that is testable and should be tested. Pixel appearance is
    not; that is what the runbook in constraint 12 is for.

12. **Write a hand-run runbook and actually run it.** There is no
    `docs/tasks/<item>/` to put a `test-script.md` in, so it goes at
    `docs/ui-redesign-runbook.md`: a `## Setup`, one section per screen, a
    `## Teardown`. Every line is a checkbox holding one concrete action and the
    observable result it must produce — a click path and what appears, a key and
    what moves, a string and where it shows. "Check the sidebar looks right" is
    not a step. Tick a box only after the step has been run.

## Output

A branch carrying the redesign, plus:

- amended and appended rules in `docs/design.md`, in that file's voice;
- `docs/ui-redesign-runbook.md`, written and ticked;
- a final report naming, in three lists: what the six Linux questions answered,
  what the five Windows questions remain unverified on, and which recorded
  requirements the removal of selection mode has made obsolete.
