# Goal: Make the sidebar status dot render at 4px and give the current row a visible green glow

**Status:** not executed
**Rating:** —

## Context
Since the UI redesign, the sidebar's status dot looks too big for both
`current` (on screen, focused) and `visible` (on screen, not focused) rows.
The current row's green glow is also gone.

The code already asks for 4px. `sidebar.css` sets `.status-dot` to
`min-width/min-height: 4px`. `session_sidebar/imp.rs` (`build_row_widgets`)
builds the dot as a `gtk::Box` with `width_request(4)`/`height_request(4)`
(commits `10b21d2`, `61ea791`). The on-screen dot is still big, so something
overrides that. The likely cause: the dot is a page of the `mark_stack`
`GtkStack`, which sizes to its largest page (the 10px icons). The Box has no
`halign`/`valign` of `Center`, so it stretches to 10×10. The `status-current`
glow (`1px`/`2px` box-shadow) is also too small to see. Confirm the cause
before fixing it.

Target: the dot renders at **4px** on screen. The `current` row gets a
**clearly visible soft green halo** a few px wide, obvious at a glance. The
`visible` dot stays green with no glow.

## Constraints
1. Check the fix in the running app, headless: Xvfb plus a screenshot, as in
   the memory note on verifying shell slices headless. Measure the dot's
   rendered size in pixels. Don't trust the declared CSS.
2. Leave the `parked`/`starting`/`queued` marks at 10px. The row height must
   not change when a row's state changes.
3. Keep the comments in `sidebar.css` and `imp.rs` accurate. They describe
   the size history (10 → 8 → 4) and which rows glow.
4. Work on this repo's session branch, per `CLAUDE.md`. `make verify` must
   pass.
