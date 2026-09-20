//! One heading in the sidebar tree: a workspace's identity and name, plus the
//! child store of [`super::row::Row`]s the tree expander reveals beneath it.

mod imp;

use gtk::gio;
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::WorkspaceId;

use super::row::Row;

glib::wrapper! {
    /// A tree root item: one per workspace, always holding at least one child
    /// — an empty workspace's child store holds the dim "No accounts"
    /// placeholder, so the accordion's expander arrow always has something to
    /// reveal (roadmap item 11 wireframe `sidebar-tree.md`).
    pub struct WorkspaceRow(ObjectSubclass<imp::WorkspaceRow>);
}

impl WorkspaceRow {
    /// A heading for the workspace named `id`/`name`, holding `accounts` as
    /// its children in the order given — [`Row::placeholder`] is inserted in
    /// their place when `accounts` is empty. `can_park_all` and
    /// `can_start_all` are
    /// [`idle_manager_core::WorkspaceBook::can_park_all`] and
    /// [`idle_manager_core::WorkspaceBook::can_start_all`]'s answers for this
    /// workspace, read once by the caller and carried as plain properties
    /// (`FR.24.3`).
    pub(crate) fn new(
        id: &WorkspaceId,
        name: &str,
        accounts: Vec<Row>,
        can_park_all: bool,
        can_start_all: bool,
    ) -> Self {
        let children = gio::ListStore::new::<Row>();
        if accounts.is_empty() {
            children.append(&Row::placeholder());
        } else {
            for row in accounts {
                children.append(&row);
            }
        }

        let workspace: Self = glib::Object::builder()
            .property("id", id.as_str())
            .property("name", name)
            .property("can-park-all", can_park_all)
            .property("can-start-all", can_start_all)
            .build();
        workspace.imp().set_children(children);
        workspace
    }

    /// This heading's children — real account rows, or the one placeholder
    /// leaf when it has none. What [`super::imp::create_model_func`] hands
    /// back for this row.
    pub(crate) fn children(&self) -> gio::ListStore {
        self.imp().children()
    }
}
