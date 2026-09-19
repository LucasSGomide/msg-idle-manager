# 06 — Wiring the phone into the window

**Roadmap:** [13](../../roadmap/13-phone-operation/README.md) · **Scope:** full-stack · **Depends on:** 02, 03, 04

## Context

By now the pieces exist apart: the desktop can draw one game at a phone's
shape, the engine seam can photograph a page and run a script in it, and a
server in its own crate lets the enrolled phone in and passes its messages on.
This slice connects them so the whole thing works end to end for the first
time.

The program's start-up code builds the server, hands it the file that
remembers the phone, and passes the server's handle and its stream of
messages into the window. If the server cannot start, because no mesh network
address was found, that is logged and remembered as a status a later dialog
shows, and the window opens as usual. The window then listens for the phone's
messages and turns each into the same action a click on the desktop would
take: choosing an account focuses it, park and start do what the sidebar
menu does, the mobile mode message flips the fourth layout toggle, a tap or a
scroll runs the matching script in the current game's page. Nothing else is
possible, because the message list has nothing else in it.

Every time the window redraws it also sends the phone a fresh snapshot of the
accounts and their states, so the phone's list is never stale. While the
phone is attached and mobile mode is on, a timer photographs the current game
about twelve times a second and sends each picture to the server, which
compresses and forwards it. Only the one game on the phone's screen is
photographed, and that game alone is told it is being watched, so a game the
owner left to rest wakes for the visit and rests again the moment the phone
moves to another account or leaves. The picture size follows the phone's own
screen size when the phone attaches.

This is one slice because the wiring only makes sense as a whole: a message
loop with no pictures, or pictures with no messages, cannot be tried by
anyone.

## User experience

- **Flow** — Turn on mobile mode from the phone: tap the toggle → the desktop
  switches to the `Mobile` layout at the phone's own screen size → the page
  shows the current account's game within a second. Turn mobile mode off from
  either end → the desktop restores the arrangement it had before mobile mode
  was entered.
- **Flow** — Switch account: if that account belongs to another workspace,
  the desktop switches workspace, as clicking its sidebar row does. Park or
  start from the phone does what the desktop's row menu item does.
- **Flow** — Leave: the desktop stops sending pictures within a second and
  the account it was showing returns to its resting behaviour. Coming back
  resumes on the same account.
- **States** — Desktop, `Mobile` layout entered from the phone: the slot takes
  the phone's viewport rather than the default.

## Technical details

- **Architecture** — `crates/idle-manager/src/main.rs` builds
  `TomlPhoneRecord`, resolves `bind_address(record.listen_override())`, calls
  `RemoteServer::start`, and passes `WindowPorts.phone: Option<PhonePorts { link: Arc<dyn PhoneLink>, intents: async_channel::Receiver<RemoteIntent> }>`;
  a `StartError` is logged and becomes `PhonePorts { link: NotListeningLink(reason), intents: <never-yielding receiver> }`
  where `NotListeningLink` is a small `PhoneLink` in the remote crate whose
  `phone_status()` is `NotListening { reason }` and whose other methods are
  no-ops; a `PhoneRecordError` from the record's first `read()` is logged
  with the reason and the server starts as `NotEnrolled` (the store has
  already moved the file aside); the shell's `Cargo.toml` gains
  `async-channel` from the workspace pin for the receiver it holds (rule 3).
- **Architecture** — `window/imp.rs` `attach_ports` stores the link and
  spawns `glib::spawn_future_local` looping `intents.recv().await` into
  `apply_remote_intent(intent)` (rule 10); `apply_remote_intent` maps
  `ChooseAccount` → `focus_session`; `Park` → `park_session` only when the
  account is `Live` (searched with `liveness_anywhere`); `Start` →
  `start_session` only when `Parked`; `SetMobileMode(true)` → the `Phone`
  toggle path with the attached viewport or the default; `SetMobileMode(false)`
  → the leave path; `Attach { viewport }` → set `attached`, call
  `set_mobile_viewport` when the mode is on and the size differs, start the
  capture timer, `set_watched(true)` on the current holder; `Leave` → stop
  the timer, `set_watched(false)`, clear `attached`; `Tap`/`Scroll` →
  `run_script(tap_script(..))`/`run_script(scroll_script(..))` on the current
  account's live view, ignored while mobile mode is off or it has no view
  (rule 8).
