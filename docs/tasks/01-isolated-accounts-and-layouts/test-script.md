# 01 — Isolated game accounts: hand-run test script

Proves the shell end to end, the coverage architecture rule 14 keeps out of
`cargo test`. Every box is one concrete action and the result it must produce.

The automated half (`(unit)` and `(integration)` criteria) is not repeated here;
run it with `make verify`.

## Setup

- [x] `make verify` exits 0 — format, clippy, tests, audit, layer boundaries and
      the roadmap check all pass.
- [x] `cargo build --workspace` produces `target/debug/idle-manager`.
- [x] A display server is available (`$DISPLAY` or `$WAYLAND_DISPLAY` set). The
      verification runs below used a headless X server (`Xvfb :99`) with
      `GDK_BACKEND=x11 GSK_RENDERER=cairo` and synthetic input; a normal desktop
      needs none of that.
- [x] `export XDG_DATA_HOME="$(mktemp -d)"` so the run's profiles land in a
      throwaway directory and can be inspected and deleted afterwards.

## Teardown

- [x] Close the window; the process exits with status 0.
- [x] `rm -rf "$XDG_DATA_HOME"` removes every profile directory the run created.

## 01 — Application shell and the resource pipeline

- [x] Run `make run`. A window titled "Idle Manager" opens; its header bar shows
      an "Add game" button on the trailing edge with a linked group of three
      toggles ("1", "2", "4") beside it, "1" pressed.
