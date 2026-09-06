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
   still holds) — grey unlit dot, name dimmed to 55% alpha.

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
