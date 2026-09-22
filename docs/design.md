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
   rule 8). Both reasons are about a control the reader can *see*: one that
   carries a label, and one that occupies room on a scarce edge. Neither
   describes a keyboard chord, which has no label to invert and takes up no
   edge at all, so this rule governs the visible control and rule 20 governs
   the key beside it.

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
   beside the dot. Roadmap item 11 turned the list into a two-level tree —
   indenting every account row under its workspace's heading — and moved it
   150 to 200, so an indented short name still rendered in full beside the dot
   with the tree's own indent taken into account. The next item that changes a
   trailing fact, or another level of indent, owes the same check: settle the
   width by eye, with a short name on screen, rather than let the name
   silently absorb — or keep paying for — a fact's cost.

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

13. **Give a sidebar tree heading no dot, no bold and no mark of which
    workspace is shown — let the accounts nested beneath it carry every status
    signal, exactly as they would in a flat list.** A heading names a group,
    not a running thing: it holds no liveness, no visibility, nothing rule 1's
    key vocabulary describes, so giving it a dot would either invent a
    seventh, meaningless state or borrow one of the six that already belongs to
    an account. Whether the workspace under a heading is the one on screen is
    already answered by its accounts' own dots — a row keyed `current` or
    `visible` only ever appears under the shown workspace's heading — so a
    second marker on the heading itself would repeat that answer, not add one.
    An expander arrow is the heading's only affordance, expanding or collapsing
    and never switching (roadmap item 11).

14. **A control that must reach the user over a native child window a
    platform's own engine draws — not GTK — gets its own row of chrome next
    to the content instead of an overlay layer on top of it; a transient
    figure that must still win that airspace gets its own native surface
    instead.** Every earlier overlay rule in this file (4, 6, 9, 10) assumes
    GTK draws the whole stack, so its own z-order settles who wins. On
    Windows a game's page is a `WebView2` child window the platform's own
    compositor always draws above anything GTK overlays in the same area,
    so an overlaid grip or readout would simply not be seen
    (`session_grid/imp.rs` `mount_grip`, `build_readout`, roadmap item 12
    task 05). The drag grip moves into a thin `.grip-strip` row above the
    place, using the window's own background because it is now a real part
    of the chrome, not a control floating over live content. Being chrome,
    that row exists only where the control it carries could: a place with no
    live game — parked, queued or waiting to start — shows the plain panel
    of rule 4 with no row above it, and so does a place in a layout with
    nowhere to drag an account to (`session_grid/imp.rs`
    `sync_grip_strip`). The zoom
    readout moves into a `gtk::Popover` — its own native surface, so it still
    draws above the page — kept `can_focus(false)` and `can_target(false)`
    so it stays true to rule 10's "acknowledgement only, never an action."
    Both keep every other rule's shape, place and timing unchanged; only
    *how* they reach the screen differs. Linux needs none of this — its
    overlay keeps working exactly as rules 4, 6, 9 and 10 already describe —
    so this rule only ever adds a platform branch, never replaces the
    overlay itself.

15. **A slot that stands in for another screen keeps that screen's size:
    centred, outlined, and clipped by the window rather than scaled to fit
    it.** The `Mobile` layout draws one slot at the phone's own viewport
    (412 × 915 by default), centred horizontally and top-aligned, its outline
    in the slot-line colour and the rest of the grid the plain window
    background; a window shorter than the slot cuts the slot's bottom off
    (`session_grid/imp.rs`, roadmap item 13 task 03). The point of the slot is
    that what the desktop shows is what the phone shows, pixel for pixel — a
    game laid out for 412 pixels of width — so scaling it to the window would
    show the user something the phone never sees, and letting it grow with
    the window would change the page's layout under the phone's fingers. No
    grip, no grip strip and no zoom readout (rule 10 by contrast): zoom is
    locked in this layout and there is nowhere to drag an account to.

16. **Pair a device with one screen that shows the same secret two ways for
    a fixed time, says where things stand on its first line, and never
    raises a second window.** The phone dialog's status line is the link's
    answer, one of four, re-read once a second while the dialog is open so a
    scan changes it without a click; a desktop that is not listening puts
    the reason on that same line, dimmed (rule 8), and greys `Enrol…` while
    the rest of the dialog keeps working. The offer is a QR code for a camera
    and the identical address as selectable text for typing, with a countdown
    beneath them; when the offer is spent the block empties, when it runs out
    the line says to start again, and closing the dialog withdraws it — a
    code nobody can see is only a door left open. The destructive action
    (`Un-enrol`) is red, sits beside the constructive one, and is insensitive
    while there is nothing to cut off (`phone_dialog/imp.rs`, roadmap item 13
    task 07).

