//! The seam between the interface and whichever web engine actually renders
//! an account's game page: `WebKitGTK` on Linux, Microsoft Edge `WebView2` on
//! Windows.
//!
//! [`EngineShared`] is the process-wide engine state, [`EngineProfile`] is
//! one account's long-lived engine profile, and [`EngineView`] is the view of
//! a single start. The rest of the interface talks only to these three
//! engine-neutral types; which backend they wrap is chosen at compile time
//! and never faked, so no port for this belongs in `idle-manager-core`
//! (architecture rules 3, 6).

#[cfg(target_os = "linux")]
mod webkit;
#[cfg(windows)]
mod webview2;

// Pure logic the Windows backend needs, kept outside `webview2`'s own
// `cfg(windows)` gate so it compiles — and its tests run — on Linux too.
// `make windows-check` only compiles and lints the Windows target; it never
// links or runs it, so this is the one part of the seam this machine can
// actually execute a test against (architecture rule 14).
#[cfg(any(windows, test))]
mod host_bounds;
#[cfg(any(windows, test))]
mod ipc_message;
#[cfg(any(windows, test))]
mod profile_name;
#[cfg(any(windows, test))]
mod virtual_key;

/// One account's engine-held data could not be deleted (roadmap item 12
/// task 06).
///
/// Engine-neutral on purpose: `account_deletion.rs` runs one sequence for
/// both engines and maps this onto item 11's own `ProfileRemovalError`, so a
/// failure shows the delete-account dialog's existing error page with
/// `Retry` and `Close` whichever engine raised it (`FR.21.11`).
#[derive(Debug, thiserror::Error)]
#[error("{reason}")]
pub(crate) struct EngineDeleteError {
    /// What went wrong, in the words the dialog's failure page shows — never
    /// prefixed with a path, the same rule `XdgProfileRemoval::remove`
    /// keeps: the dialog shows the account's folder on its own line beside
    /// this reason.
    pub(crate) reason: String,
}

#[cfg(target_os = "linux")]
pub(crate) use webkit::{EngineProfile, EngineShared, EngineView};
#[cfg(windows)]
pub(crate) use webview2::{EngineProfile, EngineShared, EngineView};

// `EngineHost` itself, not just `EngineView`, reaches outside this module on
// Windows only: `session_grid/imp.rs` downcasts `EngineView::widget()` back
// to it so it can shrink a game's native child window to nothing and restore
// it, the way task 05's cover, drag and drop-highlight handling all need
// (roadmap item 12, "Airspace"). Linux has no equivalent type to export —
// its overlay never has to win against a native child window.
#[cfg(windows)]
pub(crate) use webview2::EngineHost;
