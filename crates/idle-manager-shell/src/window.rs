//! The application window: the header bar's controls and the content area that
//! shows either the empty state or the session grid.

mod imp;

use std::rc::Rc;

use gtk::gio;
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{
    PresetCatalogue, ProfileLocator, Workspace, WorkspaceReadError, WorkspaceStore,
};

glib::wrapper! {
    /// The single top-level window, built from `ui/window.ui`.
    ///
    /// Construct one per [`gtk::Application`] activation with [`Window::new`].
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl Window {
    /// Builds the window for `app`, wired to `locator` for each account's
    /// profile directories, `catalogue` for the add-game dialog's game list,
    /// and `store` for saving the workspace. `read_outcome` is what the
    /// composition root read from the store before the window was built: a
    /// workspace to restore, `None` for a first run, or a failure the window
    /// reports in its message strip.
    #[must_use]
    pub fn new(
        app: &gtk::Application,
        locator: Rc<dyn ProfileLocator>,
        catalogue: Rc<dyn PresetCatalogue>,
        store: Rc<dyn WorkspaceStore>,
        read_outcome: Result<Option<Workspace>, WorkspaceReadError>,
    ) -> Self {
        let window: Self = glib::Object::builder().property("application", app).build();
        window
            .imp()
            .attach_ports(locator, catalogue, store, read_outcome);
        window
    }
}
