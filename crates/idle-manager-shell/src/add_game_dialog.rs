//! The modal that asks for the two things the application cannot work out for
//! itself: a display name and a starting address.

mod imp;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

glib::wrapper! {
    /// A two-field modal over the main window. It creates nothing itself: on
    /// confirm it reports the name and address through
    /// [`AddGameDialog::connect_confirmed`] and closes (architecture rule 8).
    pub struct AddGameDialog(ObjectSubclass<imp::AddGameDialog>)
        @extends gtk::Widget, gtk::Window,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
                    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl AddGameDialog {
    /// Builds the dialog. Present it with [`gtk::prelude::GtkWindowExt`] after
    /// setting its transient parent.
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Registers `handler` to run with the entered name and address when the
    /// user confirms. It runs at most once; the dialog closes immediately
    /// after.
    pub fn connect_confirmed(&self, handler: impl Fn(&str, &str) + 'static) {
        self.imp().on_confirmed.replace(Some(Box::new(handler)));
    }
}

impl Default for AddGameDialog {
    fn default() -> Self {
        Self::new()
    }
}
