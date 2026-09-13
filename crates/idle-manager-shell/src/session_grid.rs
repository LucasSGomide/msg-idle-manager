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

/// The private payload a place's grip drag carries (`FR.14.5`): a registered
/// [`glib::Boxed`] type wrapping the dragged account's [`SessionId`], offered
/// through [`gtk::gdk::ContentProvider::for_value`] and never a plain string —
/// a text-typed drag is exactly what a web page accepts and would paste into
/// its own text box. Exposed from this module so the drop-handling slice
/// accepts exactly this type.
#[derive(Clone, Debug, PartialEq, Eq, glib::Boxed)]
#[boxed_type(name = "IdleManagerDraggedAccount")]
pub struct DraggedAccount(SessionId);

impl DraggedAccount {
    /// Wraps `session` for a grip's drag source to offer as its content.
    #[must_use]
    pub fn new(session: SessionId) -> Self {
        Self(session)
    }

    /// The dragged account's id.
    #[must_use]
    pub fn session_id(&self) -> &SessionId {
        &self.0
    }
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

    /// Registers a restored `session` that has no view yet: its slot shows the
    /// placeholder panel, its line and button derived from the account's
    /// liveness on the next [`SessionGrid::sync`]. The start queue hands it a
    /// view later with [`SessionGrid::attach_view`].
    pub fn add_dormant_session(&self, session: &SessionId, display_name: &str) {
        self.imp().add_dormant_session(session, display_name);
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

    /// Drops a parked account's view out of its slot, leaving the name cover
    /// showing. The slot stays the account's. A no-op if the grid has no view
    /// for `session`.
    pub fn release_view(&self, session: &SessionId) {
        self.imp().release_view(session);
    }

    /// Puts a restarted account's new view back into the slot it still holds,
    /// behind the placeholder panel, which stays up reading `Starting` until
    /// the page paints. A no-op if the grid has no entry for `session`.
    pub fn attach_view(&self, session: &SessionId, view: &WebView) {
        self.imp().attach_view(session, view);
    }

    /// Shows `figure` as the transient readout over `session`'s place, updating
    /// whatever is already there and rearming its fade timer. A no-op if the
    /// grid has no place for `session` (`FR.11.6`).
    pub fn flash_zoom_readout(&self, session: &SessionId, figure: &str) {
        self.imp().flash_zoom_readout(session, figure);
    }

    /// Registers `handler` to run with an account's id when that account's slot
    /// placeholder button is pressed — the same start intent the sidebar row's
    /// button sends. Replaces any previous handler.
    pub fn connect_start_requested(&self, handler: impl Fn(SessionId) + 'static) {
        self.imp()
            .on_start_requested
            .replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run whenever the user clicks a slot to focus it.
    pub fn connect_slot_focused(&self, handler: impl Fn(SlotId) + 'static) {
        self.imp().on_slot_focused.replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run with an account's id and the wheel's vertical
    /// delta when `Ctrl` and the wheel turn over that account's place. Replaces
    /// any previous handler (`FR.11.3`).
    pub fn connect_zoom_scrolled(&self, handler: impl Fn(SessionId, f64) + 'static) {
        self.imp().on_zoom_scrolled.replace(Some(Box::new(handler)));
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

#[cfg(test)]
mod tests {
    use super::*;
    use glib::prelude::*;

    #[test]
    fn a_dragged_account_round_trips_through_a_value_as_a_boxed_type_not_a_string() {
        let payload = DraggedAccount::new(SessionId::new("session-0007"));

        let value = payload.to_value();
        let read_back = value
            .get::<DraggedAccount>()
            .expect("a DraggedAccount value reads back as one");

        assert_eq!(read_back.session_id(), &SessionId::new("session-0007"));
        assert_ne!(value.type_(), glib::Type::STRING);
    }
}
