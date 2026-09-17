//! The fixed async sequence behind the delete-account spinner (item 11 task
//! 08, `FR.21.10`).
//!
//! Pulled out of `window/imp.rs` so the window stays one level of
//! abstraction (code standards rule 6): [`delete_account`] knows nothing
//! about the book, the sidebar or the dialog, only the one account's own
//! holder, the grid and the removal port. The window decides what to do with
//! the [`Result`] it returns.

use std::time::Duration;

use gtk::gio;
use gtk::glib;
use gtk4 as gtk;

use idle_manager_core::{ProfileRemoval, ProfileRemovalError, SessionId};

use crate::session_grid::SessionGrid;
use crate::web_view::SessionView;

/// How long the sequence waits after the view is gone before removing the
/// profile folder — twice the slowest storage close measured (roadmap item
/// 11, Technical References), so the engine has finished closing the files
/// it is willing to close before the folder underneath them goes.
const DELETE_SETTLE_MILLIS: u64 = 250;

/// Runs the fixed deletion order (`FR.21.10`) for one account:
///
/// 1. Clears `holder`'s website data and stops it, then drops the view from
///    `grid` — skipped entirely when `holder` is `None`, which is what makes
///    a retry after a first attempt already did this safe to run again.
/// 2. Waits [`DELETE_SETTLE_MILLIS`].
/// 3. Removes the profile folder through `removal`, off the main thread.
///
/// The caller is responsible for parking the account first and for deciding
/// what a success or failure means to the book, the dialog and the sidebar —
/// this function only ever touches the one holder it is handed, the grid,
/// and the removal port.
pub(crate) async fn delete_account(
    id: SessionId,
    holder: Option<SessionView>,
    grid: SessionGrid,
    removal: std::sync::Arc<dyn ProfileRemoval>,
) -> Result<(), ProfileRemovalError> {
    if let Some(mut holder) = holder {
        holder.clear_data().await;
        holder.stop();
        grid.remove_session(&id);
    }

    glib::timeout_future(Duration::from_millis(DELETE_SETTLE_MILLIS)).await;

    let remove_id = id.clone();
    let outcome = gio::spawn_blocking(move || removal.remove(&remove_id)).await;

    if let Ok(result) = outcome {
        result
    } else {
        tracing::error!(session = %id, "the account deletion task panicked");
        Err(ProfileRemovalError {
            reason: "an internal error interrupted the deletion".to_owned(),
        })
    }
}
