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
   glow, name dimmed to 55% alpha.
