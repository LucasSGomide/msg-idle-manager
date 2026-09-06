//! The list row's backing object: an account's identity and the two strings a
//! bound list item renders (architecture rule 12).

use std::cell::RefCell;

use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

/// The property-backed object for [`super::Row`].
#[derive(glib::Properties, Default)]
#[properties(wrapper_type = super::Row)]
pub struct Row {
    /// The account's minted identifier, as a string, so an activated row can
    /// name the session to the domain.
    #[property(get, set)]
    id: RefCell<String>,
    /// The name the user typed, kept for inspection; the rendered name comes
    /// from `name-markup`.
    #[property(get, set)]
    display_name: RefCell<String>,
    /// The status key for the trailing marker: `current`, `visible` or
    /// `background`. Names both the word shown and the dot's CSS class.
    #[property(get, set)]
    status: RefCell<String>,
    /// The Pango markup for the name label — bold, dimmed or plain.
    #[property(get, set)]
    name_markup: RefCell<String>,
}

impl std::fmt::Debug for Row {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Row")
            .field("id", &self.id.borrow())
            .finish_non_exhaustive()
    }
}

#[glib::object_subclass]
impl ObjectSubclass for Row {
    const NAME: &'static str = "IdleManagerSidebarRow";
    type Type = super::Row;
}

#[glib::derived_properties]
impl ObjectImpl for Row {}
