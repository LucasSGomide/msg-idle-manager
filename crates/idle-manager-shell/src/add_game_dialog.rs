//! The modal for adding an account: a chooser of the games the application
//! knows about, with an escape hatch for one it does not.
//!
//! It creates nothing itself. On confirm it reports what the user chose — a
//! preset and an account name, or a typed name and address — through
//! [`AddGameDialog::connect_confirmed`] and closes; the book decides what that
//! means (architecture rule 8).

mod imp;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{Preset, PresetCatalogue};

glib::wrapper! {
    /// A two-stage modal over the main window: a list of catalogue games plus a
    /// "Something else" row, then a short form for the chosen path.
    pub struct AddGameDialog(ObjectSubclass<imp::AddGameDialog>)
        @extends gtk::Widget, gtk::Window,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
                    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

/// What the user settled on when they pressed "Add".
#[derive(Debug, Clone)]
pub enum Confirmed {
    /// A game from the catalogue, plus the name to give this account.
    Preset {
        /// The chosen game, its values copied whole.
        preset: Preset,
        /// The account name the user typed.
        account_name: String,
    },
    /// The escape hatch: a game the catalogue has no file for.
    Custom {
        /// The account name the user typed.
        name: String,
        /// The address the user typed.
        address: String,
    },
}

impl AddGameDialog {
    /// Builds the dialog and reads `catalogue` now, so a file added by hand
    /// since the last open shows up. Present it with
    /// [`gtk::prelude::GtkWindowExt`] after setting its transient parent.
    ///
    /// The catalogue is a trait object, so the dialog never learns the entries
    /// are files on disk (architecture rule 3); it is read once here, not held,
    /// because the next open builds a fresh dialog that reads again.
    #[must_use]
    pub fn new(catalogue: &dyn PresetCatalogue) -> Self {
        let dialog: Self = glib::Object::new();
        dialog.imp().load_catalogue(catalogue);
        dialog
    }

    /// Registers `handler` to run with the user's choice when they confirm. It
    /// runs at most once; the dialog closes immediately after.
    pub fn connect_confirmed(&self, handler: impl Fn(&Confirmed) + 'static) {
        self.imp().on_confirmed.replace(Some(Box::new(handler)));
    }
}
