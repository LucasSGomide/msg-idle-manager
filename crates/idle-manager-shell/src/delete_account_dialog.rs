//! The modal that confirms, runs and reports one account's deletion.
//!
//! It creates nothing itself and removes nothing itself: on confirm or retry
//! it reports through [`DeleteAccountDialog::connect_confirmed`] /
//! [`DeleteAccountDialog::connect_retry`] and waits to be told what happened
//! through [`DeleteAccountDialog::show_working`] / [`DeleteAccountDialog::show_failed`].
//! The window runs [`crate::account_deletion`]'s sequence and decides what
//! each outcome means (architecture rule 8, item 11 task 08).

mod imp;

use std::path::Path;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

glib::wrapper! {
    /// A small modal over the main window, its three pages a `confirm` /
    /// `working` / `failed` sequence. Build one with
    /// [`DeleteAccountDialog::new`]; present it with
    /// [`gtk::prelude::GtkWindowExt`] after setting its transient parent.
    pub struct DeleteAccountDialog(ObjectSubclass<imp::DeleteAccountDialog>)
        @extends gtk::Widget, gtk::Window,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
                    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl DeleteAccountDialog {
    /// Builds the dialog on its `confirm` page, its sentence naming `name`.
    #[must_use]
    pub fn new(name: &str) -> Self {
        let dialog: Self = glib::Object::new();
        dialog.imp().configure(name);
        dialog
    }

    /// Registers `handler` to run when the red `Delete account` button is
    /// pressed. The dialog switches to its `working` page itself; the
    /// handler starts the actual deletion sequence. Replaces any previous
    /// handler.
    pub fn connect_confirmed(&self, handler: impl Fn() + 'static) {
        self.imp().on_confirmed.replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run when `Retry` is pressed on the `failed`
    /// page. The dialog switches to its `working` page itself. Replaces any
    /// previous handler.
    pub fn connect_retry(&self, handler: impl Fn() + 'static) {
        self.imp().on_retry.replace(Some(Box::new(handler)));
    }

    /// Registers `handler` to run when `Close` is pressed on the `failed`
    /// page, or the window is otherwise closed while it shows — never while
    /// `working` shows, since that page accepts no close at all. The window
    /// uses this to leave the account parked and ready to be deleted again
    /// (`FR.21.9`). Replaces any previous handler.
    pub fn connect_closed_after_failure(&self, handler: impl Fn() + 'static) {
        self.imp()
            .on_closed_after_failure
            .replace(Some(Box::new(handler)));
    }

    /// Switches to the `working` page: a spinner and "Deleting `name`…",
    /// with no buttons. From here the window's close button and Escape do
    /// nothing until [`DeleteAccountDialog::show_failed`] runs or the dialog
    /// is closed on success (`FR.21.9`).
    pub fn show_working(&self, name: &str) {
        self.imp().show_working(name);
    }

    /// Switches to the `failed` page: "Couldn't finish deleting `name`.",
    /// `reason` on one dim line and `folder` on a second, selectable one,
    /// with `Retry` and `Close`.
    pub fn show_failed(&self, name: &str, reason: &str, folder: &Path) {
        self.imp().show_failed(name, reason, folder);
    }

    /// Closes the window because the deletion actually succeeded. Never plain
    /// [`gtk::prelude::GtkWindowExt::close`]: the `working` page refuses every
    /// other close, including this one unless it goes through here
    /// (`FR.21.9`).
    pub fn close_on_success(&self) {
        self.imp().close_on_success();
    }
}
