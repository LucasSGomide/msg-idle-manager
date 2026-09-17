//! The heading's backing object: its identity and name as properties, plus
//! its child store as a plain field — the child store is structural, not a
//! fact a bound widget reads by name (architecture rule 12).

use std::cell::{OnceCell, RefCell};

use gtk::gio;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

/// The property-backed object for [`super::WorkspaceRow`].
#[derive(glib::Properties, Default)]
#[properties(wrapper_type = super::WorkspaceRow)]
pub struct WorkspaceRow {
    /// The workspace's minted identifier, so a heading action can name it to
    /// the domain.
    #[property(get, set)]
    id: RefCell<String>,
    /// The name shown on the heading.
    #[property(get, set)]
    name: RefCell<String>,
    /// This workspace's children, built once at construction
    /// ([`super::WorkspaceRow::new`]) and handed out by
    /// [`super::WorkspaceRow::children`].
    children: OnceCell<gio::ListStore>,
}

impl std::fmt::Debug for WorkspaceRow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WorkspaceRow")
            .field("id", &self.id.borrow())
            .finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for WorkspaceRow {
    const NAME: &'static str = "IdleManagerWorkspaceRow";
    type Type = super::WorkspaceRow;
}

#[glib::derived_properties]
impl ObjectImpl for WorkspaceRow {}

impl WorkspaceRow {
    pub(super) fn set_children(&self, children: gio::ListStore) {
        self.children
            .set(children)
            .expect("children set once, right after construction");
    }

    pub(super) fn children(&self) -> gio::ListStore {
        self.children
            .get()
            .expect("children set right after construction")
            .clone()
    }
}
