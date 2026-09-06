//! The index down the leading edge of the window: one row per account, in the
//! order they were added, whether or not each one currently holds a slot.
//!
//! It renders from [`SessionBook`] state and, on row activation, reports the
//! account's id back through [`SessionSidebar::connect_row_activated`]; it
//! decides nothing itself (architecture rule 8).

mod imp;
mod row;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{SessionBook, SessionId};

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
}

impl Default for SessionSidebar {
    fn default() -> Self {
        Self::new()
    }
}