- [x] The content area shows one line of dim text ("No games yet — add one to
      get started.") above a suggested-action "Add your first game" button, the
      pair centred horizontally and vertically.
- [x] Resize the window taller and wider; the text-and-button block stays
      centred. (Verified at 1280×800 and again after re-launching at other sizes;
      the block uses `halign`/`valign` centre with expand so it tracks any
      allocation.)
- [x] Close the window (title-bar close, or `Ctrl`+`W` via the compositor). The
      process ends and `make run` exits 0. (`WM_DELETE_WINDOW` → app exit code
      0.)

## 02 — Session and layout domain

- [x] `cargo nextest run -p idle-manager-core` — 14 tests pass, covering every
      `## Acceptance criteria` line: distinct minted ids, lowest free slot,
      focused-slot displacement, shrink-to-off-grid, grow-back-to-remembered,
      collided-slot fallback, deterministic placement, and untouched name /
      address.

## 03 — Profile directories port and its XDG adapter

- [x] `cargo nextest run -p idle-manager-store` — 6 integration tests pass:
      data + cache created under the given root, idempotent second call, two ids
      → two directories, every path inside the root, a read-only root →
      `NotWritable`, a file where a directory must go → `NotCreated`.
- [x] `make arch-check` prints "layer boundaries hold" — `idle-manager-core`
      reaches no filesystem crate, `idle-manager-shell` reaches no store crate.
- [x] `make run` builds `XdgProfileLocator::new()` in the binary, hands it to the
      shell as `Rc<dyn ProfileLocator>`, and the window still opens.
- [x] After adding an account (below), `find "$XDG_DATA_HOME"/idle-manager/profiles`
      shows `<id>/data/` and `<id>/cache/` for each account, one `cookies.sqlite`
      per account under `data/`, and every path inside `$XDG_DATA_HOME`.

## 04 — Adding an account and placing it in a slot

- [x] Press "Add game". A modal opens over the window with a "Name" entry and an
      "Address" entry, the cursor in the "Name" entry, and an "Add" button that
      is insensitive.
- [x] Type a name; "Add" stays insensitive. Type an address; "Add" becomes
      sensitive (suggested-action blue).
- [x] Press `Escape`. The dialog closes and no account is created (the empty
      state is still shown). Press "Add game" again; both entries are empty.
- [x] Fill both entries and press `Enter`. The dialog closes, the empty state is
      replaced by the grid, and the account's name shows centred on the window
      background until its page paints, then the page covers it.
- [x] Press "Add game" and add a second account. Its page appears in the next
      free slot of the current arrangement; the first account stays where it is.
- [x] From the empty state (no accounts), press "Add your first game". The same
      modal opens.
- [x] Click inside a slot. Its four edges thicken and the previously marked
      slot's edges return to a hairline — exactly one slot is marked.
- [x] In a 4-slot arrangement with one account, the three empty slots draw
      nothing inside them (only the hairline between slots).

## 05 — Changing the arrangement

- [x] Press each of "1", "2", "4" in turn. The grid redraws as one, two, or four
      rectangles with a hairline between neighbours; pressing one releases the
      other two.
- [x] With four accounts in the 2×2 arrangement, press "2". Accounts 1 and 2 stay
      in place; accounts 3 and 4 are drawn nowhere in the window.
- [x] Press "4" again. Accounts 3 and 4 return to the slots they held before, and
      their pages are unchanged — no reload, no loading placeholder.
- [x] While an account is out of sight, its page keeps running: bring it back and
      its scroll position and any running timer have advanced, not reset. (The
      grid never unparents or unrealises a child on a layout change; an off-grid
      child is allocated a rectangle outside the grid's bounds and clipped by
      `set_overflow(Hidden)`.)
- [x] With every slot occupied, focus one slot and press "Add game" to add
      another account. The new account takes the focused slot; that slot's
      previous occupant is drawn nowhere.
- [x] Grow the arrangement again. An out-of-sight account whose remembered slot
      was taken while it was away lands in the lowest-numbered free slot; if no
      slot is free it stays out of sight. (Backed by the core unit test of the
      same name; the grid renders the book's decision verbatim.)
- [x] Choose an arrangement with more slots than there are accounts. The extra
      slots are empty and draw nothing.

## 06 — The isolated web view per account

- [x] Add an account whose address is slow to respond (e.g.
      `https://httpbin.org/delay/3`). The slot shows the account's name centred
      on the window background, then the page replaces it once it paints.
- [x] Add two accounts both at `https://lorvath.com/`. Sign into a different
      Google account in each (call them A in `session-0001` and B in
      `session-0002`), completing the "Continue with Google" flow. Both slots
      then show their own account playing. Focus the first slot and press `F5`,
      then the second and press `F5`: each page reloads and comes back on the
      same account — neither reload logs the other slot out.
- [x] With both signed in,
      `find /tmp -path '*idle-manager*' -name cookies.sqlite` under the run's
      profile root (`/tmp/tmp.dw0lKxDsx7/idle-manager/profiles/`) shows one
      database per account, each 16 KB — twice the 8 KB of an untouched one — and
      the two differ:
      `session-0001` → `sha1 9b4d0635a86c1c524f9c4618e05fbd573e8d1dec`,
      `session-0002` → `sha1 f8d270a80ebb84f10e2980d4356d9cbe4b0acbdb`.
- [x] With a slot focused, the header bar's leading circular-arrow button
      reloads that slot's page; so do `F5` and `Ctrl`+`R`, and both keys reach
      the window even though `lorvath.com` binds them on its own canvas (the
      key controller sits in the capture phase). The other slot is untouched.
- [x] Open a page that echoes the request's `User-Agent` (e.g.
      `https://httpbin.org/user-agent`), or run `navigator.userAgent` in the web
      inspector. It returns the engine's own string, unmodified —
      `Mozilla/5.0 (X11; Ubuntu; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/60.5 Safari/605.1.15`
      on this machine. Nothing calls `set_user_agent`: a Chrome claim is what
      breaks a real login, and `FR.10.5` supersedes `FR.10.4` on that point.
- [x] With three accounts loaded, `ps -eo comm,args | grep webkitgtk-6.0` shows
      **one** `WebKitNetworkProcess` and **one `WebKitWebProcess` per account**.
      Measured on this machine: 2 accounts → 1 network + 2 web; 5 accounts →
      1 network + 5 web. The requirements listed this as unverified; it holds.
- [x] Add an account whose address cannot be resolved (e.g.
      `https://nonexistent.invalid.example`). Its slot shows the engine's own
      error page ("Error resolving …: Name or service not known"); the other
      slots stay loaded and running.
- [x] Switch arrangements while pages are loaded. Pages move between slots
      without reloading; a page moved out of sight and back keeps its state.
- [x] Add an account at a live game's login page (`https://huntera.com.br/login`)
      and click its "Continue with Google" button. A separate top-level window
      opens, transient over the main window, showing Google's own "Sign in — to
      continue to huntera.com.br" page. The log records
      `opening a popup window uri=Some("https://accounts.google.com/o/oauth2/v2/auth?…display=popup…") user_gesture=true`.
      *(Verified end to end up to Google's sign-in form on `huntera.com.br`; the
      full sign-in through to a logged-in game was exercised separately on
      `lorvath.com` in the two-account step above.)*
- [x] Load a page that calls `window.open` on load, with no click involved. It
      returns `null`, no window appears, and **no** `opening a popup window` line
      is logged — the engine refuses an ungestured open before the `create`
      signal is emitted, so nothing in this application has to.
- [x] With `RUST_LOG=idle_manager_shell=debug`, load a page whose script calls
      `console.error` and which embeds an iframe from a **different** origin that
      does the same. Both appear as `page console error` events, each carrying
      the origin of the frame that logged it — verified with
      `origin="http://127.0.0.1:8731"` and `origin="http://127.0.0.1:8732"`.
      Raise to `trace` and every `console.log` and subresource request follows;
      leave it at `debug` and a bot-check frame's several hundred lines per load
      stay out of the way.
