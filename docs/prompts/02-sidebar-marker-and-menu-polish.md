# Goal: Strip the sidebar row down to a dot-only marker and fold Park/Start into the row menu

**Status:** not executed
**Rating:** —
**Run:** parallel with 03 — the only file both may touch is
`crates/idle-manager-shell/src/window/imp.rs`; landing order does not matter,
rebase is trivial.

## Context

Three cosmetic changes to the account sidebar (`idle-manager-shell`,
`src/session_sidebar/`). Each one reverses a design rule that a past roadmap
item wrote, so the doc changes are part of the work, not an afterthought:

1. **Drop the status word.** A row currently shows a word (`Current`,
   `Visible`, `Background`, `Parked`, `Starting`) next to the coloured status
   dot — `status_label(key)` bound onto a `gtk::Label` in the row factory. Remove
   the visible label. Keep the state named for a pointer and for a screen reader:
   set the state word as the dot's tooltip and as its accessible label, refreshed
   on the same bind that sets the dot's colour class. The dot colour stays the
   only *visible* signal.

2. **Only the current row's dot glows.** Today `status-current` (green),
   `status-background` (amber) and `status-starting` (blue) all carry a
   `box-shadow` glow in `sidebar.css`. Keep the glow on `status-current` alone.
   `status-background` and `status-starting` become flat dots — same colour, no
   shadow. `status-visible` and `status-parked` are already flat and do not
   change.

3. **Fold Park/Start into the row's ⋯ menu.** A row currently has a standalone
   `gtk::Button` (label inverts `Park` ⇆ `Start`) *and* a `gtk::MenuButton`
   (the ⋯ button) whose `gio::Menu` holds "Keep running when hidden". Remove the
   standalone button. Add a Park/Start item to the ⋯ menu, above "Keep running
   when hidden". It keeps today's button behaviour: the item label reads `Park`
   while the account is live and `Start` once it is parked or starting, and the
   item is insensitive (shown, greyed) while the account is `Starting` so the
   start cannot be triggered twice. The menu item drives the existing
   `connect_parking_toggled` path — the window still decides direction from
   liveness (architecture rule 8); the menu carries no state.

Removing the word frees trailing-edge width, so re-derive the sidebar's
`width-request` by eye with a short account name on screen, exactly as design
rule 6 already requires when the trailing edge changes.

## Constraints

1. **Rewrite design rules 1, 2 and 5 in `docs/design.md`** to match the new
   behaviour — rule 1: dot-only marker, state on hover / accessible label, glow
   on `current` alone; rule 2: the row has no standalone action button, the
   invert-with-state Park/Start action is a menu item; rule 5: the ⋯ menu now
   carries the Park/Start action as well as the keep-once settings. Keep each
   rule's "one imperative, one why" shape. Append-only numbering stays — edit
   the rules in place, do not renumber.
2. The status word derivation (`status_label` in `session_sidebar/row.rs`) stays
   as the source of the tooltip / accessible-label text — do not inline the
   strings into the factory. Design rule 1's "single status key drives both"
   principle holds: the key still derives the dot class *and* the hover text.
3. `gtk::NoSelection`, the rebuild-on-`sync` model, and the row-activation path
   (`connect_row_activated`) are unchanged.
4. No new domain state, no core changes. This is shell + CSS + the design doc
   only. `idle-manager-core` is not touched.
5. Update `crates/idle-manager-shell` tests that assert on the status word being
   a visible label or on the standalone button existing. The `status_key` /
   `status_label` / `action_label` / `action_sensitive` pure-function tests in
   `row.rs` stay green (the derivations still exist; only their consumers move).
6. Follow the repo's branch-first rule: cut `feat/…`-style session branch before
   the first code edit (this prompt has no roadmap number — name it
   `feat/sidebar-marker-polish` or similar). Run `make verify` before handing
   back.
7. This change has no roadmap item behind it. If the executor judges the design
   rule rewrites need to trace to `docs/requirements.md`, flag that to the user
   rather than inventing a requirement.

## Output

Rust + CSS changes in `crates/idle-manager-shell/`, prose edits to
`docs/design.md`. No new files expected.
