# 08 — Windows verification, the design rules and the runbook

**Roadmap:** [14](../../roadmap/14-keyboard-navigation/README.md) · **Scope:** full-stack · **Depends on:** 05, 07

## Context

Everything this item builds is now in place: workspaces read as pages, a
pager in the header bar, two keys that step through accounts and workspaces
on both engines, `Park all` and `Start all` on every workspace heading, and
a shortcuts window. This closing slice proves the whole of it on Windows,
settles the one open question the earlier slices could not, and writes down
the two new screen patterns the item introduced so later work follows them.

The application runs on Linux and on Windows, and the project has no Windows
computer — only a Windows 11 virtual machine inside a container on the Linux
development machine, driven with a physical keyboard over a remote desktop.
The Windows keys slice already proved the two chords there. What has not been
run against the Windows build is the rest: the pager turning pages, a drop
onto a part-empty last page, `Park all` on a workspace with an account still
loading, the shortcuts window, and a launch from a workspace file written by
the previous build showing the same page and focused account as before, with
the backup copy written once. This slice runs those in the virtual machine
and records the outcome in the item's hand-run test script, beside the Linux
runs the earlier slices recorded.

The open question is the pager's readout. It asks the font for figures of
equal width so `9/10` and `10/10` take the same space, but that depends on
the font carrying that feature, and the default font on the Windows machine
is unverified. If the readout jitters there, the fixed width already set on
the label is the fallback and the request for equal-width figures is
dropped, with the reason recorded.

Finally the design document, which holds the rules every screen in the
program follows, gains two rules: one for a pager in a header bar — two
arrows around an `n/m` readout, hidden rather than greyed when there is one
page, and why — and one for a shortcuts window built from the toolkit's own
overlay, listing every key the window has. Each names the widget and the
slice that built it, as the existing rules do. The test script is also
checked against the minimum the item asked of it, so nothing the item
promised to prove by hand is left unproven.

## User experience

- **Flow** — On Windows, the pager, the heading menu items and the shortcuts
  window appear and behave exactly as on Linux.
- **States** — Nothing new is drawn; this slice adds no screen. The one
  possible change is the readout losing tabular figures on Windows if the
  font lacks them, in which case its fixed width keeps the toggles still.

## Technical details

- **Design** — `docs/design.md` gains rule 18, a pager in a header bar: two
  icon-button arrows around an `n/m` readout in one linked box beside the
  controls it pages for, tabular figures, hidden rather than insensitive
  when there is one page because an always-present pager would imply pages
  exist (the GNOME HIG would disable; hiding was the recorded choice,
  `FR.22.5`), named after `window.ui` `pager` and this item's task 06.
- **Design** — `docs/design.md` gains rule 19, a shortcuts window: every key
  the window answers is listed in GTK's own `GtkShortcutsWindow` from a
  `gtk/help-overlay.ui` resource, opened by `Ctrl`+`?` and a `Keyboard
  Shortcuts` menu item, and a control that mirrors a key names it in its
  tooltip (`FR.25.1`, `FR.25.2`), named after `help-overlay.ui` and this
  item's task 02.
- **Architecture** — the Windows half of the runbook is run in the VM from
  `scripts/windows-vm/compose.yml` with a physical keyboard: the pager on the
  third account, a drop on a part-empty last page, `Park all` with one
  starting account, the shortcuts window, and a launch from a version 2
  `%APPDATA%\idle-manager\sessions.toml` showing the same page and focused
  account with `sessions.v2.toml` written once. Any Windows-only defect is
  fixed inside `web_engine/webview2/` with the constraint that forced it
  (code standards rule 18). `docs/windows-vm.md` "What the VM can and cannot
  prove" gains the keyboard chords and the pager to its list.
- **Design** — the third Blocker is settled: if `9/10` → `10/10` shifts the
  layout toggles in the VM, the `numeric` class is removed from
  `page_readout` and the fixed `width-chars` stands alone, with the reason in
  `window.ui` (rule 11's tabular figures are then met by width, not by the
  font).
- **Code standards** — `docs/tasks/14-keyboard-navigation/test-script.md` is
  checked against the item's minimum: the four-in-two walk on X11 with
  `Shift`+`Tab` while a game canvas has focus, `Ctrl`+`Tab` across three
  workspaces with one empty, the pager appearing on the third account, a
  drop on a part-empty last page, `Park all` with one starting account, and
  the version 2 launch with `sessions.v2.toml` written once; a step missing
  from the earlier sections is added to this slice's own section, never to
  theirs (architecture rule 14).

## Acceptance criteria

- [x] `(integration)` `make verify` and `make windows-package` pass
- [x] `(manual)` in the Windows VM, the pager appears on the third account,
      turns and wraps, and stepping `9/10` → `10/10` does not move the layout
      toggles — or the `numeric` class is removed and the reason recorded
- [x] `(manual)` in the VM, a drop onto the empty trailing slot of a
      part-empty last page moves the account there and the sidebar order
      follows; `Park all` with one starting account parks it on paint;
      `Ctrl`+`?` opens the shortcuts window
- [x] `(manual)` in the VM, launching over a version 2 `sessions.toml` shows
      the page holding the previously focused account with that account
      focused, and `sessions.v2.toml` is written once and unchanged by a
      second save
- [x] `(manual)` `docs/design.md` carries rules 18 and 19 in the file's
      shape, and `docs/windows-vm.md` lists the chords and the pager among
      what the VM proves
- [x] `(manual)` `test-script.md` holds every step in the item's minimum list,
      each ticked after a run on Linux, and the Windows runs above under this
      slice's section

## References

- [Roadmap item](../../roadmap/14-keyboard-navigation/README.md) — Front-end
  "Tests and the runbook"; User Experience, both `**New pattern**` bullets;
  Technical References, the pager bullet; Blockers, the third
- [Wireframes](../../roadmap/14-keyboard-navigation/wireframes/) — all three;
  their `## Design rules` sections name what the two new rules must cover
- [`docs/requirements.md`](../../requirements.md) — `FR.22.2`, `FR.22.5`,
  `FR.23.5`, `FR.25.1`, `FR.25.2`
- [`docs/architecture.md`](../../architecture.md) — rule 14
- [`docs/code-standards.md`](../../code-standards.md) — rule 18
- [`docs/design.md`](../../design.md) — rules 11, 13, and the two this slice
  adds
- [`docs/windows-vm.md`](../../windows-vm.md) — the daily loop and the proof
  list

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The VM steps follow
`docs/windows-vm.md`'s daily loop with `make windows-package` and a physical
keyboard, as item 12's runbook does.
