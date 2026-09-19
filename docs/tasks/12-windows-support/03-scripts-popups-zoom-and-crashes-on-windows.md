# 03 — Page scripts, sign-in popups, zoom and crashes on Windows

**Roadmap:** [12](../../roadmap/12-windows-support/README.md) · **Scope:** front-end · **Depends on:** 02

## Context

On Windows the application now shows each account's game page with Microsoft
Edge WebView2. The page is only shown, though. This slice makes the Windows
page behave the way it already does on Linux in four respects.

First, the small scripts the application injects into every page before the page
runs now work. One of them forwards the page's console messages to the
application's log. They were written against WebKit's way of sending a message
from a page to the program. A short bridge script, loaded first, gives them that
same interface on top of WebView2's own message channel, so the existing scripts
run unchanged.

Second, games that sign in through a popup window can open that popup. It shows
in a window of its own, with the same account's cookies, at the same size Linux
uses.

Third, zoom behaves the same. Pressing `Ctrl` and `+`, `-` or `0` while a game
has keyboard focus reaches the application's own shortcut handling instead of
being swallowed by the page. Holding `Ctrl` and turning the mouse wheel over a
game steps the application's own zoom one notch at a time. WebView2's built-in
zoom is switched off, so the two never add up.

Fourth, when a game's engine process dies or stops responding, the place shows
the stopped panel exactly as Linux does, instead of a frozen or blank page.

These four belong together because they are all hooks on the same newly created
view, and all live in the Windows engine files. The short percentage that
flashes after a zoom step is drawn over the game on Linux, and on Windows it
would be hidden behind the page. Moving it somewhere visible is a separate
slice.

## User experience

- **Flow** — `Ctrl` `+`/`-`/`0` with a game focused, or `Ctrl`+wheel over a
  game, resizes that game through the application's zoom steps (item 09
  behaviour).
- **Flow** — A game's "sign in with…" button opens a popup window on the same
  account; completing sign-in logs that account in.
- **States** — A game's engine process dies: the place shows the stopped panel
  exactly as the Linux terminated-process branch does (item 03).
- **Pattern** — The absent-game panel is reused unchanged for a stopped game
  (design rule 4, `SlotPlaceholder`).

## Technical details

- **Architecture** — add `resources/js/webview2-bridge.js`, injected first with
  `with_initialization_script` on Windows only. It defines
  `window.webkit.messageHandlers.<name>.postMessage(body)` to call
  `window.ipc.postMessage(JSON.stringify({handler: name, body}))`.
  `page-console.js` and `keep-awake.js` stay byte-for-byte unchanged. It is
  loaded with `include_str!`, as the existing scripts are (naming rule 1).
- **Architecture** — the bridge also adds a capture-phase `wheel` listener. When
  `ctrlKey` is set and `deltaY` is not 0, it calls `preventDefault` and posts
  `{handler: "zoomStep", body: sign(deltaY)}`, one step per event to match item
  09's `DISCRETE` behaviour. The Windows `with_ipc_handler` parses the JSON,
  routes `pageConsole` to the same `tracing` call Linux uses, and routes
  `zoomStep` to the window's existing `apply_zoom_step` for the account
  (architecture rule 8).
- **Architecture** — `ffi.rs` (the one unsafe-allowed module) gets the
  `ICoreWebView2Controller` from wry's `WebViewExtWindows::controller`. It sets
  `ICoreWebView2Settings::SetIsZoomControlEnabled(false)` and subscribes
  `AcceleratorKeyPressed`. For a key-down with `Ctrl`, the virtual key becomes a
  GDK keyval and modifier set and goes to the function the window's key
  controller already calls in `window/imp.rs`. `put_Handled(true)` is set only
  when that function consumed the key (FR.1.7).
- **Architecture** — `ffi.rs` subscribes `ProcessFailed`.
  `RenderProcessExited`, `RenderProcessUnresponsive` and `BrowserProcessExited`
  invoke the view's `connect_terminated` callbacks on the GTK main context via
  `glib::MainContext::default().invoke_local` (architecture rule 10). This
  replaces task 02's stub and lands in item 03's existing terminated branch.
  A browser-process exit fires it for every live account (FR.1.6).
- **Architecture** — `with_new_window_req_handler` opens the requested address
  in a new `gtk::Window` of `POPUP_WIDTH`×`POPUP_HEIGHT` holding an
  `EngineHost`. The popup view is built on the opener's environment and profile
  name, and is closed when the page calls `window.close`
  (`WindowCloseRequested`).
- **Code standards** — every callback holds a `glib::WeakRef` to the window or
  grid, never a strong one. Every `unsafe` block carries `// SAFETY:` (rule 28 as
  amended). A comment names the WebView2 constraint behind each workaround
  (rule 18).
- **Code standards** — the virtual-key to keyval mapping is a pure function in
  `webview2.rs` with unit tests, compiled and tested only under `cfg(windows)`
  and linted from Linux by `make windows-check` (rule 25).

## Acceptance criteria

- [x] `(unit)` the ipc message parser maps `{"handler":"zoomStep","body":-1}`
      to a zoom-out step, a `pageConsole` message to a console entry, and
      malformed JSON to a logged rejection with no panic
- [x] `(unit)` the virtual-key mapping turns `VK_OEM_PLUS`, `VK_ADD`,
      `VK_OEM_MINUS`, `VK_SUBTRACT`, `0` and `VK_NUMPAD0` with `Ctrl` into the
      keyvals the window's zoom shortcuts match
- [x] `(integration)` `make windows-check` and `make verify` pass
- [ ] `(manual)` in the Windows VM, a page's `console.log("x")` appears in the
      application's debug log under the account's session field
- [ ] `(manual)` in the Windows VM, `Ctrl`+`+` twice then `Ctrl`+`0` with a game
      focused makes the page 110%, 120%, then 100%, and WebView2's own zoom
      popup never appears
- [ ] `(manual)` in the Windows VM, one `Ctrl`+wheel notch over a game changes its
      zoom by exactly one step, and the size is still remembered after a
      relaunch (item 09)
- [ ] `(manual)` in the Windows VM, a game's popup sign-in opens in its own window and
      leaves that account logged in after the popup closes
- [ ] `(manual)` in the Windows VM, ending one account's renderer process in Task
      Manager shows the stopped panel in that place only, and its `Start`
      button brings the game back

## References

- [Roadmap item](../../roadmap/12-windows-support/README.md) — Front-end "The
  Windows engine" (scripts, keys, `ProcessFailed`, popups) and the "Zoom
  acknowledgement on Windows" diagram
- [Wireframes](../../roadmap/12-windows-support/wireframes/) —
  `windows-game-place.md` (stopped variant)
- [`docs/requirements.md`](../../requirements.md) — Platform Support `FR.1.2`,
  `FR.1.6`, `FR.1.7`
- [`docs/architecture.md`](../../architecture.md) — rules 8, 10
- [`docs/code-standards.md`](../../code-standards.md) — rules 18, 25, 28
- [`docs/naming.md`](../../naming.md) — rule 1
- [`docs/design.md`](../../design.md) — rule 4

## Implement with

No implementation skill exists in this repository yet. Implement by hand against
the rule docs above, then run `/msg-roadmap-sync`. The `(manual)` steps run in the
Windows VM ([`docs/windows-vm.md`](../../windows-vm.md)).
