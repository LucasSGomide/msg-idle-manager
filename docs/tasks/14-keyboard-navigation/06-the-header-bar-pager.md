# 06 — The header-bar pager

**Roadmap:** [14](../../roadmap/14-keyboard-navigation/README.md) · **Scope:** front-end · **Depends on:** 02, 04

## Context

This application keeps several browser idle games running at once in one
window. A workspace's accounts now read as pages — the first two, the next
two, and so on, with the page size set by the arrangement — and the keyboard
can step through them. What is missing is a way to see that pages exist and to
turn them with the mouse. Today nothing on screen says a workspace of six
accounts shown two at a time has two more pages behind the one in view.

This slice adds a small pager to the window's header bar, directly left of the
`1` `2` `4` `Phone` arrangement buttons: a `‹` arrow, a readout such as
`2/3`, and a `›` arrow, joined in one linked box the way the arrangement
buttons already are. Clicking an arrow turns a whole page and focuses the
first place of the new page; past the last page wraps to the first and before
the first wraps to the last. The readout uses figures of equal width, so
`9/10` and `10/10` sit in the same space and the header does not shift as
pages turn. The pager is present only while the shown workspace has more than
one page; with one page it is hidden entirely and the arrangement buttons sit
where they do today. Its hover text names the keyboard equivalent, `Next
account (Shift+Tab)`, so the key can be learnt from the control it mirrors.

The pager reads everything from the domain — which page, how many, whether
there is more than one — and asks the domain to turn the page; the window then
redraws as it does after any other change. The readout and visibility are
refreshed inside the window's one redraw, so a layout change, an added
account, a workspace switch or a key press all keep the pager truthful
without any of them knowing it exists.

It is its own slice because it is the only visible new control in the header
and the single place the window's interface description changes for this
item besides the menu, and because it depends on both the domain's page turns
and the window's shortcut function, which the redraw it hooks into shares.

## User experience

- **Entry** — The pager, in the header bar directly left of the `1` `2` `4`
  `Phone` layout toggles, shown only while the shown workspace has more than
  one page.
- **Flow** — Turn a page: click `›` or `‹` → the whole page changes → the new
  page's first slot is focused, its sidebar row goes bold → the readout reads
  the new `n/m`. Past the last page wraps to the first, and before the first
  wraps to the last.
- **Flow** — Change layout: the pager appears or disappears as the page count
  changes. Learn the keys: hover the pager → `Next account (Shift+Tab)`.
- **States** — One page: the pager is hidden. With a phone attached the phone
  follows the focus, as it does for any focus change.
- **Pattern** — A linked box of two icon buttons and a label, the same
  linked-box shape as the layout toggles (`window.ui` `layout_toggles`,
  `.linked`), with `go-previous-symbolic` / `go-next-symbolic` icons; the
  readout uses tabular figures as design rule 11 asks of a measured figure.
  The pager is an action, not an acknowledgement — no transient figure
  (design rule 10) — and a page turn re-keys the sidebar rows without any
  heading gaining a mark of the page (design rule 13).
- **New pattern** — a pager in the header bar: two arrows around an `n/m`
  readout, hidden when there is one page. Nothing in `docs/design.md` covers
  paging through a workspace; the design doc owes a rule once this ships
  (written by task 08).

## Technical details

- **Architecture** — `window.ui` gains a `<child type="end">` declared after
  `layout_toggles` — header bars pack `end` children right to left, so this
  lands directly left of the toggles — holding a `GtkBox` `pager` with the
  `linked` style class and `tooltip-text` `Next account (Shift+Tab)`
  (`FR.25.1`), containing `GtkButton` `page_previous` (`icon-name`
  `go-previous-symbolic`, tooltip `Previous page`), `GtkLabel` `page_readout`
  with the `numeric` style class and `width-chars` 5, and `GtkButton`
  `page_next` (`go-next-symbolic`, tooltip `Next page`) (`FR.22.5`; rule 13).
- **Architecture** — `window/imp.rs` binds the three as `TemplateChild`s;
  `constructed` connects `page_previous` to `book.previous_page()` and
  `page_next` to `book.next_page()`, each followed by `sync_watched`, `redraw`,
  `request_save` — the same three calls `run_shortcut`'s `NextAccount` arm
  makes (rule 8).
- **Architecture** — `redraw` sets `page_readout`'s text to
  `format!("{}/{}", page + 1, pages)` and `pager.set_visible(pages > 1)`,
  reading `page()` and `page_count()` from `book.active()`;
  `select_layout_toggle` is unchanged. The phone follows because a page turn
  is a focus change the existing `sync_watched` / `publish_state` path
  already reports (Remote Access `FR.4.2`).
- **Design** — the readout uses the `numeric` style class for tabular
  figures (rule 11); the pager carries no dot, bold or mark of the shown
  page beyond the readout. The third Blocker — whether the Windows VM's
  default font carries `tnum` — is checked by task 08; `width-chars` 5 is set
  now so a font without it still cannot shift the toggles.
- **Code standards** — no logic lives in the widget beyond reading two
  numbers and calling two book methods (rule 6); the wrap is the book's.

## Acceptance criteria

- [ ] `(unit)` the `window.ui` template still reads from the registered bundle
      and contains the ids `pager`, `page_previous`, `page_readout` and
      `page_next`
- [ ] `(integration)` `make verify` passes
- [ ] `(manual)` with two accounts in `2` the pager is absent; adding a third
      makes it appear reading `1/2`, directly left of the layout toggles
- [ ] `(manual)` clicking `›` on `1/2` shows the third account alone with its
      slot outlined and its sidebar row bold, and the readout reads `2/2`;
      clicking `›` again wraps to `1/2` with the first account focused, and
      `‹` from `1/2` wraps to `2/2`
- [ ] `(manual)` switching from `2` to `4` with three accounts hides the
      pager; switching to `1` shows it reading `n/3` with the focused account
      on screen
- [ ] `(manual)` hovering the pager reads `Next account (Shift+Tab)`; the
      arrows read `Previous page` and `Next page`
- [ ] `(manual)` with ten accounts in `Ungrouped` in `1`, stepping from `9/10`
      to `10/10` does not move the layout toggles
- [ ] `(manual)` with the phone attached, clicking `›` changes the account the
      phone shows

## References

- [Roadmap item](../../roadmap/14-keyboard-navigation/README.md) — Front-end
  "The pager"; the "Turning the page" diagram; Technical References, the
  pager bullet; Blockers, the third
- [Wireframes](../../roadmap/14-keyboard-navigation/wireframes/) —
  `header-bar-pager.md`
- [`docs/requirements.md`](../../requirements.md) — `FR.22.5`, `FR.25.1`;
  Remote Access `FR.4.2`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 12, 13
- [`docs/code-standards.md`](../../code-standards.md) — rules 6, 21, 25
- [`docs/design.md`](../../design.md) — rules 10, 11, 13
- [`docs/naming.md`](../../naming.md) — rules 4, 7

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
