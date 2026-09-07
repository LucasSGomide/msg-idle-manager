# 04 — Restoring the arrangement on launch

**Roadmap:** [07](../../roadmap/07-workspace-restore/README.md) · **Scope:** full-stack · **Depends on:** 02, 03

## Context

Everything needed to remember an arrangement now exists: the rules layer can
describe one and rebuild itself from one, the file on disk can hold one, and the
window knows how to draw an account that is waiting its turn. Nothing yet joins
them. This slice makes opening the program actually restore what the user left.

The order matters and is the point of the whole item. Before any window exists,
the program reads the saved file. It hands what it read to the window, which
rebuilds its arrangement and draws all of it at once: every account in the list,
the saved arrangement selected, every account in the slot it held. Only then, in
the slice after this one, do the games themselves start arriving. Drawing
everything first is what makes the window immediately honest about what the user
has, instead of a long freeze followed by everything appearing together.

Each restored account gets its own private storage on disk prepared for it, the
same storage it had before, which is why a restored game finds the user still
signed in. What it does not get is a running game: an account the user left
running comes back waiting its turn, and an account the user parked comes back
parked and costs nothing. So at the end of this slice the window comes back
perfectly arranged with every slot showing a panel and no game loaded — which is
precisely the state the queue in the next slice consumes.

Two things that are not a restore also have to be right. On a very first run
there is no file, which is not an error and gets no message: the window opens
empty exactly as it always has. And a file that will not parse is a real
problem, so this slice adds the one place in the window where the program can
say something the user did not ask for: a strip across the top, under the
toolbar and above everything else, holding one line and a close button. It says
the arrangement could not be read and names the file that was kept aside, and
below it the window is a first run. The next slice but one reuses that same
strip to report a save that failed, which is why it is built as its own widget
rather than a label dropped into the window.

## User experience

- **Entry** — none. This slice adds no control; it is what opening the
  application does.
- **Flow** — open the program and the window appears already arranged: the right
  accounts in the list, the right arrangement selected, the right slots filled,
  the parked ones parked.
- **States** — **first run**: no file, so the window opens empty exactly as item
  01 leaves it, with no strip and nothing to dismiss.
- **States** — **restored, nothing started yet**: the arrangement is complete and
  correct, every account that was running reads as queued, every account that
  was parked reads as parked.
- **States** — **unreadable file**: a strip spans the full width of the window
  directly under the header bar, above both the sidebar and the grid, holding
  one line saying the workspace could not be read and where the old file was
  kept, plus a close button on its trailing edge. Below it the window is a first
  run.
- **New pattern** — a dismissible message strip for something that happened
  before the user did anything. The design doc owes a rule for where such a
  message goes, how it is dismissed and whether it ever leaves on its own.

## Technical details

- **Architecture** — the composition root builds `TomlWorkspaceStore`, reads the
  workspace before building the window, and hands both what it read and the port
  itself to the shell (rule 3). The shell keeps the port as
  `Rc<dyn WorkspaceStore>` beside the locator and the catalogue, so it never
  learns a file is involved.
- **Front-end** — `Window::new` and `attach_ports` grow the store and the read
  outcome. The outcome is three-way — a workspace, no file yet, or a failure
  carrying the kept file's path — and each of the three has its own arm; a
  failure is never flattened into "no file" (code standards rules 12, 14, 15:
  logged with `tracing` in fields and shown, never dropped with `let _ =`).
- **Front-end** — `SessionView` gains a constructor that builds the holder and
  its persistent network session without building a view, and `SessionView::new`
  becomes that plus `start`, so there stays one path that builds a view. A
  restored account gets the holder, not a view: building one only to stop it
  would spend the process spike this whole item exists to spread out.
- **Front-end** — the restore path locates each restored account's profile
  directories, builds its dormant holder, registers it with the grid at its
  saved placement, and redraws once — list, arrangement and every slot — before
  anything loads. A profile directory that cannot be prepared is logged and that
  one account is skipped rather than the restore abandoned.
- **Front-end** — `message_strip.rs` with its `imp` module and
  `resources/ui/message-strip.ui`, added to the `GResource` bundle and placed in
  `window.ui` directly under the header bar, spanning the sidebar and the grid.
  It carries one line and a close button, is hidden while it has nothing to say,
  and exposes setting a message and clearing it so task 06 can reuse it for a
  failed save (architecture rules 12, 13; naming rules 1, 2, 4, 7).
- **Design** — write the window-level message rule into `docs/design.md`: where
  the strip sits, that it is dismissible, and whether it ever goes on its own.
  It is the second of the item's two design debts, and item 08 will want it for
  a crashed account.
- **Front-end** — the unreadable case shows the strip naming the kept file and
  builds the window from an empty book, so the empty state below it is item 01's
  unchanged. Dismissing the strip leaves the first-run window alone.
- **Testing** — shell behaviour is `(manual)`, run against the headless harness
  in this folder's `test-script.md`, because `cargo test` never requires a
  display server (architecture rule 14, code standards rule 25). Point
  `XDG_CONFIG_HOME` and `XDG_DATA_HOME` at a temporary tree and write the
  workspace file by hand to set each case up.

## Acceptance criteria

- [x] `(manual)` with a workspace file holding three accounts in the
      two-by-two arrangement, one of them parked, opening the program shows all
      three rows, that arrangement selected and each account in its saved slot,
      with no game loaded yet
- [x] `(manual)` an account saved as running comes back queued: its row reads
      queued and its slot shows the panel with the queued line and no button
- [x] `(manual)` an account saved as parked comes back parked, reads as parked
      immediately, and never passes through queued or starting
- [ ] `(manual)` a restored account keeps its name, its zoom and its browser
      identity, and starting it from the row menu loads its game already signed
      in, with no new login
- [x] `(manual)` with no workspace file at all, the window opens as item 01's
      empty state, with no strip and nothing to dismiss
- [x] `(manual)` with a workspace file that will not parse, a strip under the
      header bar spans the window naming the kept file, the window below it is a
      first run, and the strip's close button dismisses it for good
- [x] `(manual)` launching a second copy of the program while one is open
      presents the window already open instead of starting a second process, so
      two copies cannot write the workspace or open one account's storage at
      once
- [x] `(unit)` the message-strip template is readable from the compiled resource
      bundle, as the window and placeholder templates are

## References

- [Roadmap item](../../roadmap/07-workspace-restore/README.md) — the full
  picture, including the "Restoring on launch" diagram whose order this slice
  implements up to the point the queue takes over
- [The launch window wireframe](../../roadmap/07-workspace-restore/wireframes/launch-window.md)
  — the whole item's screen; this slice draws the arrangement, the first run and
  the message strip, and leaves the queue draining to task 05
- [`docs/requirements.md`](../../requirements.md) — `FR.8.1`, `FR.8.3`
- [`docs/design.md`](../../design.md) — rules 1, 3, 4, and the window-level
  message rule this slice writes
- [`docs/architecture.md`](../../architecture.md) — rules 3, 8, 10, 12, 13, 14
- [`docs/code-standards.md`](../../code-standards.md) — rules 6, 7, 12, 13, 14,
  15, 17, 25
- [`docs/naming.md`](../../naming.md) — rules 1, 2, 4, 7, 9

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`.
