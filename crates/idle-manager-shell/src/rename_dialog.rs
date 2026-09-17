//! The modal for naming something with a single text field: an account's
//! rename, or a workspace's create-or-rename (item 11 task 06).
//!
//! It creates nothing itself. On confirm it reports the typed name through
//! [`RenameDialog::connect_confirmed`] and closes; the caller's own name rule
//! decides whether that name is stored (architecture rule 8).

mod imp;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

/// What a candidate name is, as the caller's own rule sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NameCheck {
    /// The name is ready to confirm.
    Ok,
    /// The name trims to nothing.
    Empty,
    /// The name matches another workspace's, ignoring case. Only a workspace
    /// name check ever answers this — `account_name` never does.
    Taken,
}

glib::wrapper! {
    /// A small modal over the main window: a title, one text field, and a
    /// confirm button whose label and enabling rule the caller supplies.
    /// Build one with [`RenameDialog::new`]; present it with
    /// [`gtk::prelude::GtkWindowExt`] after setting its transient parent.
    pub struct RenameDialog(ObjectSubclass<imp::RenameDialog>)
        @extends gtk::Widget, gtk::Window,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
                    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl RenameDialog {
    /// Builds the dialog titled `title`, its confirm button labelled
    /// `confirm_label`, pre-filled with `current_name` and fully selected so
    /// typing replaces it. `check` runs against the field's current text on
    /// every change and on confirm: the confirm button is sensitive only
    /// while it answers [`NameCheck::Ok`], and the dim "Another workspace has
    /// this name." line (design rule 8) shows only while it answers
    /// [`NameCheck::Taken`].
    #[must_use]
    pub fn new(
        title: &str,
        confirm_label: &str,
        current_name: &str,
        check: impl Fn(&str) -> NameCheck + 'static,
    ) -> Self {
        let dialog: Self = glib::Object::new();
        dialog
            .imp()
            .configure(title, confirm_label, current_name, Box::new(check));
        dialog
    }

    /// Registers `handler` to run with the typed, trimmed name when the user
    /// confirms. It runs at most once; the dialog closes immediately after.
    pub fn connect_confirmed(&self, handler: impl Fn(&str) + 'static) {
        self.imp().on_confirmed.replace(Some(Box::new(handler)));
    }
}
