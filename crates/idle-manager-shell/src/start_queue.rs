//! The start queue: brings restored accounts back one at a time (`FR.8.2`).
//!
//! Starting six games together would spike memory as six rendering engines come
//! up, saturate the connection as six games fetch their assets, and leave the
//! window unusable for as long as it all takes. This starts the first account,
//! and only once its page has settled — or it has waited long enough — begins
//! the next.
//!
//! It owns no policy about *which* accounts come back; that came from
//! `SessionBook::start_order`. It owns only *when* the next one may begin. Each
//! start goes through `Window`'s existing start path unchanged, so restoration
//! adds no second way to bring an account up.

use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;
use webkit6::prelude::*;
use webkit6::{LoadEvent, WebView};

use idle_manager_core::SessionId;

use crate::window::Window;

/// How long the queue waits for one account's page to report itself loaded
/// before it moves on regardless.
///
/// A fallback, not the normal path: a healthy page fires load-finished in a few
/// seconds and the queue advances then. This bounds a game that never finishes
/// loading — a bad connection, a server having a bad day — so it cannot strand
/// the accounts behind it (`FR.8.2`). The value is confirmed against a real
/// game in this item's `test-script.md` (code standards rule 5).
const LOAD_SETTLE_TIMEOUT_SECS: u64 = 30;

struct Inner {
    window: glib::WeakRef<Window>,
    pending: RefCell<Vec<SessionId>>,
}

/// Drains a list of restored identifiers, starting one at a time. Held by the
/// window only while a restore is in progress; drops cleanly when the window
/// goes away mid-drain.
pub(crate) struct StartQueue(Rc<Inner>);

impl StartQueue {
    /// Starts draining `order` against `window`. The first account begins on
    /// the next main-loop turn, so the window is presented first.
    pub(crate) fn begin(window: glib::WeakRef<Window>, order: Vec<SessionId>) -> Self {
        let inner = Rc::new(Inner {
            window,
            pending: RefCell::new(order),
        });

        let deferred = Rc::downgrade(&inner);
        glib::idle_add_local_once(move || {
            if let Some(inner) = deferred.upgrade() {
                StartQueue(inner).advance();
            }
        });

        Self(inner)
    }

    /// Starts the next account whose turn has come, or ends the restore.
    fn advance(&self) {
        let Some(window) = self.0.window.upgrade() else {
            // The window went away mid-drain: end the restore, start nothing
            // further — the same guard `start_session` and `finish_starting`
            // already use.
            tracing::debug!("start queue: the window is gone; ending the restore");
            return;
        };

        let pending = std::mem::take(&mut *self.0.pending.borrow_mut());
        let Some((id, rest)) = next_up(&pending, |id| window.imp().is_queued(id)) else {
            tracing::info!("start queue: nothing queued; restoration finished");
            return;
        };
        *self.0.pending.borrow_mut() = rest;

        let Some(view) = window.imp().start_session(&id) else {
            tracing::warn!(session = %id, "start queue: no holder to start; skipping");
            self.advance();
            return;
        };

        tracing::debug!(session = %id, "start queue: started; waiting for it to settle");
        self.arm_next(&view);
    }

    /// Advances on `view`'s load-finished signal or on
    /// [`LOAD_SETTLE_TIMEOUT_SECS`], whichever comes first. A one-shot guard
    /// makes sure only the first of the two moves the queue on.
    fn arm_next(&self, view: &WebView) {
        let fired = Rc::new(Cell::new(false));

        let on_settled = self.0.downgrade_handler();
        let guard = Rc::clone(&fired);
        view.connect_load_changed(move |_, event| {
            if !matches!(event, LoadEvent::Finished) || guard.replace(true) {
                return;
            }
            on_settled.run();
        });

        let on_timeout = self.0.downgrade_handler();
        let guard = fired;
        glib::timeout_add_local_once(Duration::from_secs(LOAD_SETTLE_TIMEOUT_SECS), move || {
            if guard.replace(true) {
                return;
            }
            tracing::debug!("start queue: a page did not settle in time; moving on");
            on_timeout.run();
        });
    }
}

impl Inner {
    fn downgrade_handler(self: &Rc<Self>) -> AdvanceHandler {
        AdvanceHandler(Rc::downgrade(self))
    }
}

/// A weak handle back to the queue that runs one more `advance` if the queue
/// is still alive.
struct AdvanceHandler(Weak<Inner>);

impl AdvanceHandler {
    fn run(&self) {
        if let Some(inner) = self.0.upgrade() {
            StartQueue(inner).advance();
        }
    }
}

/// The next identifier the queue should start — the first in `pending` that
/// `is_queued` still accepts — paired with the rest to keep. `None` when none
/// remain, which is the queue reporting itself finished.
///
/// An identifier the book no longer reports as queued is skipped, not started:
/// a state that changed while the queue drained must never be overwritten by a
/// start nobody asked for.
fn next_up(
    pending: &[SessionId],
    is_queued: impl Fn(&SessionId) -> bool,
) -> Option<(SessionId, Vec<SessionId>)> {
    let next = pending.iter().position(is_queued)?;
    Some((pending[next].clone(), pending[next + 1..].to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(names: &[&str]) -> Vec<SessionId> {
        names.iter().map(|name| SessionId::new(*name)).collect()
    }

    #[test]
    fn the_next_up_decision_skips_an_identifier_no_longer_queued() {
        let pending = ids(&["a", "b", "c"]);
        let still_queued = ids(&["b", "c"]);

        let (next, rest) =
            next_up(&pending, |id| still_queued.contains(id)).expect("b is still queued");

        assert_eq!(
            (
                next.as_str(),
                rest.iter().map(SessionId::as_str).collect::<Vec<_>>()
            ),
            ("b", vec!["c"]),
        );
    }

    #[test]
    fn the_next_up_decision_reports_finished_when_none_remain() {
        let pending = ids(&["a", "b"]);

        let outcome = next_up(&pending, |_| false);

        assert!(outcome.is_none());
    }
}
