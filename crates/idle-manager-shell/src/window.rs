//! The application window: the header bar's controls and the content area that
//! shows either the empty state or the session grid.

mod imp;

// `window/imp.rs` keeps its own module private, as every other widget's does
// (code standards rule 12); this one type is re-exported, Windows only,
// because `web_engine/webview2/host/imp.rs`'s ipc handler needs to name it
// (roadmap item 12 task 03), the same reason `Window::handle_shortcut_key`
// and `Window::apply_zoom_step` are `pub(crate)` methods reachable through
// `.imp()` from that sibling module.
#[cfg(windows)]
pub(crate) use imp::ZoomStep;

use std::rc::Rc;
use std::sync::Arc;

use gtk::gio;
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{
    MemoryProbe, PresetCatalogue, ProfileLocator, ProfileRemoval, WorkspaceList,
    WorkspaceReadError, WorkspaceStore, ZoomMemory,
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
    /// What the application currently costs in memory, for the sidebar footer
    /// (item 05).
    pub probe: Arc<dyn MemoryProbe>,
    /// Removes one account's profile folder (item 11 task 08). `Arc`, not
    /// `Rc`, like `store` above: the deletion sequence calls it through
    /// `gio::spawn_blocking`, which needs `Send`.
    pub removal: Arc<dyn ProfileRemoval>,
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
        read_outcome: Result<Option<WorkspaceList>, WorkspaceReadError>,
    ) -> Self {
        let window: Self = glib::Object::builder().property("application", app).build();
        window.imp().attach_ports(ports, read_outcome);
        window
    }
}
