//! The application window: the header bar's controls and the content area that
//! shows either the empty state or the session grid.

mod imp;

use std::rc::Rc;
use std::sync::Arc;

use gtk::gio;
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{
    PresetCatalogue, ProfileLocator, Workspace, WorkspaceReadError, WorkspaceStore, ZoomMemory,
};

/// The ports the window runs against, built once by the composition root and
/// handed over as one value (architecture rule 3).
#[derive(Debug)]
pub struct WindowPorts {
    /// Each account's private profile directories on disk.
    pub locator: Rc<dyn ProfileLocator>,
    /// The game catalogue the add-game dialog reads.
    pub catalogue: Rc<dyn PresetCatalogue>,
    /// The workspace store the arrangement is saved through.
    pub store: Arc<dyn WorkspaceStore>,
    /// Each account's remembered zoom sizes (item 09).
    pub zoom_memory: Rc<dyn ZoomMemory>,
}

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
    /// and `ports` for profile directories, the game list, workspace saving and
    /// each account's remembered zoom — the shell never learns a file is behind
    /// any of them. `read_outcome` is what the composition root read from the
    /// store before the window was built: a workspace to restore, `None` for a
    /// first run, or a failure the window reports in its message strip.
    #[must_use]
    pub fn new(
        app: &gtk::Application,
        ports: WindowPorts,
        read_outcome: Result<Option<Workspace>, WorkspaceReadError>,
    ) -> Self {
        let window: Self = glib::Object::builder().property("application", app).build();
        window.imp().attach_ports(ports, read_outcome);
        window
    }
}
