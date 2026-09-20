//! The modal where the one phone is enrolled, seen and un-enrolled (roadmap
//! item 13 task 07).
//!
//! It shows the enrolment address as a QR code and as text for as long as
//! the offer stands, keeps its status line true by asking the link once a
//! second, and raises no window of its own for a desktop that is not
//! listening — that is one dim line inside the form (design rule 8). It
//! decides nothing about the phone: every press goes straight to the
//! [`PhoneLink`] it was built with, and the line is whatever the link answers
//! (architecture rule 8).

mod imp;
mod qr;

use std::sync::Arc;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::{PhoneLink, PhoneStatus};

glib::wrapper! {
    /// A small modal over the main window titled `Phone`. Build one with
    /// [`PhoneDialog::new`]; present it with [`gtk::prelude::GtkWindowExt`]
    /// after setting its transient parent.
    pub struct PhoneDialog(ObjectSubclass<imp::PhoneDialog>)
        @extends gtk::Widget, gtk::Window,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget,
                    gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl PhoneDialog {
    /// Builds the dialog against `link`, its status line already answered
    /// and its one-second poll running until the window closes.
    #[must_use]
    pub fn new(link: Arc<dyn PhoneLink>) -> Self {
        let dialog: Self = glib::Object::new();
        dialog.imp().configure(link);
        dialog
    }

    /// Registers `handler` to run with the new status each time the polled
    /// status differs from the last one the dialog saw — a scan, a phone
    /// attaching, an un-enrol — so the window can keep its own menu in step.
    /// Replaces any previous handler.
    pub fn connect_status_changed(&self, handler: impl Fn(&PhoneStatus) + 'static) {
        self.imp()
            .on_status_changed
            .replace(Some(Box::new(handler)));
    }
}
