# Goal: Record the user needs and functional requirements for operating a running account from a phone

**Status:** executed 2026-09-19 — `Remote Access` `UN.1`–`UN.6`, `FR.1.1`–`FR.6.3` plus `FR.3.5` appended to `docs/requirements.md`; no ditched record for the on-device port
**Rating:** —
**Run:** standalone. 06 and 07 are both executed; 07 added no rows to
`docs/requirements.md`, so nothing is contended and nothing is waiting.

## Context

The application runs on a desktop machine that stays on. The user is away from
that machine and needs to act on an account roughly every half hour. Today that
means a general-purpose remote desktop, and the complaint is specific: it works,
but it presents a **desktop-shaped, multi-account grid on a six-inch screen**,
so every interaction starts with pinch-zoom-panning to find the thing to click.

Establish early in the session what is and is not being asked for, because a
previous round of analysis closed one of these and left the other open:

- **Running the application *on* the phone is ruled out**, and not for effort
  reasons. On iOS a `WKWebView`'s content process is suspended when the app
  leaves the foreground, and no background mode covers this — it cannot work.
  On Android, Chromium's renderer-level background throttling (1 Hz, then
  roughly one wake per minute after five minutes hidden) has no public API to
  disable, so the one mechanism this application exists to defeat is
  unreachable; and item 05's measured per-renderer figures of 500–800 MiB put
  even a single account past what a foreground-service app survives. The
  session should decide whether this deserves a `docs/ditched/` record so it is
  not re-proposed, per CLAUDE.md.
- **Reaching a machine that is already running the application is the live
  idea**, and that is what these requirements are for.

Technical ground truth, as background for the grill and **not** as material for
the requirement rows — requirements state what, never how:

- Page-to-shell messaging already exists (`web_engine/ipc_message.rs`,
  `resources/js/webview2-bridge.js`). The other direction does not:
  `EngineView` exposes `load_uri`, `reload`, `set_zoom`, `terminate`,
  `set_keep_awake`, but nothing that evaluates script in a live page. Both
  engines support it, and item 12's seam means adding it is small.
- The machine's resting state is settled. `docs/ditched/02-sleep-mode.md`
  (2026-09-19) records that minimising the window with keep-awake on already
  stops the application's GPU work while the games keep ticking, so no separate
  mode was needed. That is the state a phone would be connecting to, which is
  what question 9 below turns on.
- `Layout::Single` already exists, so "one account filling the window" is a
  state the application can already be in. A free experiment — Single layout,
  window resized to phone aspect, reached over the existing remote desktop —
  tests whether the *framing* is the whole win before any client is built.
  That experiment's outcome should select between shapes at roadmap time; the
  requirements must be written so it can, rather than baking one shape in now.
- Architecture rules 9 and 10 constrain any implementation — the core is
  synchronous with no threads or clock, and every widget call happens on the
  GTK main context — so anything network-facing is a new *driving* adapter, the
  first besides the shell, wired only in the binary (rule 3) with a new
  forbidden edge in `scripts/arch-check.sh`.

**Open questions this session must close** — grill each, assume none:

1. **What is the need, precisely?** "Operate an account from a phone" or "build
   our own phone client"? If a zero-code arrangement (Single layout over the
   existing remote desktop) satisfies the need, the requirements must be able to
   say so — that changes the eventual roadmap item enormously.
2. **What must be doable from the phone?** Enumerate it: see the page, click
   anywhere on it, switch accounts, park and unpark, read the memory footer,
   enter and leave sleep mode, start an account that died. Each is a separate
   requirement or a deliberate exclusion.
3. **How is an account chosen and switched?** One at a time full-screen, or
   some overview? How many accounts does this have to scale to?
4. **What refresh and latency is acceptable?** A still image every few seconds
   and a live stream are different features with different costs. Pin the
   tolerance, because it decides most of the rest.
