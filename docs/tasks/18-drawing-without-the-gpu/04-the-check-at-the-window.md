# 04 — The check at the window

**Roadmap:** [18](../../roadmap/18-drawing-without-the-gpu/README.md) · **Scope:** front-end · **Depends on:** 02, 03

## Context

The earlier slices built a per-account switch that makes the web engine draw one
game with the processor instead of the graphics chip, saved it in the workspace
file, let a game's preset file ask for it, and shipped a preset for Lux Idle that
does. Every part of that has an automated test, and the engine behaviour was
measured on the laptop where the fault happened: with the switch on, opening Lux
Idle's list of hunting grounds no longer makes graphics memory run away.

What was not done is the part only a person at the window can do. The
measurements drove the game with an injected script and turned the switch on by
calling the window's own handler from a timer, because clicking a menu inside a
running desktop application from a script is unreliable and would have popped
menus over the owner's screen while they were using it. So nobody has yet
opened an account's three-dot menu and ticked the item by hand, watched the row
show its reloading mark and come back, parked and started the account to see
the choice kept and the graphics memory fall to almost nothing, or added a new
account from the Lux Idle preset and seen the item already ticked.

This slice is those checks and nothing else. It changes no code. It exists as
its own slice so the code slices could be accepted on the evidence they have,
while these steps stay visibly open until someone runs them on the real
machine.

## User experience

- **Entry** — the ⋯ menu on a Lux Idle account's sidebar row, and the main
  menu's Add account dialog.
- **Flow** — tick `Draw without the GPU` on a live account → reloading mark,
  then live; reopen the menu → ticked; park and start → still ticked.
- **Flow** — add an account from the `Lux Idle` preset → its menu shows the
  item ticked.
- **Pattern** — design rules 1, 5 and 21, as built by task 03.

## Technical details

- **Code standards** — no code changes; run against a build of this branch, on
  the laptop, with the GPU reading from `test-script.md`'s Setup.
- **Code standards** — on acceptance, append a `## 04 — The check at the
  window` section to `test-script.md` holding these steps, ticked as run.

## Acceptance criteria

- [ ] `(manual)` on a live Lux Idle account, ticking `Draw without the GPU` in its row ⋯ menu shows the reloading mark, then live, and the item reads ticked when the menu is reopened
- [ ] `(manual)` parking and starting that account keeps the item ticked, and its rendering process holds under 50 MiB of GPU objects (`drm-total` in its fdinfo)
- [ ] `(manual)` adding an account from the `Lux Idle` preset gives an account whose ⋯ menu shows `Draw without the GPU` ticked

## References

- [Roadmap item](../../roadmap/18-drawing-without-the-gpu/README.md) — the full picture
- [Wireframes](../../roadmap/18-drawing-without-the-gpu/wireframes/) — the row menu
- [Test script](test-script.md) — Setup and the GPU reading
- [Design](../../design.md)

## Implement with

By hand at the window.
