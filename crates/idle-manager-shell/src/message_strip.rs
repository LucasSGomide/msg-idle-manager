//! A dismissible strip across the top of the window for something that happened
//! before the user did anything — a saved workspace that would not load (item
//! 07 task 04), and later a save that failed (task 06). One line, one dismiss
//! button, hidden while it has nothing to say (design rule 9).
//!
//! It is one reusable widget rather than a label built per occasion, so the two
//! occasions cannot drift apart.

mod imp;

use gtk::glib;
use gtk::prelude::*;
use gtk4 as gtk;

glib::wrapper! {
    /// A one-line message bar built from `ui/message-strip.ui`. Build one with
    /// [`MessageStrip::new`], raise a message with [`MessageStrip::show`], and
    /// take it down with [`MessageStrip::clear`]; the close button also takes it
    /// down.
    pub struct MessageStrip(ObjectSubclass<imp::MessageStrip>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl MessageStrip {
    /// Builds a hidden strip.
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    /// Shows `message` on the strip. Replaces whatever it was showing.
    pub fn show(&self, message: &str) {
        self.set_message(message);
        self.set_visible(true);
    }

    /// Hides the strip. A no-op when it is already hidden, so a successful save
    /// after a dismissed failure leaves it dismissed (task 06).
    pub fn clear(&self) {
        self.set_visible(false);
    }
}

impl Default for MessageStrip {
    fn default() -> Self {
        Self::new()
    }
}
