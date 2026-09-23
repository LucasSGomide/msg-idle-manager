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
    /// `background`, `parked` or `starting`. Names the dot's CSS class and,
    /// through `status_label`, the dot's tooltip and accessible label.
    #[property(get, set)]
    status: RefCell<String>,
    /// The Pango markup for the name label — bold, dimmed or plain.
    #[property(get, set)]
    name_markup: RefCell<String>,
    /// The Park/Start menu item's label — `Park` while the account runs,
    /// `Start` once it is parked. The factory binds it and never branches.
    #[property(get, set)]
    action_label: RefCell<String>,
    /// The Park/Start menu item's accelerator text — `<Control>p` while the
    /// account runs, `<Control>s` otherwise. A menu item has nowhere to
    /// hover, so this is where its key is named (`FR.25.4`, design rule 19);
    /// a separate property from `action_label` because the two are set on
    /// different things — the label on the item, this on its `accel`
    /// attribute.
    #[property(get, set)]
    action_accelerator: RefCell<String>,
    /// Whether the Park/Start menu item is sensitive. `false` only while the
    /// account is starting, so a second press cannot build a second view.
    #[property(get, set)]
    action_sensitive: Cell<bool>,
    /// Whether the account must keep running at full speed while hidden,
    /// mirrored from [`idle_manager_core::Session::is_kept_awake`]. The row
    /// menu's checkable item reads this on every bind rather than reaching
    /// into the book (naming rule 12).
    #[property(get, set)]
    is_kept_awake: Cell<bool>,
    /// The workspace this account currently belongs to, as a plain id
    /// string. What the row's own `Move to ▸` submenu shows insensitive and
    /// marked "(here)" rather than as a live choice (design rule 23).
    #[property(get, set)]
    workspace_id: RefCell<String>,
    /// That workspace's name, so `Move to ▸` can name the "(here)" entry
    /// even when [`idle_manager_core::WorkspaceBook::destinations`] leaves a
    /// full workspace out of the offered list.
    #[property(get, set)]
    workspace_name: RefCell<String>,
    /// Whether this account holds the shown workspace's focused slot,
    /// independently of `status` — a parked or starting focused account
    /// never reads `status == "current"` (liveness outranks visibility in
    /// `status_key`), so this is what the focus bar and the row menu's
    /// accelerator actually key on (design rule 26).
    #[property(get, set)]
    is_current: Cell<bool>,
    /// Whether this row is the dim "No accounts" leaf under an empty
    /// workspace rather than a real account (roadmap item 11). The factory
    /// reads this to hide the status mark, the keep-awake icon and the ⋯
    /// menu, and to keep the row from being activated.
    #[property(get, set)]
    is_placeholder: Cell<bool>,
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
