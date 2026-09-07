//! The list row's backing object: an account's identity and the two strings a
//! bound list item renders (architecture rule 12).

use std::cell::{Cell, RefCell};

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
    /// The status key for the trailing marker: `current`, `visible`,
    /// `background`, `parked` or `starting`. Names both the word shown and the
    /// dot's CSS class.
    #[property(get, set)]
    status: RefCell<String>,
    /// The Pango markup for the name label — bold, dimmed or plain.
    #[property(get, set)]
    name_markup: RefCell<String>,
    /// The trailing action button's label — `Park` while the account runs,
    /// `Start` once it is parked. The factory binds it and never branches.
    #[property(get, set)]
    action_label: RefCell<String>,
    /// Whether the trailing action button is pressable. `false` only while the
    /// account is starting, so a second press cannot build a second view.
    #[property(get, set)]
    action_sensitive: Cell<bool>,
    /// Whether the account must keep running at full speed while hidden,
    /// mirrored from [`idle_manager_core::Session::is_kept_awake`]. The row
    /// menu's checkable item reads this on every bind rather than reaching
    /// into the book (naming rule 12).
    #[property(get, set)]
    is_kept_awake: Cell<bool>,
    /// The trailing keep-awake indication's text: the glyph when the account
    /// is kept awake, empty otherwise. A separate property from
    /// `is_kept_awake` because the two are read by different widgets — the
    /// menu's checkbox reads the raw flag, the indicator label reads this.
    #[property(get, set)]
    keep_awake_mark: RefCell<String>,
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
