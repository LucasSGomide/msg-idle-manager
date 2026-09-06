//! The panel that stands in a slot for a parked account: its name, a line of
//! state text, and one action button. Item 08 renders its failure panel from
//! this same widget with different words and a different action, so the state
//! text and the button label are properties rather than markup.
//!
//! It reports the button press through [`SlotPlaceholder::connect_start_requested`]
//! and decides nothing itself (architecture rule 8).

mod imp;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

glib::wrapper! {
    /// A centred panel on the window's background, built from
    /// `ui/slot-placeholder.ui`. Build one with [`SlotPlaceholder::new`], set
    /// `name`, `state-text`, `button-label` and `button-sensitive`, and install
    /// a press handler with [`SlotPlaceholder::connect_start_requested`].
    pub struct SlotPlaceholder(ObjectSubclass<imp::SlotPlaceholder>)
        @extends gtk::Widget, gtk::Box,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl SlotPlaceholder {
    /// Builds an empty panel with a pressable button.
    #[must_use]
    pub fn new() -> Self {
        glib::Object::builder()
            .property("button-sensitive", true)
            .build()
    }

    /// Registers `handler` to run when the panel's button is pressed. Replaces
    /// any previous handler.
    pub fn connect_start_requested(&self, handler: impl Fn() + 'static) {
        self.imp()
            .on_start_requested
            .replace(Some(Box::new(handler)));
    }
}

impl Default for SlotPlaceholder {
    fn default() -> Self {
        Self::new()
    }
}
