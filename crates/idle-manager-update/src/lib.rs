//! Velopack's own hooks: the one place in the workspace allowed to know
//! Velopack exists (architecture rules 2, 3, 4; roadmap item 16).
//!
//! Velopack's helper, `Update.exe` on Windows, starts the freshly-unpacked or
//! freshly-restored program with special arguments during an install, an
//! update or an uninstall and expects it to exit at once — the program has no
//! window, no store, nothing open yet at that point. [`run_hooks`] is that
//! contract, called as the very first statement of `main` before anything
//! else touches the filesystem or the display server.

use velopack::VelopackApp;

/// Runs Velopack's install/update/uninstall hooks and exits the process at
/// once if the current invocation was one of them.
///
/// Call this before any other line in `main`: before the renderer choice,
/// before the tracing subscriber, before any store is opened. A hook
/// invocation carries an argument such as `--veloapp-install <dir>` that
/// names it as Velopack's helper restarting the program mid-swap, never a
/// user launch, and the only correct response is to do the hook's work (or
/// nothing, on Linux, where there is no swap to react to) and exit — running
/// the rest of `main` against a half-installed folder is what this guards
/// against. An ordinary launch returns immediately and changes nothing.
pub fn run_hooks() {
    VelopackApp::build().run();
}
