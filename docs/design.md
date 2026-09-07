# Design rules

Rules for anything design-shaped. `project.yml` points every
`**Design**` bullet in a roadmap item at this file.

Each rule is one imperative and one line of why. A rule with no why is a
preference, and the next person will not know whether to keep it.

Numbered, because roadmap items cite them by number — renumbering breaks the
citations, so append rather than reorder.

1. **Mark a sidebar row's state with one coloured dot and nothing else visible;
   carry the state word only as the dot's tooltip and accessible label, and
   derive dot class and hover text from a single status key in
   `session_sidebar/row.rs` and `sidebar.css`.** The marker has to carry several
   facts as the roadmap grows — visibility now, parked in item 03, unresponsive
   in item 08 — so a key-per-state that the dot's CSS class *and* its hover text
   both derive from keeps each new state one arm of one match, never a fourth
   branch in the row factory; dropping the always-visible word hands the
   trailing edge back the width a name was losing to it, and a pointer or a
   screen reader still names the state on demand. The states this repo has so
   far: `current` (holds the focused slot) — green dot with a neon glow, name
   bold; `visible` (on screen, not focused) — green dot, flat; `background`
   (running out of sight) — amber dot, flat, name dimmed to 55% alpha; `parked`
   (not running, whatever place it still holds) — grey dot, flat, name dimmed to
   55% alpha; `starting` (unparked, no page painted yet) — blue dot, flat;
   `queued` (restored and waiting its turn in the start queue, item 07) — purple
   dot, flat, name dimmed to 55% alpha. The glow is `current`'s alone: it marks
   the one row you are looking at, not every row that is busy.

   **The ordering, now that there are six.** A key is chosen by walking a fixed
   list and taking the first that applies, so a new state slots into the list
   rather than adding a branch. Liveness comes before visibility: an account
   that is not simply running says so first, whatever slot it still holds —
   `parked`, then `starting`, then `queued`, each a weaker claim than the last
   (stopped outranks starting-up outranks waiting-for-a-turn). Only a running
   account reaches the visibility keys, and there `current` outranks `visible`
   outranks `background`. The list is the whole vocabulary; the seventh state
   item 08 adds names where it falls in it and needs no new rule.

2. **Give a row's Park/Start action one control whose label and effect invert
   with the row's state — `Park` while it runs, `Start` once it is parked — and
   fold it into the row's ⋯ menu rather than standing a button on the row.** The
   action is always "move this account to the other liveness", so a single
   control that reads the current state can never be triggered in a direction
   that does not apply, where a pair of buttons would always have one half
   disabled. It goes in the menu, not on the row: the trailing edge is scarce
   (rule 6) and Park/Start already shares it with the dot, the keep-awake mark
   and the menu button, while the press itself is one deliberate step into a
   menu the reader opened. The menu item is shown but insensitive while the
   account is `starting`, so the start cannot be triggered twice; it carries no
   state of its own — the window reads liveness for the direction (architecture
   rule 8).

3. **Dim a row's name to 55% alpha whenever the account is out of sight or
   parked, and keep bold for the current row alone.** Item 02 already dimmed the
   out-of-sight name; parking is the same message to the reader — this row is
   not something you are watching or paying for right now — so it takes the same
   style rather than a second faded look the eye has to learn. When both apply
   (parked and current), dimming wins: an account that is not running is not
   "current" in any sense the bold was meant to carry.

4. **Stand in for an absent game with a plain centred panel on the window's
   background — the account's name, one line of state text, one action button —
   never a busy card.** The panel sits among slots that do hold live games and
   must not pull the eye from them; keeping it to three stacked elements with
   generous space also leaves item 08's failure panel — the same widget with
   different words and a different button — nothing to redesign. The state line
   and the button label are widget properties, not markup, so the two panels
   cannot drift apart.

5. **Put both the set-and-forget settings a row carries and its
   invert-with-state Park/Start action behind one ⋯ menu button on the row's
   trailing edge — the action first, the settings below it.** "Keep running when
   hidden" is chosen once per account and left alone; Park/Start is reached
   often but still only ever means one thing at a time (rule 2). Neither earns
   permanent width on a trailing edge already spending it on the dot and the
   keep-awake mark, and a `gtk::MenuButton` opening a `gio::Menu` costs the row
   nothing while closed — so one affordance gives both a home, the rarely-touched
   setting and the deliberate action alike, without a fixture beside the name
   for either.

6. **When a row's trailing edge is asked to carry more than one fact, let the
   account's name give way, never a fact at the edge — and size the sidebar so
   that giving way costs the name only length, never legibility.** The name
   label is the row's only hexpanding child and already ellipsises with
   `pango::EllipsizeMode::End`, so it is the one part built to lose length;
   every fact after it — the status dot, the keep-awake mark, the menu button —
   keeps its full width and its full position, whatever the account
   is called. But the sidebar's `width-request` is not a number fixed once and
   forgotten: it is the trailing edge's total natural width plus enough room
   left over for a short name to still render in full, and it is *derived*
   from that edge, not the other way around. Item 04's keep-awake mark moved
   it from 220 to 240 for exactly this reason — at 220 a four-letter name
   collapsed to a bare ellipsis, which is the edge eating the name rather than
   the name giving way to it. Then dropping the status word and the standalone
   action button (rules 1, 2) shrank the edge and the check ran the other way:
   240 to 150, settled by eye with a three-letter name still rendering in full
   beside the dot. The next item that changes a trailing fact owes the same
   check: settle the width by eye, with a short name on screen, rather than let
   the name silently absorb — or keep paying for — a fact's cost.

7. **Set an escape-hatch option apart from the real options in a chooser with a
   hairline above it and dimmed text, never a separate control.** The add-game
   dialog's list ends with "Something else…" — the way out for a game the
   application has no file for. It has to be reachable from the same list as the
   games, so the dialog is never a dead end even with nothing configured, but it
   must not read as one of them: a full-width `gtk::Separator` above it and the
   `dim-label` style on its text say "past here is not a game" without spending a
   second widget or a second stage on the distinction. The separator shows only
   when there are real options above it — with none, there is nothing to
   separate from.

8. **Report a partial failure inside a form as one dim line beneath the field it
   belongs to, and keep the form working.** A preset file that will not parse
   costs the chooser one row, not its ability to open: the file's name and the
   one-line reason sit in a `dim-label` `gtk::Label` under the list, the rows
   that did parse still show, and the action row is untouched. The same line
   carries the empty case — when nothing parsed it names the folder to put files
   in, and the escape hatch still stands. A failure the user can act on is
   information, not an error dialog: it never takes a modal of its own and never
   blocks the path that still works.
