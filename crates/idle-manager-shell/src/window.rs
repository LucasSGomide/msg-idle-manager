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
    MemoryProbe, PhoneLink, PresetCatalogue, ProfileLocator, ProfileRemoval, RemoteIntent,
    WorkspaceList, WorkspaceReadError, WorkspaceStore, ZoomMemory,
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
    /// The phone's way in (roadmap item 13 task 06). `None` when no link
    /// exists at all — a second activation after the first took it; a
    /// server that could not start still hands over a link, one whose status
    /// says so, so the phone dialog can name the reason.
    pub phone: Option<PhonePorts>,
}

/// The two halves of the phone link the composition root built: the handle
/// the window publishes state and frames through, and the channel the
/// phone's intents arrive on (architecture rule 3).
#[derive(Debug)]
pub struct PhonePorts {
    /// Publishes state and frames, offers enrolment, answers the status.
    pub link: Arc<dyn PhoneLink>,
    /// What the phone asked for, in order. A closed channel — the sender
    /// dropped — ends the window's loop at once and is how a server that
    /// never started hands over nothing.
    pub intents: async_channel::Receiver<RemoteIntent>,
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
