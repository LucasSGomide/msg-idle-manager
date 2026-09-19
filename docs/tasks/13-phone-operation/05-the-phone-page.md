# 05 — The phone page

**Roadmap:** [13](../../roadmap/13-phone-operation/README.md) · **Scope:** front-end · **Depends on:** 04

## Context

The owner's phone needs something to open. This slice writes that something:
a single web page the desktop's own server hands to the phone, which the owner
opens once in the phone's browser and adds to the home screen, after which it
opens like an app. One page covers Android and iPhone alike. It is plain HTML
with its styling and script inside, no framework and no build step, because
it has exactly three screens and one job.

The first screen appears while mobile mode is off on the desktop: one large
switch in the middle and a line saying to turn it on to see a game. Nothing
else. Turning it on asks the desktop to switch to its phone-shaped
arrangement, and the page becomes the second screen: the game, filling the
phone screen, drawn from the pictures the desktop keeps sending. A short touch
is sent as a tap at that point of the page; a drag is sent as a scroll. A
small round handle in the top corner opens the third screen, a panel that
slides over the game listing every account under its workspace, with the same
state word the desktop shows and one button per account that reads `Park`
while it runs and `Start` when it does not. Tapping a name switches the game.
The panel also holds the switch to turn mobile mode back off.

The page proves itself before it sees anything. On connecting it receives a
challenge, answers with a code derived from the secret it was given at
enrolment, and checks the desktop's own code in return. It tells the desktop
when it is being looked at and when it is not: switching apps or locking the
phone sends a leave, coming back sends an attach with the screen's size, and a
heartbeat goes out every five seconds so a phone that simply vanishes is
noticed. If the desktop un-enrols the phone, the page shows one line saying so
and nothing more.

It is one slice because it is one file, served by the route the server slice
already built, and because nothing in it can be tried until the server exists.

## User experience

- **Entry** — On the phone, the experience is the web page reached from the
  enrolment link and added to the home screen. Enrol, phone side: scanning
  the code opens the page in the phone's browser, which stores its credential
  and shows the mobile mode toggle.
- **Flow** — Turn on mobile mode from the phone: tap the toggle → the desktop
  switches to the `Mobile` layout at the phone's own screen size → the page
  shows the current account's game filling the screen within a second. Turn
  mobile mode off from either end → the phone page shows only the toggle
  again. Un-enrol from the desktop → the phone is cut off at once, its page
  reads that it was un-enrolled and offers nothing else.
- **Flow** — Switch account: tap the handle in the top-left corner → the
  account list slides in over the game → tap a name → the list slides away and
  the game changes to that account. Park or start from the phone: in the
  list, each account row carries one button reading `Park` while it runs and
  `Start` once it is parked; tapping it does what the desktop's row menu item
  does, and the row's state word follows. Tap and scroll the game: a short
  touch is a tap at that point; a drag scrolls the page under the finger.
  There is no keyboard. Leave: switch to another app, lock the phone, or open
  the list and choose nothing; the desktop stops sending pictures within a
  second, and coming back resumes on the same account.
- **States** — Mobile mode off, phone connected: the page shows only the
  mobile mode toggle, centred, and nothing else. Current account parked,
  queued or starting: the account's name, its state word and a `Start` button
  (insensitive while starting or queued) where the game would be, mirroring
  the desktop's absent-game panel. Active workspace holds no accounts: the
  line `No games in this workspace` where the game would be, and the list
  still opens. Connecting or reconnecting: the last picture stays dimmed under
  a one-line `Reconnecting…` label; taps are not sent until the connection
  proves itself again. Refused: after un-enrolment, or when the stored
  credential no longer matches, one line, `This phone is no longer enrolled`,
  and no controls.
- **Pattern** — The state words are the desktop sidebar's own liveness
  vocabulary and nothing else (design rule 1, `session_sidebar/row.rs`
  `status_key`); the row button is the one inverting control of design rule
  2, insensitive while starting or queued; the absent-game panel is design
  rule 4's plain centred panel: name, one state line, one button; workspace
  headings carry no dot and no mark of which workspace is shown (design rule
  13).
- **New pattern** — the phone page itself: a full-screen picture with a
  top-left handle that slides an account list over it. It is HTML the
  application serves, outside GTK; `docs/design.md` owes a short section on
  what of its rules carry to it once this ships.

## Technical details

- **Architecture** — `crates/idle-manager-remote/assets/phone.html` replaces
  the placeholder: one file, inline CSS and JavaScript, embedded with
  `include_str!` and served by `GET /enrol/{code}` (with the two `<meta>`
  tags substituted in) and `GET /` (without); on load it moves the meta values
  into `localStorage` and strips them from the DOM (naming rule 1).
- **Architecture** — connection: opens `ws://<host>/ws`, answers `hello` with
  `auth { proof: hmac(secret, "phone|" + challenge), challenge: <32 random bytes hex> }`
  using an inline pure-JavaScript SHA-256/HMAC (no `crypto.subtle` on a
  plain-HTTP origin), verifies `welcome.proof` against `"desktop|" + own
  challenge` and shows nothing until it matches; reconnects with a 1 s, 2 s,
  5 s backoff and shows the `Reconnecting…` state meanwhile; a `bye` or a
  socket refused after a proof failure shows the refused state.
