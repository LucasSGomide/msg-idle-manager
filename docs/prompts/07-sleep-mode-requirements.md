# Goal: Record the user needs and functional requirements for a sleep mode that keeps games running while the interface stops rendering

**Status:** executed 2026-09-19 — no requirement rows; ditched as `docs/ditched/02-sleep-mode.md`
**Rating:** 7
**Run:** parallel with 06 — this touches `docs/requirements.md` only, 06
touches `crates/idle-manager-metrics/` only.

## Context

The wanted behaviour: leave the application running for hours with its
interface not drawing anything, so the machine spends its cycles on the games
rather than on presenting them, and the user comes back to it occasionally
rather than sitting in front of it.

Start the session by establishing how much of this already ships, because the
answer is "most of the mechanism, none of the naming":

- **Keep-awake already defeats the throttling.** `UN.6` / `FR.6.1`–`FR.6.4`
  give each session a flag that disables hidden-page timer throttling and CSS
  animation suspension, and injects the document-start script that routes
  `requestAnimationFrame` through a timer while the page reports hidden.
- **Item 04 measured where that flag actually matters.** Its `## Measured`
  section records the finding that an off-grid account is never marked hidden
  and needs no protection, and that **a minimised window is the one case where
  keep-awake does anything at all.** So "minimise, with keep-awake on" is
  already close to the wanted behaviour.
- **`FR.3.3` is the constraint the whole idea runs into.** Off-grid live views
  stay mapped at out-of-bounds coordinates inside a clipping container
  precisely because *an unmapped view is treated as hidden by WebKit*
  (`GtkStack` is excluded for unmapping its children). Anything that stops the
  interface rendering by unmapping or hiding it inherits that, and therefore
  depends entirely on keep-awake to keep the games ticking.
- **Parking is the mechanism that returns memory**, and it does so by killing
  the view — which stops the game. So sleep mode cannot both keep a game
  running and hand its memory back; those are different features.

What is genuinely new is the *mode*: one named state, entered deliberately,
that puts the whole application into that configuration and back out again.
The requirements work is mostly about closing the questions below, not about
discovering a mechanism.

**Open questions this session must close** — grill each one, do not assume an
answer:

1. **Entry.** Manual (a button, a menu item, a hotkey) or automatic (a timer,
   an idle threshold, closing the window)? Or several?
2. **Exit, and this is the sharp one.** If sleep mode hides the toplevel
   rather than minimising it, how does the user get back? GTK 4 has no tray
   API of its own; a `StatusNotifierItem` means a new dependency, and a
   relaunch-to-restore only works if the application already handles being
   single-instance. Establish whether hiding buys enough over plain minimising
   to be worth any of that.
3. **Scope.** The whole application, or per-account, or per-workspace?
4. **The tension with `FR.6.1`.** Sleep mode is useless unless the accounts in
   it are kept awake — but `FR.6.1` deliberately takes each account's flag from
   its preset, because forcing every game to full speed spends memory and
   processor time protecting games that were never at risk. Does entering sleep
   mode override that per-account default, respect it, or prompt? A requirement
   that quietly overrides `FR.6.1` needs to say so and say why.
5. **What is actually saved.** The game loop is the cost and cannot be reduced
   without reducing progress; what sleep mode can drop is GTK's draw/composite
   pass and WebKit's paint and GPU work. Decide whether the requirements name a
   target or simply demand that the saving be measured.
6. **Relationship to parking.** Are sleep and parking orthogonal axes over
   `Visibility` and `Liveness`, or does one imply anything about the other?
7. **Whether it stands alone.** A separate idea — reaching a running instance
   from a phone — is under discussion and unrecorded. Sleep mode appears to
   stand on its own; confirm that, so this does not quietly become the first
   slice of a much larger feature.

## Constraints

1. **Run `/msg-pre-roadmap`.** This is the requirements step and stops there —
   no roadmap item (`/msg-roadmap-plan-item` is a separate, later act), no task
   breakdown, no code.
2. **`docs/requirements.md` is append-only.** Add rows; never edit, reorder or
   delete an existing one.
3. **Continue the existing code sequence within the right module.** Codes are
   namespaced per module — `Session Management` currently reaches `UN.21` and
   `FR.21.12`, while `Platform Support` runs its own `UN.1`/`FR.1.x`. Sleep
   mode belongs to `Session Management` unless the session argues otherwise.
   Match the existing column shape and date the rows.
4. **Cite rather than restate.** Where a need is already recorded — the
   keep-awake family especially — reference the existing code instead of
   duplicating it, and use `Refines <code>` / `Supersedes <code>` the way the
   later rows in the file already do.
5. **State what, never how.** Read `FR.6.2` and `FR.6.3` for the register: they
   name the engine behaviour being defeated and why, without naming a function
   to call. No `gtk_widget_hide` in a requirement.
6. **Invent nothing.** Every row traces to something the user actually said or
   agreed to in the grill. An unanswered question above is a question to ask,
   not a gap to fill with a sensible default.
7. **A claimed saving must be verifiable.** The project's precedent is item
   04's `## Measured` section and item 05's `make memory-report`; a requirement
   asserting sleep mode costs less must be written so a measurement can settle
   it.
8. **Note the measurement caveat.** The memory probe currently fails
   permanently once a zombie descendant appears (prompt 06). If a requirement
   here depends on reading memory to prove itself, that fix is a prerequisite —
   say so rather than writing a requirement that cannot be tested yet.
9. **No branch needed.** Documentation and planning edits only, per CLAUDE.md.

## Tone

Match the house register of `docs/requirements.md`: one need or one
requirement per row, plain declarative sentences, the *why* included wherever
a reader a year from now would otherwise ask it. No hedging, no marketing.

## Output

New rows appended to `docs/requirements.md` — user needs under the next free
`UN.` code and their functional requirements under the matching `FR.` codes —
plus a short statement of which of the seven questions above the grill closed
and how.
