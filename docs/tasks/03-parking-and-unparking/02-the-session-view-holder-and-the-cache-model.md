# 02 — The session view holder and the cache model

**Roadmap:** [03](../../roadmap/03-parking-and-unparking/README.md) · **Scope:** front-end · **Depends on:** —

## Context

The program runs each game account in its own browser view. Today the code that
builds one does it in a single step: it creates the account's private storage
area — the folder holding its cookies and its login — and the view together,
hands the view back, and forgets the storage area. That works for as long as a
view lives exactly as long as the account does. It stops working the moment an
account can be shut down and started again, which is what the following slices
add.

The browser engine imposes one rule that shapes everything here: a view is tied
to its storage area at the instant it is created, and can never be re-tied
afterwards. So starting an account back up cannot revive the view that was
thrown away; it has to build a brand new one against the storage area that was
kept alive the whole time. The storage area is the durable thing and the view is
disposable.

This slice restructures the code around that. In place of a function that
returns a view there is a small holder object per account, owning the storage
area and the account's settings permanently and the view only for as long as the
account is running. Nothing yet asks it to stop or start — that is the next
slice — but everything after this can.

It also sets one engine-wide option. The engine keeps a cache of recently used
rendering processes, so a page opened again starts faster. That cache is exactly
wrong here: it would mean a shut-down account hands its memory to the engine
rather than back to the system, and the whole feature would be a claim rather
than a fact. So the application asks the engine for its lowest cache setting at
start-up, trading slower page loads for memory that really comes back. How much
slower is not known, and measuring it is part of accepting this slice.

Nothing a user can see changes. The evidence that it worked is that everything
still works: games load, logins survive a restart, sign-in windows still open.

## User experience

- **States** — nothing visible changes. Every state the sidebar draws —
  `Current`, `Visible`, `Background`, and the empty list — renders exactly as it
  did before this slice.
- **Flow** — the observable evidence is negative and is all in the runbook: a
  game still loads into its slot, a login still survives quitting and restarting
  the application, and a sign-in popup still opens into the account it belongs
  to.

## Technical details

- **Front-end** — restructure `crates/idle-manager-shell/src/web_view.rs` from
  the free function `build(data_dir, cache_dir, start_address) -> WebView` into
  a type owning the `webkit6::NetworkSession`, the account's settings and an
  `Option<WebView>`. `NetworkSession` is a construct-only property of `WebView`
  (`webkit6` 0.6.1, `src/auto/web_view.rs:141`), which is why the holder must
  outlive the view it holds. Naming rule 9 — it is `SessionView`, named for what
  it owns, never `SessionViewManager`; naming rule 2 keeps the module file
  `web_view.rs`.
- **Front-end** — the holder gets a stop and a start. Stop is two calls in a
  fixed order — `terminate_web_process()` on the view, then drop it — leaving
  the option `None`. That call lives on the view, not the context (`webkit6`
  0.6.1, `src/auto/web_view.rs:1590`), so termination is inherently per-session
  and needs no process bookkeeping of our own. Start fills the option again from
  the same `NetworkSession`, applies the settings through the existing
  `configure`, then loads the start address. Neither is called by anything yet;
  tasks 03 and 04 are the callers.
- **Code standards** — rule 18: comment the termination order with the
  constraint, not the calls. Dropping the view first leaves the engine to decide
  when the process dies, which is the difference between memory returned and
  memory merely promised (`FR.5.2`).
- **Front-end** — `WebContext::set_cache_model(CacheModel::DocumentViewer)`
  (`webkit6` 0.6.1, `src/auto/web_context.rs:207`, `src/auto/enums.rs:350`) once
  at shell start-up, beside the `GResource` registration in the shell's
  `lib.rs`. `DocumentViewer` is the lowest of the three cache models and the one
  WebKit associates with keeping no cached processes; the other two keep some.
  It is a global on the shared web context, not a per-session setting, so it
  does not belong in the holder (`FR.5.3`).
- **Front-end** — the existing `connect_web_process_terminated` handler in
  `wire_diagnostics` branches on `WebProcessTerminationReason`, which has three
  variants (`webkit6` 0.6.1, `src/auto/enums.rs:4677`): `TerminatedByApi` is a
  deliberate park, logged and otherwise ignored; `Crashed` and
  `ExceededMemoryLimit` stay an error trace. Telling the third from the first
  two is what keeps item 08's automatic reload from fighting a deliberate park,
  so the branch is written here and item 08 inherits it rather than discovering
  it.
- **Front-end** — `window/imp.rs` keeps one holder per session instead of a bare
  view: `create_account` builds the holder and hands the grid the view it
  produced. Architecture rules 10 and 12 are unchanged by the restructure.
- **Testing** — architecture rule 14 and code standards rule 25: no test in this
  repository may require a display server, so this slice's evidence is the
  item's `test-script.md` and its criteria are `(manual)`.
- **Front-end** — the roadmap item's third blocker is settled on this slice's
  runbook: record the first-load time of one game before and after the
  cache-model change, so the cost of the lowest cache setting is a number rather
  than an unknown.

## Acceptance criteria

- [ ] `(manual)` the application starts and every account loads its game as
      before, with exactly one `WebKitWebProcess` per account in `ps`
- [ ] `(manual)` nothing the sidebar draws changes: `Current`, `Visible` and
      `Background` rows and the empty-list line render exactly as they did
      before the restructure
- [ ] `(manual)` a login made in an account survives quitting and restarting the
      application — the holder keeps the same data directory and cookie database
- [ ] `(manual)` a sign-in popup still opens from a game's login button and
      shares the opening account's session
- [ ] `(manual)` with `CacheModel::DocumentViewer` applied, a game's first load
      still completes, and the time it takes is recorded in `test-script.md`
      beside the same load before the change
- [ ] `(manual)` `kill -9` on one account's `WebKitWebProcess` is logged by the
      terminated handler as `Crashed`, distinct from a park's
      `TerminatedByApi`, and no reload is attempted

## References

- [Roadmap item](../../roadmap/03-parking-and-unparking/README.md) — the full
  picture, including the Technical References for `webkit6`
- [`docs/requirements.md`](../../requirements.md) — `FR.2.3`, `FR.5.2`,
  `FR.5.3`, `FR.5.4`
- [`docs/architecture.md`](../../architecture.md) — rules 10, 12, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 18, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 9
- [`docs/design.md`](../../design.md) — rule 1; nothing this slice draws changes,
  so it only has to stay true

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
