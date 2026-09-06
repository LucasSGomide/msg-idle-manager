//! One row in the sidebar list: an account's name and where it currently sits.

mod imp;

use gtk::glib;
use gtk4 as gtk;

use idle_manager_core::{SessionId, Visibility};

glib::wrapper! {
    /// A list item's data: the account's id, the name markup the bound widget
    /// shows, and a status key naming its state marker. Build one with
    /// [`Row::new`]; update it in place with [`Row::refresh`].
    pub struct Row(ObjectSubclass<imp::Row>);
}

impl Row {
    /// A row for `id`, showing `display_name`, placed at `visibility`. `current`
    /// marks the row whose account holds the focused slot.
    pub(crate) fn new(
        id: &SessionId,
        display_name: &str,
        visibility: Visibility,
        current: bool,
    ) -> Self {
        let row: Self = glib::Object::builder()
            .property("id", id.as_str())
            .property("display-name", display_name)
            .build();
        row.refresh(display_name, visibility, current);
        row
    }

    /// Rewrites the rendered state from the account's current standing.
    pub(crate) fn refresh(&self, display_name: &str, visibility: Visibility, current: bool) {
        self.set_display_name(display_name);
        self.set_status(status_key(visibility, current));
        self.set_name_markup(name_markup(display_name, visibility, current));
    }
}

/// The state a row's trailing marker names, as a stable key.
///
/// The single place a domain state becomes a marker. Item 03 adds `"parked"`
/// here and item 08 `"unresponsive"`, so the factory that builds each row — and
/// `sidebar.css`, which styles one dot class per key — grow one arm, never a
/// branch (roadmap 02, front-end).
pub(super) fn status_key(visibility: Visibility, current: bool) -> &'static str {
    match visibility {
        Visibility::InSlot(_) if current => "current",
        Visibility::InSlot(_) => "visible",
        Visibility::OffGrid => "background",
    }
}

/// The word shown beside the status dot for `key`.
pub(super) fn status_label(key: &str) -> &'static str {
    match key {
        "current" => "Current",
        "visible" => "Visible",
        "background" => "Background",
        _ => "",
    }
}

/// The name label's Pango markup: bold for the current row, dimmed for an
/// out-of-sight account, plain otherwise (design rule 1).
fn name_markup(display_name: &str, visibility: Visibility, current: bool) -> String {
    let name = glib::markup_escape_text(display_name);
    if current {
        format!("<b>{name}</b>")
    } else if matches!(visibility, Visibility::OffGrid) {
        format!("<span alpha=\"55%\">{name}</span>")
    } else {
        name.to_string()
    }
}
