//! One row in the sidebar list: an account's name, where it currently sits,
//! whether it is running, and the action that toggles that.

mod imp;

use gtk::glib;
use gtk4 as gtk;

use idle_manager_core::{Liveness, SessionId, Visibility};

glib::wrapper! {
    /// A list item's data: the account's id, the name markup the bound widget
    /// shows, a status key naming its state marker, and the action button's
    /// label. Build one with [`Row::new`]; update it in place with
    /// [`Row::refresh`].
    pub struct Row(ObjectSubclass<imp::Row>);
}

impl Row {
    /// A row for `id`, showing `display_name`, placed at `visibility` with
    /// `liveness`. `current` marks the row whose account holds the focused slot.
    pub(crate) fn new(
        id: &SessionId,
        display_name: &str,
        liveness: Liveness,
        visibility: Visibility,
        current: bool,
    ) -> Self {
        let row: Self = glib::Object::builder()
            .property("id", id.as_str())
            .property("display-name", display_name)
            .build();
        row.refresh(display_name, liveness, visibility, current);
        row
    }

    /// Rewrites the rendered state from the account's current standing.
    pub(crate) fn refresh(
        &self,
        display_name: &str,
        liveness: Liveness,
        visibility: Visibility,
        current: bool,
    ) {
        self.set_display_name(display_name);
        self.set_status(status_key(liveness, visibility, current));
        self.set_name_markup(name_markup(display_name, liveness, visibility, current));
        self.set_action_label(action_label(liveness));
    }
}

/// The state a row's trailing marker names, as a stable key.
///
/// The single place a domain state becomes a marker. Item 03 added `"parked"`
/// here and item 08 adds `"unresponsive"`, so the factory that builds each row
/// — and `sidebar.css`, which styles one dot class per key — grow one arm,
/// never a branch (design rule 1). Parked wins over every place: an account
/// that is not running says so first, whatever slot it still holds.
pub(super) fn status_key(
    liveness: Liveness,
    visibility: Visibility,
    current: bool,
) -> &'static str {
    if matches!(liveness, Liveness::Parked) {
        return "parked";
    }
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
        "parked" => "Parked",
        _ => "",
    }
}

/// The trailing action button's label: `Park` while the account runs, `Start`
/// once it is parked or on its way back up. One control whose meaning inverts
/// with the row's state (design rule 2).
pub(super) fn action_label(liveness: Liveness) -> &'static str {
    match liveness {
        Liveness::Live => "Park",
        Liveness::Parked | Liveness::Starting => "Start",
    }
}

/// The name label's Pango markup. Dimmed for an account that is not on screen
/// *or* not running — the two states share the style (design rule 3); bold for
/// the current row; plain otherwise (design rule 1).
fn name_markup(
    display_name: &str,
    liveness: Liveness,
    visibility: Visibility,
    current: bool,
) -> String {
    let name = glib::markup_escape_text(display_name);
    if matches!(liveness, Liveness::Parked) || matches!(visibility, Visibility::OffGrid) {
        format!("<span alpha=\"55%\">{name}</span>")
    } else if current {
        format!("<b>{name}</b>")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use idle_manager_core::SlotId;

    use super::*;

    #[test]
    fn a_parked_account_keys_as_parked_whatever_its_visibility() {
        let in_slot = status_key(Liveness::Parked, Visibility::InSlot(SlotId::FIRST), true);
        let off_grid = status_key(Liveness::Parked, Visibility::OffGrid, false);

        assert_eq!((in_slot, off_grid), ("parked", "parked"));
    }

    #[test]
    fn a_live_account_keeps_the_current_visible_and_background_keys() {
        let current = status_key(Liveness::Live, Visibility::InSlot(SlotId::FIRST), true);
        let visible = status_key(Liveness::Live, Visibility::InSlot(SlotId::FIRST), false);
        let background = status_key(Liveness::Live, Visibility::OffGrid, false);

        assert_eq!(
            (current, visible, background),
            ("current", "visible", "background")
        );
    }
}
