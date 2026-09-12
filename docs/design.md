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

9. **Say what went wrong before the user did anything in one strip across the
   top of the window — directly under the header bar, spanning the sidebar and
   the grid — carrying one line and a dismiss button on its trailing edge, and
   nothing else.** Two things reach it: a saved workspace that would not load,
   read before the window is built (item 07 task 04), and a save that failed
   mid-session (task 06). Both are the machine's problem, not something the user
   asked for, so they belong above the whole window rather than beside any one
   control, and they read as attention-not-urgent — the theme's warning tint,
   never the error red. **It never leaves on its own.** A problem the user did
   not cause is one they decide when they have dealt with; a strip that faded
   would take the only record of it. It goes when its close button is pressed
   and not before, and a later success (a workspace that now loads, a save that
   now works) leaves a dismissed strip dismissed rather than reopening it. It is
   **one reusable widget** — `message_strip.rs` with `message-strip.ui` — set to
   a message and shown, or cleared, by whatever raises it; the launch failure
   and the mid-session failure must not drift into two shapes.

10. **Acknowledge a direct gesture with a transient figure drawn over the place
    it affected — low in the place, centred, on its own opaque ground — that
    fades on a timer and leaves nothing behind; never draw one for a change the
    user did not directly ask for.** Item 09's zoom gesture needs to be *seen*
    even when it changes nothing visible: a step at the size limit, a step over
    a parked account with no page to resize, a run of steps that would otherwise
    look like one. So each gesture flashes the resulting percentage over that
    place (`session_grid/imp.rs` `build_readout`), about a second
    (`ZOOM_READOUT_FADE_MILLIS`), one figure that keeps updating rather than a
    queue — the timer is cancelled and rearmed on every gesture. It sits low and
    centred so it never covers what the reader is adjusting, and it is an
    overlay so it costs no layout (rule 6). It is **never a permanent fixture**:
    the size itself lives in the sidebar dot's vocabulary and the page, not in a
    figure that stays. And it is **only for something the user did** — an
    arrangement switch redraws every game at its remembered size and shows no
    figure over any of them, because the arrangement asked for that change, not
    the person. A future acknowledgement of a different direct gesture uses this
    same shape rather than inventing a second.

11. **Show a measured figure right-aligned to a shared edge, in the smaller type
    at a fixed-width numeric style, rounded to one unit, and draw an unmeasured
    one as an en dash — never a zero.** The sidebar's memory footer
    (`memory_footer.rs`, wireframe `05-memory-accounting/wireframes/memory-footer.md`)
    is the first place the application shows a number at all. Right-aligned to a
    common edge so a column of figures can be compared and summed by eye; the
    fixed-width numeric style (`.numeric`, the CSS `font-variant-numeric`) so a
    digit changing on a sample never shifts its neighbour; rounded to the whole
    unit — `MiB` for memory — because a footer is a glance and the conversion
    from the stored kibibytes happens once, at the label, never on the way in. A
    value with no measurement behind it is `–`, because a `0` is a claim that a
    measurement was taken and came back empty. The distinction lives in the type
    — the reading is absent, not zeroed — and the formatter is where absence
    becomes the dash (`format_figure`).

12. **A readout that changes appearance because a measurement crossed a line
    must clear itself the moment a later measurement crosses back, and must
    offer no action — the deliberate opposite of rule 9's message strip.** The
    sidebar memory footer's over-budget appearance (`memory_footer.rs`, wireframe
    `## Over budget`) is the first case: when a sample's total crosses the budget
    in `docs/memory-budget.md` the block takes the theme's warning tint — not the
    error red, this is attention not alarm — and adds one line naming the budget
    (`FR.20.1`). It does three things rule 9's strip does not. It **does not
    persist**: the moment a sample comes back under, tint and line go, with
    nothing to dismiss, because it reports a condition that is true right now or
    is not, where the strip reports an event that happened once and a faded strip
    would lose the only record of it (`FR.20.3`). It **offers no action**: no
    Park button, because the application cannot tell which account is
    responsible and choosing what to give up is the user's (`FR.7.4`,
    `FR.20.2`). And it **moves nothing** — no figure shifts, resizes or reorders
    when the appearance changes, so an eye returning to the total every few
    minutes never has to re-find it. The verdict itself is the core's
    (`MemoryReading::budget_verdict`); the footer toggles one CSS class from the
    answer and compares nothing.
