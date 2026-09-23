# 07 — The phone dialog and the header menu

**Roadmap:** [13](../../roadmap/13-phone-operation/README.md) · **Scope:** front-end · **Depends on:** 06

## Context

Until now the only way to enrol a phone was to read the enrolment address out
of a log line. This slice gives the desktop the small window meant for it.

A new menu button in the header bar holds two items: `Enrol a phone…` and
`Un-enrol the phone`. The first opens a dialog titled `Phone`. Its top line
says where things stand: no phone enrolled, a phone enrolled, a phone
connected right now, or, dimmed, that the desktop is not listening because no
mesh network address was found at start-up. Pressing `Enrol…` asks the server
for a one-time address and shows it two ways, as a QR code the phone's camera
can read and as a selectable line of text for typing by hand, with a countdown
under them. The address is good for ten minutes. When the phone scans it, the
status line flips to `Phone enrolled` on its own within a second, and the code
disappears. If the countdown runs out, the code disappears and the line says to
start again. `Un-enrol`, styled as the destructive action it is, cuts the phone
off at once; the menu item does the same without opening the dialog.

The dialog never raises an error window of its own. A desktop that is not
listening simply shows why in the dim line and greys out `Enrol…`, while
`Un-enrol` still works if a phone was enrolled earlier.

It is its own slice because it is the one screen in this item drawn with the
desktop toolkit, and because it is only worth trying once the wiring slice
has connected the server to the window.

## User experience

- **Entry** — A new header-bar menu button holds `Enrol a phone…` and
  `Un-enrol the phone`.
- **Flow** — Enrol: header menu → `Enrol a phone…` → the phone dialog shows a
  QR code and the same address as text, valid for ten minutes → scan it with
  the phone → the dialog's status line changes to `Phone enrolled` and the QR
  code disappears.
- **Flow** — Un-enrol: header menu → `Un-enrol the phone` → the phone is cut
  off at once and the desktop's dialog status returns to `No phone enrolled`.
- **States** — Not listening: the dialog's status line reads
  `Not listening: no mesh network address found` and `Enrol a phone…` is
  insensitive; the rest of the dialog stays usable. Expired code: the QR block
  empties and the line reads `The code expired; start again`.
- **Pattern** — The not-listening line is design rule 8's one dim line inside
  a form that keeps working, never a modal.
- **New pattern** — a dialog that shows a QR code and the same address as
  selectable text for a fixed time. Nothing in `docs/design.md` covers a
  pairing screen; the design doc owes a rule once this ships.

## Technical details

- **Architecture** — `window.ui` adds a `gtk::MenuButton` `phone_menu` with
  `open-menu-symbolic` to the right of the linked layout toggles and before
  `Add game`, as the wireframe places it, its `gio::Menu` holding
  `Enrol a phone…` (`win.enrol-phone`) and `Un-enrol the phone`
  (`win.revoke-phone`); `window/imp.rs` registers both `gio::SimpleAction`s,
  keeps `revoke-phone` enabled only while `phone_status()` is `Enrolled`, and
  refreshes that on every `redraw()` (rule 8).
- **Architecture** — new `phone_dialog.rs` + `phone_dialog/imp.rs` +
  `ui/phone-dialog.ui` registered in `idle-manager.gresource.xml` (rules 12,
  13; naming rules 4, 7): a small modal `gtk::Window` titled `Phone`,
  transient for the main window, with a status `gtk::Label`, a `gtk::Picture`
  for the code, a selectable address `gtk::Label`, a countdown `gtk::Label`,
  and `Enrol…` and `Un-enrol`
  (`destructive-action`) buttons; the dialog takes an `Arc<dyn PhoneLink>` and
  reads nothing else.
- **Architecture** — `Enrol…` calls `begin_enrolment()`, renders the offered
  address with the `qrcode` crate (pure Rust, added to the shell's
  `Cargo.toml` from the workspace pin) into a `gdk::MemoryTexture` (module
  `phone_dialog/qr.rs`), shows the address text, and arms a
  `glib::timeout_add_local` every second that updates the countdown from
  `expires_in_secs`, polls `phone_status()`, and at zero calls
  `cancel_enrolment()`, clears the block and sets the expired line; when the
  status turns `Enrolled` the block clears and the line reads `Phone enrolled`
  (rule 10).
- **Architecture** — the status line maps `PhoneStatus`: `NotEnrolled` →
  `No phone enrolled`; `Enrolled { attached: false }` → `Phone enrolled`;
  `Enrolled { attached: true }` → `Phone connected`; `NotListening { reason }`
  → `Not listening: <reason>` with the `dim-label` class and `Enrol…`
  insensitive; `Enrol…` is also insensitive while a code is live; `Un-enrol`
  is insensitive unless `Enrolled` (design rules 2, 8).
- **Architecture** — `Un-enrol` and the menu item both call `revoke_phone()`
  then refresh the line and the action's enabled state.
- **Code standards** — the status mapping and the countdown formatting are
  pure functions with unit tests (rules 21, 25); the polling timer is removed
  on dialog close (rule 14 spirit: nothing left running).

## Acceptance criteria

- [x] `(unit)` the status mapping yields the four lines above from the four
      `PhoneStatus` values, with the dim class only for `NotListening`
- [x] `(unit)` the countdown formats 600 s as `Valid for 10:00` and 59 s as
      `Valid for 0:59`
- [x] `(unit)` the `phone-dialog.ui` template is readable from the registered
      bundle (the existing template test pattern in `lib.rs`)
- [x] `(integration)` `make verify` passes
- [x] `(manual)` `Enrol a phone…` shows a scannable QR code and the same
      address as text; scanning it on the phone flips the line to
      `Phone enrolled` within a second and the code disappears
- [x] `(manual)` letting the countdown expire empties the code block and
      shows `The code expired; start again`; `Enrol…` becomes sensitive again
- [x] `(manual)` `Un-enrol the phone` from the menu cuts the connected phone
      off at once and greys the menu item; the dialog then reads
      `No phone enrolled`
- [x] `(manual)` with Tailscale stopped, the dialog reads
      `Not listening: no mesh network address found` in dim text, `Enrol…` is
      insensitive, and the window is otherwise unchanged

## References

- [Roadmap item](../../roadmap/13-phone-operation/README.md) — Front-end
  "The header bar" (the menu) and "The phone dialog"; the "Enrolling the
  phone" diagram
- [`openapi.json`](../../roadmap/13-phone-operation/openapi.json) —
  `GET /enrol/{code}`
- [Sequence diagrams](../../roadmap/13-phone-operation/sequence-diagrams.md)
  — `GET /enrol/{code}`
- [Wireframes](../../roadmap/13-phone-operation/wireframes/) —
  `phone-dialog.md`, `header-bar-and-mobile-layout.md`
- [`docs/requirements.md`](../../requirements.md) — Remote Access `FR.5.1`,
  `FR.6.1`, `FR.6.2`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10, 12, 13
- [`docs/code-standards.md`](../../code-standards.md) — rules 14, 21, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 4, 7
- [`docs/design.md`](../../design.md) — rules 2, 8

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
