# 06 — The isolated web view per account

**Roadmap:** [01](../../roadmap/01-isolated-accounts-and-layouts/README.md) · **Scope:** front-end · **Depends on:** 03, 05

## Context

This slice puts the actual game in the slot. Until now an account is a name on a
plain background; after this it is the game's own web page, loaded from the
address that was typed, filling its place in the window.

The whole point of the application is that two accounts of the same game do not
collide, and this is the slice that delivers it. Each account gets its own
browsing engine session, built with the two private folders the earlier slice
created. Every cookie, every bit of storage the game saves progress in, every
offline database and cached file goes into that account's folder and nowhere
else. Two accounts can be logged into the same game at the same time and neither
can see the other's data. The application itself never handles a password: there
is no login screen, nothing is stored and nothing is sent anywhere. Logging in
happens on the game's own page exactly as it would in a browser, and the only
thing kept afterwards is what the game left behind.

The order in which the pieces are built is forced by the engine and is the main
thing to get right. The private session must be built first, because it takes
both folders when it is created and cannot be told about them afterwards. Then
its cookie store is switched to writing on disk rather than in memory — without
that step the folders exist but the login evaporates when the program closes.
Only then is the page view built and attached to that session, because the
attachment can only be made at the moment of creation and can never be changed.
That last constraint looks like a detail today and decides the shape of a later
roadmap item entirely.

One more thing was expected to be not optional and turned out to be actively
harmful. The plan was for every account to claim to be a current version of a
common desktop browser, because the engine introduces itself under a name few
game sites recognise. Measured against a real login, that claim is what breaks
the login: the engine provides none of the fixtures the browser it names would,
and the sites that care compare the two. So no user agent is set, and a game
that genuinely needs a different one gets it from its own preset in roadmap item
06, tested against that game. `FR.10.5` records the reversal.

Two more things this slice owns, because both are per-view and neither has a
widget of its own. A window the page asks for — the "continue with Google"
button every hosted sign-in uses — has to open as a real window sharing the
account's session, or the sign-in never returns. And the diagnostics: until
roadmap item 08 turns failures into UI, a `tracing` trail is the only way anyone
sees why a page loaded but would not log in.

## User experience

- **Flow** — confirming the add-game dialog loads the game's own page in the
  account's slot; log in there the same way you would in a browser. Nothing
  about the login passes through the application.
- **States** — **loading**: the slot shows the account's name centred until the
  page paints, then the page covers it.
- **States** — **bad address**: whatever the engine renders for an address it
  cannot reach; nothing catches it in this item, and a later roadmap item adds
  the handling.

## Technical details

- **Architecture** — `web_view.rs` owns one account's engine objects in a forced
  order: build a `webkit6::NetworkSession` with the data and cache directories
  from the `ProfileLocator` port, call `set_persistent_storage` on its cookie
  manager with a path inside the data directory and the SQLite storage kind,
  then build the `WebView` through its builder with `network_session` set,
  because that property is construct-only.
- **Code standards** — rule 18: `set_user_agent` is not called, and a comment
  standing where the constant was carries the measurement that removed it; the
  construction order carries a comment naming the construct-only constraint that
  forces it.
- **Architecture** — the `create` signal opens a second `WebView` on the
  construct-only `related-view` property, so the popup shares the account's
  network session and web process and the cookie it is granted lands in that
  account's profile; it is parented into a transient `gtk::Window` on
  `ready-to-show`. No gesture check sits in front of it: with
  `javascript-can-open-windows-automatically` at its default the engine has
  already refused every ungestured `window.open` before the signal is emitted.
- **Code standards** — rule 15: the load lifecycle, subresource loads and their
  failures, script dialogs, permission requests, TLS failures and web-process
  death go to `tracing` in fields. The page's own console arrives the same way
  through a document-start user script injected into every frame and a
  `UserContentManager` message handler, which is what attaches an origin to each
  line; `console.error` maps to `warn`, `console.warn` to `debug` and the rest to
  `trace`, because one bot-check frame logs hundreds of lines per load. `WebKit`'s
  console-to-stdout stream stays on in debug builds beside it for the engine's
  own messages, which no `console` override can see.
- **Architecture** — rule 3: the shell receives the locator as the port, never
  the store crate; `crates/idle-manager/src/main.rs` stays the only place that
  knows the locator is XDG-backed.
- **Architecture** — rule 10: view creation and every settings call run on the
  GTK main context.
- **Architecture** — one web process per view and one shared network process is
  the engine's own default in this API version, so nothing configures it; the
  job here is to measure it during the run and record the figures in the item's
  `test-script.md`.
- **Architecture** — rule 14 and code standards rule 25: this slice is covered
  by the item's `test-script.md`; no test may require a display server.
- **Design** — the game's page carries no browser chrome, so the shell owns the
  one control a stuck login needs: a `view-refresh-symbolic` button on the header
  bar's leading edge and an `F5` / `Ctrl`+`R` `EventControllerKey` on the window
  in the capture phase, both calling `SessionGrid::reload_focused`, which reloads
  the view in the focused slot and no other. Capture phase because a game that
  binds those keys on its own canvas would otherwise consume them first. This is
  the whole of the reload story until item 08; the design doc owes a rule on
  where a window-level game action lives once more than one exists.

## Acceptance criteria

- [x] `(manual)` confirming the dialog shows the account's name until the page
      paints, then the game's own page fills the slot
- [x] `(manual)` two accounts of the same game, added with the same address, are
      logged into different game accounts at the same time and neither logs the
      other out
- [x] `(manual)` the header-bar reload button — and `F5` / `Ctrl`+`R` — reloads
      the page in the focused slot and leaves the other slots untouched; the key
      press reaches the window even when the focused game binds it on its canvas
- [x] `(manual)` after logging in, that account's data directory under the XDG
      data root holds a non-empty cookie database file, and the other account's
      directory holds a different one
- [x] `(manual)` a page that reports the browser it sees — or
      `navigator.userAgent` in the web inspector — returns the engine's own
      string, unmodified, because nothing sets a user agent
- [x] `(manual)` clicking a hosted "sign in with Google" button opens a separate
      top-level window, transient over the main window, showing Google's own
      sign-in page for that site
- [x] `(manual)` a `window.open` that no click asked for opens nothing and emits
      no popup trace, because the engine refuses it before the signal is emitted
- [x] `(manual)` with `RUST_LOG=idle_manager_shell=debug`, a page's
      `console.error` appears as a `page console error` event carrying the origin
      of the frame that logged it, including a cross-origin one
- [x] `(manual)` with three accounts loaded, `ps` shows three web processes and
      exactly one network process; the counts are written into the item's
      `test-script.md` because the roadmap item lists this as unverified
- [x] `(manual)` an address that cannot be reached renders the engine's own
      error inside its slot and leaves the other slots loaded and running
- [x] `(manual)` switching the arrangement while pages are loaded moves them
      without reloading — a running game keeps its state and an out-of-sight
      account is still ticking when it returns

## References

- [Roadmap item](../../roadmap/01-isolated-accounts-and-layouts/README.md) — the full picture, including the Technical References section pinning each engine call to its version and source line
- [Wireframes](../../roadmap/01-isolated-accounts-and-layouts/wireframes/) — `main-window.md` describes what a slot shows before its page paints
- [`docs/architecture.md`](../../architecture.md) — rules 3, 10, 12, 13 and 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 5, 18 and 25
- [`docs/naming.md`](../../naming.md) — rules 2 and 11
- [`docs/design.md`](../../design.md) — no numbered rules yet

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
