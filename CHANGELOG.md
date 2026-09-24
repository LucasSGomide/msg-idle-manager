# Changelog

All notable changes to Idle Manager are documented in this file. Unreleased
work lands at the top when `make release-prepare` runs.

## [0.1.0] - 2026-09-24

### New

- Domain layer for sessions, layouts, and profile locations

- Filesystem profile locator implementing XDG standard

- GTK4 application shell with window, grid, and dialogs

- Wire up composition root and entry point

- Add focus-session placement transition

- Add the session sidebar with fold and row activation

- Reload the focused game from the header bar and F5

- Add session liveness with park, unpark and started transitions

- Hold each account's engine objects in a SessionView

- Park a running account from its sidebar row

- Start a parked account from the same row button

- Show a start panel in a parked account's slot

- Keep-awake flag in the core and the engine feature lookup

- The row settings menu and the engine's hidden-page switches

- The frame-callback shim for a hidden page

- The row's keep-awake indication, and the width it costs

- The preset and catalogue port in the domain

- TOML preset catalogue, shipped games, and first-run seeding

- Two-stage add-game dialog, and per-account zoom and identity

- Dot-only sidebar row marker with park/start in the row menu

- The workspace value, restore, and the WorkspaceStore port

- The workspace file on disk

- The queued row and the queued slot

- Restore the arrangement on launch

- The start queue

- Save the workspace after every change

- Per-arrangement zoom in the domain

- The book's zoom transitions and the focused account

- The zoom memory port and the account state file

- Zooming the focused account from the keyboard

- Zooming the account under the pointer

- Snapping every account on an arrangement switch

- Remembering the sizes across a restart

- Memory reading, the /proc probe, and the report script

- WebGL per game and diagnostics off by default (task 07)

- Sidebar memory footer, engine memory-pressure settings, budget warning

- Add the one account-name rule and SessionBook::rename

- The grip over each place

- Move an account to another slot in the book

- Renaming an account from the sidebar

- Dropping an account on a place (task 05)

- Add the workspace book and version-2 file model

- Persist version-2 workspace lists to disk

- Show workspaces as a sidebar tree

- Choose a workspace when adding a game

- Name workspaces from New, Rename and Remove

- Delete an account and its folder

- Wire workspace switching, moving and selection through the window

- Resolve the shared webview2 engine data root

- Read private memory on windows through the process tree

- Put one web-engine seam between the interface and webkitgtk or webview2

- Pick the engine and memory probe per platform, no console on a windows release

- The remote vocabulary, its two ports and the phone record on disk

- Capture a frame, run a script and wake a view on both engines

- A debug switch that minimises the window after a delay

- The remote server — enrolment, the socket and its proof

- The Mobile layout on the desktop

- The phone page

- Wire the phone into the window

- The phone dialog and the header menu

- A stable address for the enrolled phone, `/?d=<device id>`

- The phone port through the Windows VM, the latency probe, and the docs the item owed

- The shortcuts window and the main menu

- The seat model becomes pages, and the file becomes version 3

- Stepping and paging in the domain

- One shortcut table, and the keys on the GTK controller

- The header-bar pager

- Wire the pager's arrows and readout into the window

- The keys on Windows

- Park all and Start all

- The eight window chords, and focus that hands over the keyboard

- The new keys on Windows

- Redesign the header bar, sidebar and dialogs

- Check GitHub for a newer signed release in the background

- Offer a newer release in a notice bar and install it on restart


### Fixed

- Gate the wheel zoom on the focused slot, one step per notch

- Stop the memory footer's hexpand from widening the collapsed sidebar

- Disconnect the previous load-changed handler on each keep-awake toggle

- Apply the measured memory-pressure limit instead of the discarded placeholder

- Gate the page-console bridge behind its own diagnostics switch

- Wire F12 to open the inspector

- Raise the web-process memory limit above a played game's working set

- Draw with the OpenGL renderer so web frames skip a CPU copy

- Show grab cursor on the slot-grip hover handle

- Skip a zombie or unreadable descendant instead of blanking the memory figure

- Tell a watched page it is visible, or the game pauses itself

- A page loaded while minimised gets its observers, and is armed from its first script

- Revert four sidebar details per owner feedback

- Shrink the status dot and its glow to match the pause icon

- Shrink the status dot to 4px

- Centre the status dot and enlarge the current-row glow


### merge

- Bring in the zombie-descendant PSS probe fix

- Bring in windows support (item 12)

