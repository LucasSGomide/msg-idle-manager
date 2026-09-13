//! The modal for renaming an account: the add-game dialog's details stage cut
//! down to one field.
//!
//! It creates nothing itself. On confirm it reports the typed name through
//! [`RenameDialog::connect_confirmed`] and closes; the book decides whether
//! that name is stored (architecture rule 8).

mod imp;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

glib::wrapper! {
    /// A small modal over the main window: one text field holding the
    /// account's current name, `Cancel` and `Rename`.
    pub struct RenameDialog(ObjectSubclass<imp::RenameDialog>)
        @extends gtk::Widget, gtk::Window,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
                    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl RenameDialog {
    /// Builds the dialog pre-filled with `current_name`, all of it selected so
    /// typing replaces it. Present it with [`gtk::prelude::GtkWindowExt`]
    /// after setting its transient parent.
    #[must_use]
    pub fn new(current_name: &str) -> Self {
        let dialog: Self = glib::Object::new();
        dialog.imp().set_current_name(current_name);
        dialog
    }

    /// Registers `handler` to run with the typed name when the user confirms.
    /// It runs at most once; the dialog closes immediately after.
    pub fn connect_confirmed(&self, handler: impl Fn(&str) + 'static) {
        self.imp().on_confirmed.replace(Some(Box::new(handler)));
    }
}