- **Architecture** — presence: sends `attach { viewport: { width: innerWidth, height: innerHeight } }`
  when it is visible and an account is chosen (and mobile mode is on), `leave`
  on `visibilitychange` to hidden and on `pagehide`, `ping` every 5 s; opening
  the list keeps the attach while an account is current, and after a
  reconnect it re-attaches on the same account.
- **Architecture** — rendering: reads each binary message's two big-endian
  `u32`s, decodes the JPEG with `createImageBitmap`, draws it on a full-screen
  `<canvas>` scaled to fit with the aspect kept, and maps `pointer` events
  back through that scale into viewport pixels; a press released within 300 ms
  and 10 px is `tap {x, y}`, a longer move is `scroll {x, y, dx, dy}` sent per
  move with the deltas; nothing is drawn locally; `requestFullscreen` on the
  first tap where allowed.
- **Architecture** — screens from `state`: toggle screen while
  `mobileMode` is false; game screen otherwise, with the absent-game panel
  when `current`'s liveness is not `live` (button sends `start {account}`,
  insensitive for `starting`/`queued`) and the empty line when the shown
  workspace has no accounts; the list panel renders `workspaces` as headings
  and rows with the word and the `Park`/`Start` button (sends `park`/`start`),
  the current row highlighted, and a `Mobile mode` switch sending
  `mobile {on}`.
- **Architecture** — head tags: `<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">`,
  `<meta name="apple-mobile-web-app-capable" content="yes">`,
  `<meta name="mobile-web-app-capable" content="yes">`, a `<title>` of
  `Idle Manager`, so "Add to Home Screen" opens standalone on both systems.
- **Code standards** — the page's logic that can be tested without a browser
  (message parsing, the tap/scroll classifier, the coordinate mapping) is
  written as pure functions at the top of the script; a Rust unit test asserts
  the served page contains the two meta placeholders and the `/ws` path, so a
  rename cannot silently break the client (rule 25).

## Acceptance criteria

- [ ] `(unit)` the page served by `GET /` contains no secret and the page
      served by `GET /enrol/{code}` contains both meta tags with the minted
      values
- [ ] `(unit)` the inline SHA-256 produces
      `ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad` for
      `abc` and the HMAC produces RFC 4231 test case 2's value (checked by
      running the extracted function under `node` in the test, skipped when
      `node` is absent)
- [ ] `(manual)` on an Android phone, opening the enrolment address shows the
      toggle screen, and "Add to Home Screen" then opens the page standalone
- [ ] `(manual)` on an iPhone, the same two steps hold
- [ ] `(unit)` the tap/scroll classifier and the coordinate mapping, run
      under `node` like the hash test: a press released within 300 ms and
      10 px yields `tap` at the canvas point divided by the draw scale, and a
      longer move yields `scroll` with the moved deltas
- [ ] `(manual)` with mobile mode on, the game fills the phone screen with the
      aspect kept, a tap on a game button takes effect on each of the three
      games in `docs/memory-budget.md` (the `isTrusted` check), and a drag
      scrolls the page
- [ ] `(manual)` with the desktop switched to a workspace holding no accounts,
      the page shows `No games in this workspace` where the game would be and
      the handle still opens the list
- [ ] `(manual)` the handle opens the list grouped by workspace with the
      right words; tapping another account switches the game; `Park` turns
      the row's word to `parked` and the game screen to the name, word and
      `Start` panel
- [ ] `(manual)` switching to another app, then back, shows `Reconnecting…`
      briefly if the socket dropped and resumes on the same account
- [ ] `(manual)` `Un-enrol the phone` on the desktop turns the page into the
      single line `This phone is no longer enrolled`

## References

- [Roadmap item](../../roadmap/13-phone-operation/README.md) — Back-end "The
  client page"; the "Watching and tapping a game" and "Switching account from
  the phone" diagrams; Technical References: `crypto.subtle` is unavailable
  on a plain-HTTP origin so the proof needs a script-level SHA-256, while
  WebSocket, canvas, pointer events, Page Visibility and fullscreen all work
  there; script-dispatched events carry `isTrusted = false`, so the three
  budgeted games are the acceptance test
- [`openapi.json`](../../roadmap/13-phone-operation/openapi.json) — every
  `Server*` and `Phone*` schema and `BinaryFrame`
- [Sequence diagrams](../../roadmap/13-phone-operation/sequence-diagrams.md)
  — `GET /ws`
- [Wireframes](../../roadmap/13-phone-operation/wireframes/) —
  `phone-toggle-screen.md`, `phone-game-screen.md`, `phone-account-list.md`
- [`docs/requirements.md`](../../requirements.md) — Remote Access `FR.1.1`–
  `FR.1.5`, `FR.2.1`–`FR.2.3`, `FR.3.3`, `FR.4.4`, `FR.5.2`
- [`docs/architecture.md`](../../architecture.md) — rule 8
- [`docs/code-standards.md`](../../code-standards.md) — rule 25
- [`docs/naming.md`](../../naming.md) — rule 1
- [`docs/design.md`](../../design.md) — rules 1, 2, 3, 4, 13

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The `(manual)` steps need the
desktop wiring slice to be merged as well, and a phone on the same mesh network.
