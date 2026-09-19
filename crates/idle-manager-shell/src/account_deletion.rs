//! The fixed async sequence behind the delete-account spinner (item 11 task
//! 08, `FR.21.10`), run the same way on both engines (roadmap item 12 task
//! 06).
//!
//! Pulled out of `window/imp.rs` so the window stays one level of
//! abstraction (code standards rule 6): [`delete_account`] knows nothing
//! about the book, the sidebar or the dialog, only the one account's own
//! holder, the grid and the removal port. The window decides what to do with
//! the [`Result`] it returns.
//!
//! [`deletion_steps`] is the order itself, as data (code standards rules
//! 21–23): one pure function both engines' sequences are read from, so the
//! step that differs between them — what "delete the engine's data" *means*
//! — cannot quietly drag the order with it.

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

/// Which web engine a build runs, for [`deletion_steps`] to answer about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Engine {
    /// `WebKitGTK`, on Linux: one isolated network session per account, with
    /// its own folder on disk (`FR.1.4`).
    WebKitGtk,
    /// Microsoft Edge `WebView2`, on Windows: one shared user-data folder
    /// holding every account's named profile, held open by the one browser
    /// process every account shares (`FR.1.3`, `FR.2.4`).
    WebView2,
}

impl Engine {
    /// The engine this build actually runs. The one place the platform is
    /// read, so [`deletion_steps`] itself stays pure and answerable for both
    /// engines on either platform (architecture rule 14).
    pub(crate) const CURRENT: Self = if cfg!(windows) {
        Self::WebView2
    } else {
        Self::WebKitGtk
    };
}

/// One step of the deletion order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeletionStep {
    /// Ask the engine to delete everything it holds for the account
    /// (`EngineProfile::delete`). Irreversible the moment it completes: this
    /// is the step that makes an attempt unsafe to abandon half way.
    EngineDelete,
    /// Drop the account's view, its holder and its place in the grid.
    DropHolder,
    /// Wait [`DELETE_SETTLE_MILLIS`] for the engine to finish closing the
    /// files it is willing to close.
    Settle,
    /// Remove the account's own `profiles/<id>/` folder through the removal
    /// port.
    RemoveFolder,
}

/// The order one account's deletion runs in, for `engine`.
///
/// The same four steps in the same order on both engines, and that is the
/// point of stating it here rather than only in the code below: the engine
/// step comes **first**, before the view is dropped, and on Windows it has
/// to. Every account's storage lives inside one shared user-data folder the
/// single browser process keeps open for as long as it runs, so the only way
/// to delete exactly one account's data — leaving every sibling's untouched
/// — is to ask the engine while it is still running and the account still
/// has the view its profile can be reached through (task 06's context, and
/// `web_engine/webview2.rs`'s own `EngineProfile::delete`). Linux has no such
/// constraint, and this is also exactly item 11's measured order there, so
/// nothing about the Linux sequence changed to accommodate Windows (code
/// standards rule 18).
pub(crate) const fn deletion_steps(engine: Engine) -> [DeletionStep; 4] {
    match engine {
        Engine::WebKitGtk | Engine::WebView2 => [
            DeletionStep::EngineDelete,
            DeletionStep::DropHolder,
            DeletionStep::Settle,
            DeletionStep::RemoveFolder,
        ],
    }
}

/// Runs [`deletion_steps`] for one account (`FR.21.10`).
///
/// Every step that needs `holder` is skipped when it is `None`, which is what
/// makes a retry after an attempt that already ran them safe to run again
/// (`FR.21.11`). A failed engine deletion stops the sequence and reports the
/// reason, so the dialog shows item 11's error page with `Retry` and `Close`
/// rather than removing the folder of an account whose engine data is still
/// there.
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
    let mut holder = holder;
    let mut outcome = Ok(());

    for step in deletion_steps(Engine::CURRENT) {
        match step {
            DeletionStep::EngineDelete => {
                if let Some(held) = holder.as_ref() {
                    held.profile().delete(held.view()).await.map_err(|error| {
                        ProfileRemovalError {
                            reason: error.reason,
                        }
                    })?;
                }
            }
            DeletionStep::DropHolder => {
                if let Some(mut held) = holder.take() {
                    held.stop();
                    grid.remove_session(&id);
                }
            }
            DeletionStep::Settle => {
                glib::timeout_future(Duration::from_millis(DELETE_SETTLE_MILLIS)).await;
            }
            DeletionStep::RemoveFolder => {
                outcome = remove_folder(&id, &removal).await;
            }
        }
    }

    outcome
}

/// Removes `id`'s own profile folder through `removal`, off the main thread,
/// mapping a panicked task onto the same error the dialog shows for any other
/// failure.
async fn remove_folder(
    id: &SessionId,
    removal: &std::sync::Arc<dyn ProfileRemoval>,
) -> Result<(), ProfileRemovalError> {
    let remove_id = id.clone();
    let removal = std::sync::Arc::clone(removal);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_engines_delete_the_engines_data_then_drop_then_wait_then_remove_the_folder() {
        let expected = [
            DeletionStep::EngineDelete,
            DeletionStep::DropHolder,
            DeletionStep::Settle,
            DeletionStep::RemoveFolder,
        ];

        assert_eq!(
            (
                deletion_steps(Engine::WebKitGtk),
                deletion_steps(Engine::WebView2),
            ),
            (expected, expected),
        );
    }

    #[test]
    fn the_linux_order_is_unchanged_from_item_11s_measured_order() {
        // Item 11 task 08 measured this order on Linux: clear the account's
        // website data, drop its view, wait for the engine to settle, then
        // remove the folder. That Windows reordered nothing on Linux is the
        // whole claim task 06 makes about it.
        assert_eq!(
            deletion_steps(Engine::WebKitGtk),
            deletion_steps(Engine::WebView2),
        );
    }

    #[test]
    fn the_engine_deletes_its_own_data_before_the_view_is_dropped_on_windows() {
        // The shared-folder constraint as a test rather than only a comment:
        // on Windows a profile can be reached only through a live view, so a
        // drop before the engine step would leave no way to delete one
        // account's data without touching its siblings'.
        let steps = deletion_steps(Engine::WebView2);
        let engine_delete = steps
            .iter()
            .position(|step| *step == DeletionStep::EngineDelete);
        let drop_holder = steps
            .iter()
            .position(|step| *step == DeletionStep::DropHolder);

        assert!(engine_delete < drop_holder);
    }

    #[test]
    fn the_current_engine_is_the_one_this_build_runs() {
        let expected = if cfg!(windows) {
            Engine::WebView2
        } else {
            Engine::WebKitGtk
        };

        assert_eq!(Engine::CURRENT, expected);
    }
}
