# 01 — The remote vocabulary, its two ports, and the phone record on disk

**Roadmap:** [13](../../roadmap/13-phone-operation/README.md) · **Scope:** back-end · **Depends on:** —

## Context

This application runs several browser idle games on an always-on desktop. A
later slice lets a phone watch and tap one of those games through a small
server the desktop runs. Before any of that can be built, the desktop and the
server need to agree on the words they exchange, and the server needs a place
to keep the one phone that is allowed in.

This slice writes those words into the pure domain crate, the part of the
program that knows the rules but touches no screen, disk or network. It
defines the closed list of things a phone may ask for: attach with a screen
size, leave, choose an account, park one, start one, switch mobile mode on or
off, tap at a point, scroll at a point. Nothing else exists in that list, so
adding, renaming or deleting an account from the phone is impossible by
construction rather than by a check someone could forget. It defines the
snapshot the phone renders: whether mobile mode is on, the viewport size,
which account is current, and every workspace with its accounts and their
state words. It defines the two shapes a captured picture can take, the
policy that decides when a silent phone counts as gone, and the JavaScript
text that turns a tap or a scroll into page events. Every one of these is
tested with plain values, with no display and no network.

It also defines the two contracts the rest of the program will implement: one
the desktop holds to talk to the server, one the server uses to read and
write the enrolled phone. The disk half of the second contract is built here
too: a small file under the user's configuration folder, readable only by the
user, holding the phone's identifier and secret. Finally the new server crate
is created empty, so the build checks that keep the crates in their lanes
know about it from the start, and the architecture document names it.

It is its own slice because two later slices, the desktop's fourth layout and
the server, both build on these types and can only run side by side once the
types exist.

## Technical details

- **Architecture** — add `crates/idle-manager-core/src/remote.rs` holding
  `RemoteIntent` (variants exactly: `Attach { viewport: Viewport }`, `Leave`,
  `ChooseAccount(SessionId)`, `Park(SessionId)`, `Start(SessionId)`,
  `SetMobileMode(bool)`, `Tap { x: u32, y: u32 }`,
  `Scroll { x: u32, y: u32, dx: i32, dy: i32 }`), `RemoteState`
  (`mobile_mode`, `viewport`, `current: Option<SessionId>`, `workspaces:
  Vec<RemoteWorkspace { name, accounts: Vec<RemoteAccount { id, name,
  liveness: Liveness }> }>`) with `RemoteState::from_book(&WorkspaceBook)`,
  `Viewport { width, height }` with `DEFAULT_MOBILE_VIEWPORT` = 412 × 915,
  `Frame::{Rgba { width, height, stride, bytes }, Jpeg(Vec<u8>)}`,
  `PhoneStatus::{NotListening { reason }, NotEnrolled, Enrolled { attached }}`,
  `EnrolledPhone { device_id, secret: Vec<u8>, enrolled_on }`,
  `EnrolmentOffer { address, expires_in_secs }`; re-export from `lib.rs`
  (rule 1: no serde, no I/O).
- **Architecture** — add `Presence::observe(now_millis) -> PresenceChange`
  in `remote.rs` with `HEARTBEAT_INTERVAL_SECS = 5` and
  `SILENCE_LIMIT_SECS = 15`: attached after `attach` plus a heartbeat, gone
  once the limit passes without one, emitting the leave transition exactly
  once (rule 9: time is an argument).
- **Architecture** — add pure `tap_script(x, y) -> String` and
  `scroll_script(x, y, dx, dy) -> String` in `remote.rs`: the tap finds
  `document.elementFromPoint` and dispatches `pointerdown`, `mousedown`,
  `pointerup`, `mouseup`, `click` with `clientX`/`clientY`; the scroll walks
  to the nearest scrollable ancestor or the window and adds the deltas.
- **Architecture** — add to `ports.rs`: `PhoneLink: Debug + Send + Sync` with
  `publish_state(&RemoteState)`, `publish_frame(Frame)`,
  `begin_enrolment() -> EnrolmentOffer`, `cancel_enrolment()`,
  `revoke_phone()`, `phone_status() -> PhoneStatus`; and
  `PhoneRecordStore: Debug + Send + Sync` with exactly
  `read() -> Result<Option<EnrolledPhone>, PhoneRecordError>`,
  `write(&EnrolledPhone)` and `clear()`; `PhoneRecordError` is a `thiserror`
  enum beside the port (`Unreadable { path, reason }`, `Inaccessible`), the
  `WorkspaceReadError` shape (rules 5, 6, 11; naming rule 10).
