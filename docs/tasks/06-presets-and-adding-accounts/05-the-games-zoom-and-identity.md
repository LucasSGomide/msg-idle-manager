# 05 — A view drawn at the game's zoom and browser identity

**Roadmap:** [06](../../roadmap/06-presets-and-adding-accounts/README.md) · **Scope:** front-end · **Depends on:** 03, 04

## Context

Several of these games are built for a full screen and are unreadable in a
quarter of one. The browser engine can draw a page larger or smaller, one page
at a time, and this slice makes that size come from the game's file: an account
created from a game is drawn at the size that game asks for, applied before the
page starts loading, so it is never drawn at the wrong size and then jumped to
the right one.

The second value is different in kind, though it travels the same way. Some
games refuse to load, or refuse to sign a user in, for a browser they do not
recognise, and a program can tell a website which browser it is. The obvious
move — claim to be a popular browser for every game — was tried and measured,
and it made things worse: the engine cannot back the claim up, so the game's
sign-in and its bot check both reject the mismatch outright, which is a harder
failure than being an unrecognised browser. So this is an override rather than a
default. A game whose file says nothing about it gets the engine's own identity,
which is what works for every game tried so far. A game that genuinely turns the
engine away gets a string of its own, put there by somebody who tried it against
that game.

Both values belong to the account, not to the page. An account can be stopped to
hand its memory back and started again later, and starting it builds a brand-new
page from scratch — so the size and the identity are remembered on the account
and re-applied to every page it is given, or a stopped-and-started game would
come back drawn wrong.

One consequence is worth stating: because the values are copied onto the account
when it is created, editing a game's file changes nothing for accounts that
already exist. The edit takes effect for the next account created from that
game.

## User experience

- **Flow** — the created account carries the game's browser identity and its zoom
  from the moment it first loads, not after a visible correction.
- **Flow** — the background-speed setting the game supplied is only a starting
  value: it can still be changed per account afterwards, exactly as item 04
  describes, and from then on the account owns it and the file has no further
  say.
- **States** — **a game with no identity of its own**: the account presents the
  engine's own identity, exactly as every account does today. **A game with an
  identity in its file**: the account presents that string and nothing appended
  to it. **An account created through "Something else"**: the engine's ordinary
  size and the engine's own identity.

## Technical details

- **Front-end** — `SessionView::new` in `web_view.rs` gains the zoom multiplier
  and the optional browser identity. Both are remembered on the holder beside
  the keep-awake flag and re-applied by `SessionView::start` to every view it
  builds, so an account parked and started again comes back at the same size
  with the same identity — the same reason the keep-awake flag is remembered
  there rather than only on the live view (code standards rule 18).
- **Front-end** — `WebViewExt::set_zoom_level` on the view itself (`webkit6`
  0.6.1, `src/auto/web_view.rs:1577`) and `Settings::set_user_agent` on the
  view's settings object (`src/auto/settings.rs:1322`), both applied before
  `load_uri`. Both are per view rather than per context, which is what makes
  them per-account with no shared state involved.
- **Front-end** — call `set_user_agent` **only when the account carries an
  identity**; when it carries none, call neither setter and leave the engine's
  own alone. Not `set_user_agent_with_application_details`
  (`src/auto/settings.rs:1329`): it appends to the engine's own identity rather
  than replacing it, which makes it the safer of the two calls but not what an
  override is for — a game that turns the engine away turns away an engine with
  a suffix too.
- **Front-end** — the comment in `configure` recording item 01's measurement
  stays exactly where it is and gains a pointer to where the override now lives
  (code standards rule 18). It is the only record of why there is no default
  string.
- **Front-end** — `window/imp.rs` reads the zoom and the identity off the
  `Session` the book returned, never off a preset it kept a handle on. That is
  what makes an account independent of the file it came from, and it is the
  difference between "the next account created from this game uses the edit" and
  "watch the folder and restart affected accounts", which the item explicitly
  does not do.
- **Front-end** — the popup a sign-in opens is built from the opener with
  `related-view` and inherits its settings object, so the identity an override
  sets reaches the login window as well; check that rather than assuming it.
- **Testing** — architecture rule 14 and code standards rule 25 keep GTK out of
  `cargo test`, so this slice's evidence is the item's `test-script.md`. Reading
  the identity back needs a page that echoes it; name that page in the test
  script's `## Setup` so every later check can reuse the step. Task 03's three
  shipped games have landed by the time this slice runs, so the item's own
  coverage sentence — add an account from a shipped entry and confirm the game
  loads at its intended zoom and is not turned away as an unknown browser — is
  checked here against the real games, not against a hand-written file.

## Acceptance criteria

- [x] `(manual)` an account created from a game whose file asks for a zoom of
      `0.8` draws its page smaller than one asking for `1.0`, and does so on the
      first paint rather than after a visible resize
- [x] `(manual)` an account created through "Something else" draws at the
      engine's ordinary size and reports the engine's own identity
- [ ] `(manual)` an account created from each of the three shipped games loads
      that game at the zoom its file asks for and reaches the game's sign-in
      without being turned away as an unrecognised browser
- [x] `(manual)` an account created from a game whose file names no browser
      identity reports the engine's own identity on a page that echoes it
- [x] `(manual)` an account created from a game whose file names a browser
      identity reports exactly that string, with nothing appended
- [x] `(manual)` parking such an account and starting it again brings back the
      same size and the same identity on its fresh page
- [ ] `(manual)` a sign-in popup opened by an account with an identity override
      reports the same string as the account that opened it
- [x] `(manual)` editing a game's file leaves an account already created from it
      drawing exactly as before, and the next account created from that game uses
      the edited values
- [x] `(manual)` setting one account's zoom and identity leaves every other
      account's page loaded, running and untouched

## References

- [Roadmap item](../../roadmap/06-presets-and-adding-accounts/README.md) — the
  full picture, including the Technical References on the two `webkit6` setters
  and why the appending one is not the right call for an override
- [Wireframes](../../roadmap/06-presets-and-adding-accounts/wireframes/) — the
  dialog this slice's accounts are created from
- [`docs/requirements.md`](../../requirements.md) — `FR.10.1`, `FR.10.5`,
  `FR.5.4`, `FR.6.1`
- [`docs/architecture.md`](../../architecture.md) — rules 3, 8, 10, 12, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 6, 13, 15, 17, 18,
  25
- [`docs/naming.md`](../../naming.md) — rules 2, 11, 12
- [`docs/design.md`](../../design.md) — rules 1 and 3, which the row of an
  account created this way must still read by

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
