//! The grid that holds one web view per session and places each one in the
//! slot the core assigned it — or, for an off-grid session, just outside its
//! own bounds where `set_overflow(Hidden)` clips it away without unrealising
//! it.

mod imp;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{SessionBook, SessionId, SlotId};
use webkit6::WebView;

glib::wrapper! {
    /// A container widget: every child is an account's web view, allocated the
    /// rectangle of the slot the core placed it in. Build one with
    /// [`SessionGrid::new`], feed it sessions with [`SessionGrid::add_session`],
    /// and call [`SessionGrid::sync`] whenever the book changes.
    pub struct SessionGrid(ObjectSubclass<imp::SessionGrid>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

glib::wrapper! {
    /// The layout manager [`SessionGrid`] installs on itself: it allocates each
    /// child either its slot rectangle or a rectangle outside the grid.
    pub struct SlotLayout(ObjectSubclass<imp::SlotLayout>)
        @extends gtk::LayoutManager;
}

impl SessionGrid {
    /// Builds an empty grid arranged for the single-slot layout.
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Adds `view` as the slot content for `session`, showing `display_name`
    /// centred until the page paints. The view is placed off-grid until the
    /// next [`SessionGrid::sync`].
    pub fn add_session(&self, session: &SessionId, display_name: &str, view: &WebView) {
        self.imp().add_session(session, display_name, view);
    }

    /// Re-places every known session from `book` and redraws. A session in the
    /// book with no view here is skipped; its slot simply stays empty.
    pub fn sync(&self, book: &SessionBook) {
        self.imp().sync(book);
    }

    /// Reloads the page in the currently focused slot. A no-op when that slot
    /// is empty or holds an off-grid session.
    pub fn reload_focused(&self) {
        self.imp().reload_focused();
    }

    /// Registers `handler` to run whenever the user clicks a slot to focus it.
    pub fn connect_slot_focused(&self, handler: impl Fn(SlotId) + 'static) {
        self.imp().on_slot_focused.replace(Some(Box::new(handler)));
    }
}

impl Default for SessionGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl SlotLayout {
    fn new() -> Self {
        glib::Object::new()
    }
}

impl Default for SlotLayout {
    fn default() -> Self {
        Self::new()
    }
}
