//! The save-on-change path (task 06): every action that changes the
//! arrangement ends here, and a burst of them becomes one write.
//!
//! Saving is never something the user asks for, so it must never be something
//! they can get wrong. A [`Saver::request`] only rearms a short timer; the
//! write happens once activity stops, off the GTK main context, and a failure
//! is logged and shown in the message strip rather than dropped (code standards
//! rules 14, 15).

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{WorkspaceList, WorkspaceStore, WorkspaceWriteError};

use crate::message_strip::MessageStrip;

/// How long a save request waits before it writes, so a drag through three
/// arrangements in two seconds collapses into one file on disk (code standards
/// rule 5). Each new request during the wait restarts it.
const SAVE_DEBOUNCE_MILLIS: u64 = 400;

struct Inner {
    store: Arc<dyn WorkspaceStore>,
    strip: MessageStrip,
    /// The workspace list the pending write will save — replaced by each
    /// request, so only the latest state reaches disk.
    latest: RefCell<Option<WorkspaceList>>,
    /// The armed debounce timer, if one is pending.
    timer: RefCell<Option<glib::SourceId>>,
}

/// Collapses a burst of change notifications into a single background write.
pub(crate) struct Saver(Rc<Inner>);

impl Saver {
    /// Builds a saver that writes through `store` and reports failures in
    /// `strip`.
    pub(crate) fn new(store: Arc<dyn WorkspaceStore>, strip: MessageStrip) -> Self {
        Self(Rc::new(Inner {
            store,
            strip,
            latest: RefCell::new(None),
            timer: RefCell::new(None),
        }))
    }

    /// Records that `workspaces` should be saved and (re)arms the debounce
    /// timer. Cheap: a caller in doubt calls it.
    pub(crate) fn request(&self, workspaces: WorkspaceList) {
        self.0.latest.replace(Some(workspaces));

        if let Some(timer) = self.0.timer.borrow_mut().take() {
            timer.remove();
        }

        let inner = Rc::downgrade(&self.0);
        let timer =
            glib::timeout_add_local_once(Duration::from_millis(SAVE_DEBOUNCE_MILLIS), move || {
                if let Some(inner) = inner.upgrade() {
                    Saver(inner).write_in_background();
                }
            });
        self.0.timer.replace(Some(timer));
    }

    /// Writes the pending workspace now, synchronously, and waits for it. The
    /// one place a write is allowed to block: the window's close request, where
    /// nothing else will trigger the write for a change made a moment before
    /// quitting.
    pub(crate) fn flush(&self) {
        if let Some(timer) = self.0.timer.borrow_mut().take() {
            timer.remove();
        }
        let Some(workspace) = self.0.latest.borrow_mut().take() else {
            return;
        };
        if let Err(error) = self.0.store.write(&workspace) {
            tracing::warn!(error = %error, "the final workspace save on close failed");
        } else {
            tracing::debug!("workspace flushed on close");
        }
    }

    fn write_in_background(&self) {
        self.0.timer.borrow_mut().take();
        let Some(workspace) = self.0.latest.borrow_mut().take() else {
            return;
        };

        let store = Arc::clone(&self.0.store);
        let strip = ObjectExt::downgrade(&self.0.strip);
        glib::spawn_future_local(async move {
            let outcome = gio::spawn_blocking(move || store.write(&workspace)).await;
            report(&strip, outcome);
        });
    }
}

/// Handles the result the background write comes back with: a success is a
/// debug line, a failed write is logged with its reason and shown in the strip,
/// and a task that panicked is an error. Never a silently dropped `Result`.
fn report(
    strip: &glib::WeakRef<MessageStrip>,
    outcome: Result<Result<(), WorkspaceWriteError>, Box<dyn std::any::Any + Send>>,
) {
    match outcome {
        Ok(Ok(())) => tracing::debug!("workspace saved"),
        Ok(Err(error)) => {
            tracing::warn!(reason = %error.reason, "the workspace could not be saved");
            if let Some(strip) = strip.upgrade() {
                strip.show(&format!(
                    "The arrangement could not be saved: {}.",
                    error.reason
                ));
            }
        }
        Err(_) => {
            tracing::error!("the workspace save task panicked");
        }
    }
}