- **Architecture** — `redraw()` ends with
  `link.publish_state(&RemoteState::from_book(&book))`; the current account is
  `book.active().focused_session()`.
- **Architecture** — the capture timer is a `glib::timeout_add_local` every
  `FRAME_INTERVAL_MILLIS = 80` that, when `attached && is_mobile_mode() &&
  current is Live && no capture in flight`, calls the current holder's
  `capture_frame` and hands the result to `publish_frame`, logging a failure
  once per distinct reason; a `Cell<bool>` marks the in-flight capture; with
  `IDLE_MANAGER_DUMP_FRAMES=<dir>` set (task 02's switch) the window also
  writes every captured frame to that folder.
- **Architecture** — on `focus_session` while attached, `set_watched(false)`
  is applied to the previous current holder and `set_watched(true)` to the
  new one before the next tick; `apply_minimised` passes the new `minimised`
  through `set_watched` for the watched account so a restore never marks it
  background.
- **Architecture** — the `Phone` toggle from the desktop passes the attached
  phone's viewport when `attached`, else `DEFAULT_MOBILE_VIEWPORT`.
- **Code standards** — the handler closures hold `glib::WeakRef`s to the
  window (item 12's convention); constants carry units (rule 5); no
  `let _ =` on a `publish_*` result (rule 14).

## Acceptance criteria

- [ ] `(unit)` `apply_remote_intent(Park(id))` on a `Parked` account changes
      nothing, and `Start(id)` on a `Live` account changes nothing (tested on
      the pure decision helper the handler calls)
- [ ] `(unit)` the capture gate returns true only when attached, mobile mode
      on, current `Live` and not in flight (table-driven over the four
      booleans)
- [ ] `(unit)` `NotListeningLink::phone_status()` is `NotListening { reason }`
      with the `StartError`'s text, and its `publish_state`, `publish_frame`
      and `revoke_phone` return without effect
- [ ] `(integration)` `make verify` and `make windows-check` pass
- [ ] `(manual)` with the phone attached, `make dev` logs show the
      `publish_state` on every sidebar change and frames flowing at about
      12 per second (`RUST_LOG=idle_manager_remote=debug`)
- [ ] `(manual)` tapping the mobile mode toggle on the phone flips the
      desktop's `Phone` toggle and the slot takes the phone's reported size;
      turning it off from the phone restores the previous arrangement
- [ ] `(manual)` choosing an account in another workspace from the phone
      switches the desktop's shown workspace and the phone shows that game;
      `Park` from the phone parks it on the desktop
- [ ] `(manual)` with the window minimised and a non-keep-awake account
      current, the phone shows the game animating; after the phone leaves,
      the account's `set_background`/feature state returns to what
      `background_for` dictates (visible in the debug log)
- [ ] `(manual)` when no mesh address exists (`[listen] address = "0.0.0.0:0"`
      removed and Tailscale stopped), the application starts normally and the
      log names `NoMeshAddress`

## References

- [Roadmap item](../../roadmap/13-phone-operation/README.md) — Back-end "The
  composition root"; Front-end "Wiring the phone into the window"; the
  "Watching and tapping a game" and "Switching account from the phone"
  diagrams
- [`openapi.json`](../../roadmap/13-phone-operation/openapi.json) — the
  whole item
- [Sequence diagrams](../../roadmap/13-phone-operation/sequence-diagrams.md)
  — `GET /ws`
- [Wireframes](../../roadmap/13-phone-operation/wireframes/) —
  `header-bar-and-mobile-layout.md`
- [`docs/requirements.md`](../../requirements.md) — Remote Access `FR.1.3`,
  `FR.1.4`, `FR.2.2`, `FR.2.3`, `FR.3.1`–`FR.3.3`, `FR.4.2`, `FR.4.3`, `FR.6.3`
- [`docs/architecture.md`](../../architecture.md) — rules 3, 8, 10
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 14
- [`docs/design.md`](../../design.md) — rule 1

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The phone `(manual)` steps
use the phone page slice's page and a phone on the same mesh network.