17. **A page the application serves to another device follows this file's
    rules where the medium allows and states where it departs.** The phone
    page (`idle-manager-remote`, roadmap item 13 task 05) is HTML outside
    GTK, yet it carries: rule 1's vocabulary — an account's liveness is the
    word `live` / `parked` / `starting` / `queued` beneath its name, and the
    current account is the highlighted row; rule 2's one inverting control —
    `Park` on a live row, `Start` on the others, insensitive while
    `starting` / `queued`; rule 13's headings — a workspace is a plain heading
    with no marker of which is shown. Where it departs: the game fills the
    whole screen with one translucent round handle in the top-left corner
    that slides the list over it, because a phone has no sidebar to give the
    list a permanent column; a lost socket keeps the last picture, dimmed,
    under one `Reconnecting…` line rather than a message strip (rule 9 by
    contrast), because the strip's job — say what went wrong before the user
    acts — is done by dimming the very thing they would act on; and an
    un-enrolled phone shows one sentence and nothing else, since there is no
    form left to keep working.

18. **Show a workspace's pages as two icon-button arrows around an `n/m`
    readout, joined in one linked box beside the controls it pages through,
    and hide the whole box the moment there is only one page rather than
    grey it.** The header-bar pager (`window.ui` `pager`, roadmap item 14
    task 06) sits directly left of the `1` `2` `4` `Phone` arrangement
    toggles it shares a row with, in the same `.linked` shape those toggles
    already use, with tabular figures on the readout (rule 11) so `9/10` and
    `10/10` take the same space and the toggles beside it never shift. The
    GNOME HIG would grey a control that does nothing rather than remove it,
    but a pager greyed at one page still claims pages exist to turn — the
    recorded choice is to hide it instead, so its very presence already
    answers "is there more than one page" (`FR.22.5`) and nobody has to press
    a disabled arrow to find out.

19. **List every key the window answers in one `GtkShortcutsWindow` built
    from a `gtk/help-overlay.ui` resource, opened by `Ctrl`+`?` and one
    `Keyboard Shortcuts` item in the main menu — and name a control's own key
    in its tooltip wherever the control mirrors one.** `help-overlay.ui`
    (roadmap item 14 task 02) is discovered by GTK itself at a fixed
    resource path, so the shortcuts window costs the application no wiring
    beyond the file: `win.show-help-overlay` and its accelerator are
    registered by the toolkit the moment the resource is found. Discoverability
    does not stop at the overlay: the pager's tooltip names `Shift`+`Tab` and
    a workspace heading's tooltip names `Ctrl`+`Tab` (`FR.25.1`), so a key can
    be learned either from the one place that lists all of them or from the
    control it moves (`FR.25.2`). Where the control is a menu item there is
    nowhere to hover, so the key goes in the item's accelerator text instead —
    `gio::MenuItem`'s `accel` attribute, which `GtkPopoverMenu` draws at the
    item's trailing edge — and the tooltip rule is read as "name the key on
    the control", not "name it in a tooltip" (`FR.25.4`, roadmap item 15).

20. **Give a two-state action one key per direction, each idempotent and
    inert where it does not apply, rather than one key that flips.**
    `Ctrl`+`P` always means parked and `Ctrl`+`S` always means running
    (`FR.26.3`); pressed in the direction the account is already in, each is
    swallowed and does nothing at all (`FR.26.6`). This is rule 2's reasoning
    carried onto a keyboard rather than an exception to it. Rule 2 asks for
    one control for two reasons — a control that reads the state cannot be
    pressed the wrong way, and a pair of buttons would leave one half greyed
    on the scarce trailing edge rule 6 is about — and neither survives the
    move: the wrong way is already a silent no-op, so there is no state to
    reach and nothing to undo, and a key occupies no edge. What is genuinely
    lost is that a key carries no label saying which way the press will go.
    The row's coloured dot already answers that, before either key is
    pressed. A single flip key would obey rule 2's letter and read worse:
    nothing would say which direction the next press takes, and liveness
    moves on its own — queued, then starting, then live — between deciding
    and pressing, so the flip could invert under the reader's hand. Two
    idempotent keys cannot. The heading menu's `Park all` / `Start all` were
    already a pair rather than a flip, and `Ctrl`+`Shift`+`P` /
    `Ctrl`+`Shift`+`S` follow them unchanged (`FR.26.4`).
