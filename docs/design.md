# Design rules

Rules for anything design-shaped. `project.yml` points every
`**Design**` bullet in a roadmap item at this file.

Each rule is one imperative and one line of why. A rule with no why is a
preference, and the next person will not know whether to keep it.

Numbered, because roadmap items cite them by number — renumbering breaks the
citations, so append rather than reorder.

1. **Mark a sidebar row's state with a trailing word and a coloured dot, one
   pair per state, styled in `session_sidebar/row.rs` and `sidebar.css` by a
   single status key.** The marker has to carry several facts as the roadmap
   grows — visibility now, parked in item 03, unresponsive in item 08 — so a
   key-per-state that both the label text and the dot's CSS class derive from
   keeps each new state one arm of one match, never a fourth branch in the row
   factory. The states this repo has so far: `current` (holds the focused slot)
   — green dot with a neon glow, name bold; `visible` (on screen, not focused) —
   green dot, no glow; `background` (running out of sight) — amber dot with a
   glow, name dimmed to 55% alpha; `parked` (not running, whatever place it
   still holds) — grey unlit dot, name dimmed to 55% alpha; `starting`
   (unparked, no page painted yet) — blue dot with a glow, and the row's action
   button insensitive so the start cannot be pressed twice.

2. **Give a list row one action button whose label and effect invert with the
   row's state, not two buttons or a menu.** The row action here is always "move
   this account to the other liveness" — `Park` while it runs, `Start` once it
   is parked — so a single control that reads the current state can never be
   pressed in a direction that does not apply, where a pair would always have
   one half disabled and a menu would hide a one-click action behind two.

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

5. **Put a setting a row carries once and then forgets behind a menu button on
   the row's trailing edge; keep the row's action button, rule 2's single
   invert-with-state control, free of it.** "Keep running when hidden" is
   chosen once per account and left alone, unlike Park/Start, which is pressed
   constantly — stacking it on the row would either spend permanent width on
   something touched once, or turn the row's one action into two, which rule 2
   already forbids. A `gtk::MenuButton` opening a `gio::Menu` costs the row
   nothing while closed, which is the point: something reached rarely is a
   click away behind its own affordance, not a fixture beside the one that
   matters every time.

6. **When a row's trailing edge is asked to carry more than one fact, let the
   account's name give way, never a fact at the edge — and size the sidebar so
   that giving way costs the name only length, never legibility.** The name
   label is the row's only hexpanding child and already ellipsises with
   `pango::EllipsizeMode::End`, so it is the one part built to lose length;
   every fact after it — place, state, keep-awake, the action button, the menu
   button — keeps its full width and its full position, whatever the account
   is called. But the sidebar's `width-request` is not a number fixed once and
   forgotten: it is the trailing edge's total natural width plus enough room
   left over for a short name to still render in full, and it is *derived*
   from that edge, not the other way around. Item 04's keep-awake mark moved
   it from 220 to 240 for exactly this reason — at 220 a four-letter name
   collapsed to a bare ellipsis, which is the edge eating the name rather than
   the name giving way to it. The next item that adds another trailing fact
   owes the same check: settle the width by eye, with a short name on screen,
   rather than let the name silently absorb a fact's cost.