- **Architecture** — add `crates/idle-manager-store/src/phone_record.rs` with
  `TomlPhoneRecord` implementing `PhoneRecordStore` over
  `<XDG config>/idle-manager/phone.toml` (path helper `phone_file()` in
  `paths.rs` beside `workspace_file`), its own serde record types with the
  secret as hex (rule 7), the file created `0600` through
  `std::os::unix::fs::OpenOptionsExt` under `#[cfg(unix)]`, a missing file
  read as `None`, a malformed file moved aside to `phone.toml.unreadable` and
  reported as `PhoneRecordError::Unreadable`, the `TomlWorkspaceStore`
  precedent (rule 11); the same file's optional
  `[listen] address = "auto" | "<ip>:<port>"` key is read by the inherent
  `TomlPhoneRecord::listen_override() -> Option<String>`, not by the port.
- **Architecture** — create `crates/idle-manager-remote/` with `Cargo.toml`
  (depends on `idle-manager-core`, `serde`, `serde_json`, `sha1`, `base64`,
  `hmac`, `sha2`, `rand`, `jpeg-encoder`, `if-addrs`, `async-channel`) and an
  empty `lib.rs` with its crate doc; declare those crates and `qrcode` in the
  workspace root pinned to a minor version; add
  `idle-manager-remote gtk4 gdk4 glib gio webkit6 wry webview2-com gdk4-win32 idle-manager-shell idle-manager-store idle-manager-metrics`
  to `scripts/arch-check.sh` and `idle-manager-remote` to the shell's, store's
  and metrics' forbidden lists (rules 2, 4).
- **Architecture** — `docs/architecture.md`: the shape diagram, the folder
  structure and the "Where a change goes" table name `idle-manager-remote` as
  the second driving adapter; `docs/stack.md` "Libraries" lists the new crates
  with one-line reasons.
- **Code standards** — every new public item carries a `///` contract (rule
  17); constants carry units (rule 5); the core is unit-tested and the store
  integration-tested under `tests/` (architecture rule 14), tests sit at the
  foot of each file (rule 24) and need no display (rule 25).

## Acceptance criteria

- [ ] `(unit)` `RemoteState::from_book` on a book with two workspaces lists
      every account exactly once under its workspace with its `Liveness`, and
      `current` is the shown workspace's focused session
- [ ] `(unit)` `Presence` reports attached after `attach` and a heartbeat,
      stays attached at 14 s of silence, reports gone once at 15 s, and does
      not report gone a second time at 30 s
- [ ] `(unit)` `tap_script(120, 340)` contains `elementFromPoint(120, 340)`
      and dispatches `pointerdown`, `mousedown`, `pointerup`, `mouseup` and
      `click` in that order; `scroll_script` adds `dx`/`dy` to the nearest
      scrollable ancestor
- [ ] `(unit)` `RemoteIntent` has exactly the eight variants named above and
      no variant that names, renames, regroups or deletes an account, changes
      keep-awake, layout or zoom, or loads an address (an exhaustive match in
      the test)
- [ ] `(integration)` `TomlPhoneRecord::write` then `read` round-trips an
      `EnrolledPhone`, and on Unix the file's mode is `0600`
- [ ] `(integration)` a missing `phone.toml` reads as `Ok(None)`; a malformed
      one is moved to `phone.toml.unreadable` and read as
      `PhoneRecordError::Unreadable`
- [ ] `(integration)` `clear()` removes the record so a following `read` is
      `Ok(None)`, and `TomlPhoneRecord::listen_override()` returns the
      `[listen] address` value when present and `None` when the key is absent
- [ ] `(integration)` `make arch-check` passes with the new rules and fails
      when a test edit makes `idle-manager-remote` depend on
      `idle-manager-shell`
- [ ] `(integration)` `make verify` passes

## References

- [Roadmap item](../../roadmap/13-phone-operation/README.md) — Back-end "A
  sixth crate", "The domain vocabulary", "Two new ports", "Delivering a tap
  and a scroll", "The store"; Technical References: script-dispatched events
  carry `isTrusted = false`, which is why `tap_script` dispatches the full
  pointer/mouse/click sequence rather than `click` alone
- [`openapi.json`](../../roadmap/13-phone-operation/openapi.json) —
  `RemoteState`, `Liveness`, `Viewport` and the `Phone*` message shapes the
  intents mirror
- [Sequence diagrams](../../roadmap/13-phone-operation/sequence-diagrams.md)
  — the `PhoneRecordStore` calls in `GET /enrol/{code}` and `GET /ws`
- [Wireframes](../../roadmap/13-phone-operation/wireframes/) — the whole item
- [`docs/requirements.md`](../../requirements.md) — Remote Access `FR.4.5`,
  `FR.6.1`, `FR.6.3`
- [`docs/architecture.md`](../../architecture.md) — rules 1, 2, 4, 5, 6, 7,
  9, 11, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 1, 5, 17, 24, 25
- [`docs/naming.md`](../../naming.md) — rules 5, 10

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
