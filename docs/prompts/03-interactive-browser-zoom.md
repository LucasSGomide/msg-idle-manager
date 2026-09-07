# Goal: Add Ctrl-based interactive zoom to the browser view and persist each account's zoom to disk

**Status:** not executed
**Rating:** —
**Run:** parallel with 02 — the only file both may touch is
`crates/idle-manager-shell/src/window/imp.rs`; landing order does not matter.

## Context

The browser view has no interactive zoom. `ZoomLevel` is read once from the
game's preset file and applied in `SessionView::start()` via
`view.set_zoom_level(...)`; the preset file is read-only shared config and the
app keeps nothing writable per account — accounts are rebuilt from presets on
every launch, there is no per-account state on disk at all.

This prompt adds two things:

1. **Interactive zoom gestures**, acting on the account that holds the focused
   slot (the same "current" account the sidebar marks):
   - `Ctrl` `+` / `Ctrl` `=` — zoom in one step
   - `Ctrl` `-` — zoom out one step
   - `Ctrl` `0` — reset to the account's configured preset zoom
   - `Ctrl` + mouse wheel over a page — zoom that page continuously
   A step is ±10% (multiply / divide the current zoom by 1.1, or the nearest
   equivalent WebKitGTK exposes). Every result is clamped to `ZoomLevel`'s
   existing `0.25..=5.0` range. The keyboard shortcuts are window-level and
   route to the focused slot's view; `Ctrl`+wheel acts on the view the pointer
   is over.

2. **Per-account zoom persistence.** The interactively-chosen zoom is written to
   a new small writable per-account state file so the account reopens at that
   zoom after a park/restart and after quitting and relaunching the app. The
   preset value becomes the default the account starts life with and what
   `Ctrl` `0` resets to; once the user has zoomed, the persisted value wins at
   `start()`.

## Constraints

1. **Respect the crate layering** (`docs/architecture.md`, enforced by
   `make arch-check`). `idle-manager-core` may not touch serde or the
   filesystem: define a port trait in core (a reader/writer of per-account
   zoom, shaped like the existing `PresetCatalogue` / `ProfileLocator` ports)
   and put the TOML + fs adapter in `idle-manager-store`, wired in the
   composition root (`crates/idle-manager/src/main.rs`) alongside the locator
   and the preset catalogue.
2. **State file location:** a per-account file under the account's profile
   directory — e.g. `<XDG data>/idle-manager/profiles/<id>/state.toml` (a
   sibling of the existing `data/` and `cache/` dirs the `ProfileLocator`
   creates). Follow `FR.8.3`: this is engine-owned data, it goes under the XDG
   **data** dir, never the config dir the presets live in. One key for now
   (`zoom`); shape it so a later per-account setting is one more key, not a new
   file.
3. **Reads never fail the whole app.** A missing state file is normal (fresh
   account) and means "use the preset zoom". A malformed one falls back to the
   preset zoom and logs — same tolerance as `TomlPresetCatalogue`.
4. **Writes are best-effort and debounced-ish.** Persist on zoom change; a
   failed write logs and is not fatal. Do not write on every wheel tick if that
   means hammering the disk — settle to the final value (a short timeout, or
   write on gesture end) is fine.
5. **Core changes:** `Session` already carries `zoom: ZoomLevel`. Add whatever
   `SessionBook` method is needed to change a live account's zoom
   (`set_zoom(&SessionId, ZoomLevel)`), matching the no-op-on-missing-id,
   returns-the-new-value style of `park` / `set_keep_awake`. `ZoomLevel`
   clamping / validation stays in core.
6. **`SessionView`** gains a way to change zoom on the live `WebView` after
   `start()` (today `apply_account_settings` only runs at build time). `start()`
   reads the persisted zoom (falling back to the preset) rather than always the
   preset.
7. Window-level shortcuts: use `gtk::Application::set_accels_for_action` with
   `gio` actions, or an `EventControllerKey` on the window — whichever fits the
   existing window code. The `Ctrl`+wheel handler is an `EventControllerScroll`
   on the web view. Do not let `Ctrl`+wheel also scroll the page.
8. This work has no roadmap item and adds a genuinely new capability (writable
   per-account state). Per the repo's planning rules this likely wants a
   `docs/requirements.md` entry and a roadmap item before implementation —
   **flag this to the user and confirm how they want to proceed before writing
   code.** If they say proceed as a prompt, follow the branch-first rule
   (`feat/interactive-zoom` or similar) and run `make verify` before handing
   back.
9. New behaviour is backed by tests: `ZoomLevel` step + clamp (unit, core), the
   state-file adapter round-trip and its missing/malformed fallbacks
   (integration, store), and the `SessionBook::set_zoom` semantics (unit, core).
   The gesture wiring itself is verified by hand — add a section to the relevant
   `docs/tasks/*/test-script.md` only if this lands under a roadmap item.

## Output

Rust across `idle-manager-core` (port + `SessionBook`/`ZoomLevel`),
`idle-manager-store` (new TOML adapter + a new integration test file),
`idle-manager-shell` (`web_view.rs`, `window/imp.rs`), and
`crates/idle-manager/src/main.rs`. A new state-file module in the store crate.
