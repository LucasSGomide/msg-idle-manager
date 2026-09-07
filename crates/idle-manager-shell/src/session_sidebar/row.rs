//! One row in the sidebar list: an account's name, where it currently sits,
//! whether it is running, and the action that toggles that.

mod imp;

use gtk::glib;
use gtk4 as gtk;

use idle_manager_core::{Liveness, Session, Visibility};

/// The glyph the trailing keep-awake indication renders. A diamond, not a
/// circle, so it never reads as a second status dot (design rule 6) — it
/// names a configuration that does not change, not a state that does.
const KEEP_AWAKE_MARK: &str = "◆";

glib::wrapper! {
    /// A list item's data: the account's id, the name markup the bound widget
    /// shows, a status key naming its state marker, and the action button's
    /// label. Build one with [`Row::new`]; update it in place with
    /// [`Row::refresh`].
    pub struct Row(ObjectSubclass<imp::Row>);
}

impl Row {
    /// A row for `session`. `current` marks the row whose account holds the
    /// focused slot — the one fact a [`Session`] does not carry about itself.
    pub(crate) fn new(session: &Session, current: bool) -> Self {
        let row: Self = glib::Object::builder()
            .property("id", session.id().as_str())
            .property("display-name", session.display_name())
            .build();
        row.refresh(session, current);
        row
    }

    /// Rewrites the rendered state from `session`'s current standing.
    pub(crate) fn refresh(&self, session: &Session, current: bool) {
        let (liveness, visibility, display_name) = (
            session.liveness(),
            session.visibility(),
            session.display_name(),
        );

        self.set_display_name(display_name);
        self.set_status(status_key(liveness, visibility, current));
        self.set_name_markup(name_markup(display_name, liveness, visibility, current));
        self.set_action_label(action_label(liveness));
        self.set_action_sensitive(action_sensitive(liveness));
        self.set_is_kept_awake(session.is_kept_awake());
        self.set_keep_awake_mark(keep_awake_indication(session.is_kept_awake()));
    }
}

/// The state a row's trailing marker names, as a stable key.
///
/// The single place a domain state becomes a marker. Item 03 added `"parked"`
/// here, item 04 `"starting"`, and item 08 adds `"unresponsive"`, so the
/// factory that builds each row — and `sidebar.css`, which styles one dot class
/// per key — grow one arm, never a branch (design rule 1). Liveness wins over
/// every place: an account that is not simply running says so first, whatever
/// slot it still holds.
pub(super) fn status_key(
    liveness: Liveness,
    visibility: Visibility,
    current: bool,
) -> &'static str {
    match liveness {
        Liveness::Parked => return "parked",
        Liveness::Starting => return "starting",
        Liveness::Live => {}
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
        "starting" => "Starting",
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

/// Whether the row's action button is pressable. Insensitive only while the
/// account is starting, so an impatient second press cannot build a second
/// view for one account (the domain models `Starting`, so this holds in every
/// caller at once).
pub(super) fn action_sensitive(liveness: Liveness) -> bool {
    !matches!(liveness, Liveness::Starting)
}

/// The row's trailing keep-awake indication: the glyph while the account's
/// flag is on, empty while it is off. A separate, independent mark from
/// `status_key` — keep-awake is a configuration a row carries, not a sixth
/// state (design rule 1); two facts that can both be true at once must not
/// share one key.
pub(super) fn keep_awake_indication(is_kept_awake: bool) -> &'static str {
    if is_kept_awake { KEEP_AWAKE_MARK } else { "" }
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
    use idle_manager_core::{SessionBook, SlotId};

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

    #[test]
    fn a_starting_account_keys_as_starting_whatever_its_visibility() {
        let in_slot = status_key(Liveness::Starting, Visibility::InSlot(SlotId::FIRST), true);
        let off_grid = status_key(Liveness::Starting, Visibility::OffGrid, false);

        assert_eq!((in_slot, off_grid), ("starting", "starting"));
    }

    #[test]
    fn the_action_label_inverts_with_liveness() {
        let live = action_label(Liveness::Live);
        let parked = action_label(Liveness::Parked);

        assert_eq!((live, parked), ("Park", "Start"));
    }

    #[test]
    fn the_action_button_is_insensitive_only_while_starting() {
        let while_starting = action_sensitive(Liveness::Starting);
        let while_live = action_sensitive(Liveness::Live);
        let while_parked = action_sensitive(Liveness::Parked);

        assert_eq!(
            (while_starting, while_live, while_parked),
            (false, true, true)
        );
    }

    #[test]
    fn the_keep_awake_indication_shows_only_when_the_flag_is_on() {
        let on = keep_awake_indication(true);
        let off = keep_awake_indication(false);

        assert_eq!((on, off), (KEEP_AWAKE_MARK, ""));
    }

    #[test]
    fn status_key_is_the_same_whether_or_not_the_account_is_kept_awake() {
        let mut book = SessionBook::new();
        let plain = book.add("Plain", "https://example.test/plain");
        let awake = book.add("Awake", "https://example.test/awake");
        book.park(&plain);
        book.park(&awake);
        book.set_keep_awake(&awake, true);

        let sessions = book.sessions();
        let plain_session = sessions
            .iter()
            .find(|s| s.id() == &plain)
            .expect("just added to the book");
        let awake_session = sessions
            .iter()
            .find(|s| s.id() == &awake)
            .expect("just added to the book");

        let plain_key = status_key(plain_session.liveness(), plain_session.visibility(), false);
        let awake_key = status_key(awake_session.liveness(), awake_session.visibility(), false);

        assert_eq!(plain_key, awake_key);
    }
}
