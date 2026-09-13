//! The index down the leading edge of the window: one row per account, in the
//! order they were added, whether or not each one currently holds a slot.
//!
//! It renders from [`SessionBook`] state and, on row activation, reports the
//! account's id back through [`SessionSidebar::connect_row_activated`]; it
//! decides nothing itself (architecture rule 8).

mod imp;
mod row;

use std::sync::Arc;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{MemoryProbe, SessionBook, SessionId};

glib::wrapper! {
    /// The account list widget, built from `ui/session-sidebar.ui`. Build one
    /// with [`SessionSidebar::new`], feed it [`SessionSidebar::sync`] whenever
    /// the book changes, and install a handler with
    /// [`SessionSidebar::connect_row_activated`].
    pub struct SessionSidebar(ObjectSubclass<imp::SessionSidebar>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl SessionSidebar {
    /// Builds an empty sidebar showing its empty-state line.
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Starts the memory footer sampling `probe`. Call once, when the ports are
    /// attached; the footer runs each sample off the main context (architecture
    /// rule 10).
    pub fn start_memory_sampling(&self, probe: Arc<dyn MemoryProbe>) {
        self.imp().start_memory_sampling(probe);
    }

    /// Redraws every row from `book`: one row per account in add order, each
    /// row's name and trailing place from the account's visibility, and the
    /// focused-slot account's row marked as current. Shows the empty-state line
    /// in place of the list when the book holds no accounts.
    pub fn sync(&self, book: &SessionBook) {
        self.imp().sync(book);
    }

    /// Registers `handler` to run with an account's id when its row is
    /// activated. Replaces any previous handler.
    pub fn connect_row_activated(&self, handler: impl Fn(SessionId) + 'static) {
        self.imp().on_activated.replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with an account's id when its row's
    /// park/start button is pressed. The window decides which direction the
    /// press means from the account's current liveness; the row carries no
    /// state of its own (architecture rule 8). Replaces any previous handler.
    pub fn connect_parking_toggled(&self, handler: impl Fn(SessionId) + 'static) {
        self.imp()
            .on_parking_toggled
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with an account's id and the value asked
    /// for when its row menu's "Keep running when hidden" item is chosen.
    /// The menu reports only the intent it was given; whether the flag
    /// actually changes, and what the shell does in response, is the
    /// window's to decide (architecture rule 8). Replaces any previous
    /// handler.
    pub fn connect_keep_awake_toggled(&self, handler: impl Fn(SessionId, bool) + 'static) {
        self.imp()
            .on_keep_awake_toggled
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with an account's id when its row menu's
    /// `Rename…` item is chosen. The row decides nothing else: the window
    /// opens the rename dialog and the book decides whether a typed name is
    /// stored (architecture rule 8). Replaces any previous handler.
    pub fn connect_rename_requested(&self, handler: impl Fn(SessionId) + 'static) {
        self.imp()
            .on_rename_requested
            .replace(Some(Box::new(handler)));
    }
}

impl Default for SessionSidebar {
    fn default() -> Self {
        Self::new()
    }
}
