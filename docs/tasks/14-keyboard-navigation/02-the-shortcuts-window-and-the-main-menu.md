# 02 — The shortcuts window and the main menu

**Roadmap:** [14](../../roadmap/14-keyboard-navigation/README.md) · **Scope:** front-end · **Depends on:** —

## Context

This application keeps several browser idle games running at once in one
window. The window already answers a few keys of its own — `F5` and `Ctrl`+`R`
reload the focused game, `Ctrl` with plus, minus and zero change its zoom —
and this roadmap item adds two more, one that steps to the next account and
one that steps to the next workspace. None of them is written down anywhere a
person using the program can find it. A game's page fills the window, the
buttons that share those keys only name them in a hover tooltip, and the two
new keys have no button at all.

This slice makes the keys discoverable from the program itself, using the
standard GTK way of doing so. GTK ships a ready-made "shortcuts window": a
small overlay that lists a program's keys in groups, opened with `Ctrl`+`?`.
The toolkit finds it on its own, by looking for a resource file at a fixed
name next to the program's other interface files, and attaches it to the main
window without a line of application code. So the work here is a description
file listing seven entries — reload, zoom in, zoom out, reset zoom, next
account, next workspace and the shortcuts window itself — registered under
that fixed name, plus one new item at the end of the header bar's ☰ menu,
`Keyboard Shortcuts`, that opens the same overlay with the mouse. Because the
menu no longer holds only phone-related items, its hover text becomes plain
`Menu`.

The two navigation keys are listed before the slices that make them work land,
so the list is complete when the item ships and this slice never has to be
reopened. It is its own slice because it touches nothing any other slice
edits: a new resource file, the resource manifest, the window's interface
description, and one test that the file is readable from the compiled bundle.
That makes it safe to build beside the deep change to the workspace model
that opens this item.

## User experience

- **Entry** — A `Keyboard Shortcuts` item at the end of the ☰ menu, and
  `Ctrl`+`?`.
- **Flow** — Learn the keys: ☰ → `Keyboard Shortcuts`, or `Ctrl`+`?` → a
  shortcuts window lists reload, zoom in, zoom out, reset zoom, next account,
  next workspace and the window itself. `Esc` or the close button hides it;
  nothing in the workspace changes.
- **States** — Shown or hidden. There is no empty state: the list is fixed.
- **Pattern** — The window is an action the user opened, not an
  acknowledgement: it takes focus and closes on `Esc` (design rule 10). On
  Windows it is its own native surface, so it draws above a `WebView2` child
  window as the zoom popover does (design rule 14); no overlay layer is used.
- **New pattern** — a GTK shortcuts window listing the window's keys. Nothing
  in `docs/design.md` covers a help overlay; the design doc owes a rule once
  this ships (written by task 08).

## Technical details

- **Architecture** — add
  `crates/idle-manager-shell/resources/ui/help-overlay.ui` holding a
  `GtkShortcutsWindow` with id `help_overlay`, one `GtkShortcutsSection`, one
  `GtkShortcutsGroup` titled `Idle Manager`, and one `GtkShortcutsShortcut`
  per key: `F5 <Control>r` "Reload the focused game", `<Control>plus` "Zoom
  in", `<Control>minus` "Zoom out", `<Control>0` "Reset zoom", `<Shift>Tab`
  "Next account", `<Control>Tab` "Next workspace", `<Control>question`
  "Keyboard shortcuts" (`FR.25.2`; rule 13).
- **Architecture** — register it in `idle-manager.gresource.xml` at the alias
  `gtk/help-overlay.ui` (`<file alias="gtk/help-overlay.ui" …>ui/help-overlay.ui</file>`)
  under the existing prefix `/org/idlemanager/IdleManager`, which is the path
  `GtkApplication` derives from the id `org.idlemanager.IdleManager` in
  `crates/idle-manager/src/main.rs`. GTK then registers `win.show-help-overlay`
  on the `GtkApplicationWindow` and binds `Ctrl`+`?`; nothing is wired in
  Rust.
- **Architecture** — `window.ui`'s `phone_menu_model` gains a second
  `<section>` with one `<item>`: label `Keyboard Shortcuts`, action
  `win.show-help-overlay`. `phone_menu`'s `tooltip-text` becomes `Menu`.
- **Code standards** — `crates/idle-manager-shell/src/lib.rs` gains
  `the_help_overlay_is_readable_from_the_registered_bundle`, looking up
  `/org/idlemanager/IdleManager/gtk/help-overlay.ui`, following the existing
  template tests (rules 21, 25). `GtkShortcutsWindow` is deprecated from GTK
  4.18; `Cargo.toml` pins `v4_10`, so no deprecation lint fires and libadwaita
  is not added (rule 27: no `#[allow]`).
- **Naming** — `help-overlay.ui` is the one `.ui` not named after the widget
  it defines, because GTK looks it up by this fixed name; `docs/naming.md`
  rule 4 gains one sentence recording the exception.

## Acceptance criteria

- [x] `(unit)` `/org/idlemanager/IdleManager/gtk/help-overlay.ui` is readable
      from the registered bundle and is not empty
- [x] `(integration)` `make verify` passes, with no deprecation warning from
      `GtkShortcutsWindow`
- [x] `(manual)` `Ctrl`+`?` while a game page has keyboard focus opens the
      shortcuts window listing exactly the seven entries above, and `Esc`
      closes it with the grid unchanged
- [x] `(manual)` ☰ → `Keyboard Shortcuts` opens the same window; hovering ☰
      reads `Menu`
- [x] `(manual)` `docs/naming.md` rule 4 names `help-overlay.ui` as the one
      exception and says why

## References

- [Roadmap item](../../roadmap/14-keyboard-navigation/README.md) — Front-end
  "The shortcuts window"; the "The shortcuts window" diagram; Technical
  References, the fourth
- [Wireframes](../../roadmap/14-keyboard-navigation/wireframes/) —
  `shortcuts-window.md`
- [`docs/requirements.md`](../../requirements.md) — `FR.25.2`
- [`docs/architecture.md`](../../architecture.md) — rule 13
- [`docs/code-standards.md`](../../code-standards.md) — rules 21, 25, 27
- [`docs/design.md`](../../design.md) — rules 10, 14; no existing rule
  covers a help overlay
- [`docs/naming.md`](../../naming.md) — rule 4

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
