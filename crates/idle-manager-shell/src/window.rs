//! The application window: the header bar's controls and the content area that
//! shows either the empty state or the session grid.

mod imp;

use std::rc::Rc;

use gtk::gio;
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{PresetCatalogue, ProfileLocator};

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
    /// Builds the window for `app`, wired to `locator` for each new account's
    /// profile directories and to `catalogue` for the add-game dialog's game
    /// list.
    #[must_use]
    pub fn new(
        app: &gtk::Application,
        locator: Rc<dyn ProfileLocator>,
        catalogue: Rc<dyn PresetCatalogue>,
    ) -> Self {
        let window: Self = glib::Object::builder().property("application", app).build();
        window.imp().attach_ports(locator, catalogue);
        window
    }
}