5. **What reach is required?** Same network only, or from anywhere? Note that a
   mesh VPN collapses these into one case, so the requirement should state the
   reachability the user needs and leave the mechanism to the roadmap item.
6. **Security, as a requirement rather than an afterthought.** Whatever this is,
   it can act on pages that are logged into the user's game accounts. Who is
   allowed to reach it, what authenticates them, what happens on a hostile or
   shared network, and what the thing refuses to do even for an authenticated
   caller.
7. **Does this have to work on Windows too?** `Platform Support` `FR.1.2`
   commits every shipped Session Management feature to working on Windows. A
   new feature inherits that expectation unless the session says otherwise.
8. **Which module does this belong to?** The file currently namespaces codes per
   module — `Session Management` and `Platform Support` each run their own
   `UN.`/`FR.` sequences. Remote access may warrant a third; decide rather than
   defaulting.
9. **What the phone sees when the window is minimised, which is the sharp
   one.** The resting state of the machine is now settled: minimised, with
   keep-awake accounts running at full speed. But minimising unmaps every view
   (`FR.3.3`), and the shell deliberately marks each account *without*
   keep-awake as background to its engine on minimise (`background_for` in
   `web_view.rs`, applied by `apply_minimised` in `window/imp.rs`). So the
   phone would be looking at accounts the application has intentionally put to
   sleep, through views the engine considers hidden. Establish what the user
   expects to see and to be able to click in that state — and whether being
   looked at from a phone should change an account's backgrounding at all,
   which would override the same per-account choice `docs/ditched/02-sleep-mode.md`
   declined to override.

## Constraints

1. **Run `/msg-pre-roadmap`.** Requirements only — no roadmap item, no
   exploration document, no task breakdown, no code. `/msg-roadmap-plan-item`
   is a separate, later act.
2. **`docs/requirements.md` is append-only.** Add rows; never edit, reorder or
   delete an existing one. Match the existing column shape and date the rows.
3. **Continue the right code sequence.** Codes are namespaced per module —
   `Session Management` currently reaches `UN.21` and `FR.21.12`, while
   `Platform Support` runs its own `UN.1`/`FR.1.x`. Open question 8 decides
   which sequence these join.
4. **State what, never how.** No HTTP, no screenshots, no port numbers, no
   `document.elementFromPoint`, no named VPN product in a requirement row. Read
   `FR.6.2` and `FR.6.3` for the register: they name the behaviour and the
   reason without naming a call.
5. **Do not bake in a mechanism the experiment has not selected.** Question 1
   exists because the cheapest shape may satisfy the need; a requirement that
   presumes a bespoke client forecloses that.
6. **Security gets its own requirements.** At minimum: what authenticates a
   caller, and the boundary of what an authenticated caller may cause to happen.
   Do not leave this to the roadmap item.
7. **Invent nothing.** Every row traces to something the user actually said or
   agreed to in the grill. An unanswered question above is a question to ask,
   not a gap to fill with a sensible default.
8. **Read `docs/ditched/02-sleep-mode.md` first.** Prompt 07 ran on
   2026-09-19 and produced no requirement rows: a sleep mode was ditched
   because minimising with keep-awake on already is that state. Its reasoning
   is the ground under question 9, and its closing line — that a future
   measurement showing a minimised window still costs presentation time comes
   back as a measured finding, not as a mode — binds here too. Do not
   re-propose a mode by another name.
9. **No branch needed.** Documentation and planning edits only, per CLAUDE.md.

## Tone

Match the house register of `docs/requirements.md`: one need or one requirement
per row, plain declarative sentences, the *why* included wherever a reader a
year from now would otherwise ask it. No hedging, no marketing.

## Output

New rows appended to `docs/requirements.md` — user needs under the next free
`UN.` code in the chosen module and their functional requirements under the
matching `FR.` codes — plus a short statement of which of the nine questions
above the grill closed, how, and whether the ruled-out on-device port got a
`docs/ditched/` record.
