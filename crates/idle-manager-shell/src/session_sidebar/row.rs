//! One row in the sidebar list: an account's name, a single coloured dot
//! naming where it currently sits and whether it is running, and — behind the
//! row's ⋯ menu — the action that toggles that.

mod imp;

use gtk::glib;
use gtk4 as gtk;

use idle_manager_core::{Liveness, Session, Visibility};

/// The glyph the trailing keep-awake indication renders. A diamond, not a
/// circle, so it never reads as a second status dot (design rule 6) — it
/// names a configuration that does not change, not a state that does.
const KEEP_AWAKE_MARK: &str = "◆";

/// The dim line a named workspace with no accounts shows once expanded — a
/// real leaf in the tree, indented exactly like an account row, rather than
/// hidden state on the heading, so the tree model's own expand/collapse
/// handles when it is in the flat list for free (roadmap item 11).
const NO_ACCOUNTS_TEXT: &str = "No accounts";

glib::wrapper! {
    /// A list item's data: the account's id, the name markup the bound widget
    /// shows, a status key naming its state marker, and the Park/Start menu
    /// item's label. Build one with [`Row::new`], or [`Row::placeholder`] for
    /// the dim "No accounts" leaf under an empty workspace.
    pub struct Row(ObjectSubclass<imp::Row>);
}

impl Row {
    /// A row for `session`, seated as `visibility` — [`WorkspaceBook::placement`]'s
    /// answer, never `session.visibility()` directly, since an account in a
    /// workspace that is not shown must key as `background` regardless of the
    /// slot it still holds there (`FR.15.7`). `current` marks the row whose
    /// account holds the shown workspace's focused slot.
    ///
    /// [`WorkspaceBook::placement`]: idle_manager_core::WorkspaceBook::placement
    pub(crate) fn new(session: &Session, visibility: Visibility, current: bool) -> Self {
        let (liveness, display_name) = (session.liveness(), session.display_name());

        let row: Self = glib::Object::builder()
            .property("id", session.id().as_str())
            .property("display-name", display_name)
            .property("status", status_key(liveness, visibility, current))
            .property(
                "name-markup",
                name_markup(display_name, liveness, visibility, current),
            )
            .property("action-label", action_label(liveness))
            .property("action-sensitive", action_sensitive(liveness))
            .property("is-kept-awake", session.is_kept_awake())
            .property(
                "keep-awake-mark",
                keep_awake_indication(session.is_kept_awake()),
            )
            .build();
        row
    }

    /// The dim, insensitive "No accounts" leaf shown under a named workspace
    /// that holds nothing, once expanded. Carries no account id, no dot, no
    /// menu and no keep-awake mark; [`Row::is_placeholder`] is what the
    /// factory reads to hide those widgets and skip activation.
    pub(crate) fn placeholder() -> Self {
        glib::Object::builder()
            .property("id", "")
            .property("display-name", NO_ACCOUNTS_TEXT)
            .property("status", "")
            .property(
                "name-markup",
                format!(
                    "<span alpha=\"55%\">{}</span>",
                    glib::markup_escape_text(NO_ACCOUNTS_TEXT)
                ),
            )
            .property("action-label", "")
            .property("action-sensitive", false)
            .property("is-kept-awake", false)
            .property("keep-awake-mark", "")
            .property("is-placeholder", true)
            .build()
    }
}

/// The state a row's trailing marker names, as a stable key.
///
/// The single place a domain state becomes a marker. Item 03 added `"parked"`
/// here, item 04 `"starting"`, item 07 `"queued"`, and item 08 adds
/// `"unresponsive"`, so the factory that builds each row — and `sidebar.css`,
/// which styles one dot class per key — grow one arm, never a branch (design
/// rule 1). Liveness wins over every visibility key: an account that is not
/// simply running says so first, whatever slot it still holds. Among the
/// not-simply-running states, waiting for a turn (`"queued"`) is a weaker claim
/// than being on its way up (`"starting"`), which is a weaker claim than being
/// stopped (`"parked"`) — but each account is in exactly one, so the order is a
/// reading aid, not a precedence the code resolves.
pub(super) fn status_key(
    liveness: Liveness,
    visibility: Visibility,
    current: bool,
) -> &'static str {
    match liveness {
        Liveness::Parked => return "parked",
        Liveness::Starting => return "starting",
        Liveness::Queued => return "queued",
        Liveness::Live => {}
    }
    match visibility {
        Visibility::InSlot(_) if current => "current",
        Visibility::InSlot(_) => "visible",
        Visibility::OffGrid => "background",
    }
}

/// The state word for `key`. No longer shown on the row — the dot is the only
/// visible signal now (design rule 1) — but still the one source of the dot's
/// tooltip and its accessible label, so a pointer and a screen reader can name
/// the state the colour carries.
pub(super) fn status_label(key: &str) -> &'static str {
    match key {
        "current" => "Current",
        "visible" => "Visible",
        "background" => "Background",
        "parked" => "Parked",
        "starting" => "Starting",
        "queued" => "Queued",
        _ => "",
    }
}

/// The Park/Start menu item's label: `Park` while the account runs, `Start`
/// once it is parked, queued, or on its way back up. One item whose meaning
/// inverts with the row's state (design rule 2).
pub(super) fn action_label(liveness: Liveness) -> &'static str {
    match liveness {
        Liveness::Live => "Park",
        Liveness::Parked | Liveness::Starting | Liveness::Queued => "Start",
    }
}

/// Whether the row's Park/Start menu item is sensitive. Insensitive — shown,
/// greyed — while the account is starting *or* queued: a second press must not
/// build a second view, and during a restore the start queue owns the order, so
/// nothing but the queue may start a queued account (design rule 2).
pub(super) fn action_sensitive(liveness: Liveness) -> bool {
    !matches!(liveness, Liveness::Starting | Liveness::Queued)
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
/// *or* not running — parked and queued are both "not running" and share the
/// style (design rule 3); bold for the current row; plain otherwise (design
/// rule 1).
fn name_markup(
    display_name: &str,
    liveness: Liveness,
    visibility: Visibility,
    current: bool,
) -> String {
    let name = glib::markup_escape_text(display_name);
    if matches!(liveness, Liveness::Parked | Liveness::Queued)
        || matches!(visibility, Visibility::OffGrid)
    {
        format!("<span alpha=\"55%\">{name}</span>")
    } else if current {
        format!("<b>{name}</b>")
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use idle_manager_core::{SessionBook, SessionId, SlotId};

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
    fn a_queued_account_keys_as_queued_whatever_its_slot_or_focus() {
        let in_current_slot = status_key(Liveness::Queued, Visibility::InSlot(SlotId::FIRST), true);
        let in_other_slot = status_key(Liveness::Queued, Visibility::InSlot(SlotId::FIRST), false);
        let off_grid = status_key(Liveness::Queued, Visibility::OffGrid, false);

        assert_eq!(
            (in_current_slot, in_other_slot, off_grid),
            ("queued", "queued", "queued")
        );
    }

    #[test]
    fn the_queued_key_has_a_word_a_tooltip_and_a_screen_reader_can_read() {
        assert_eq!(status_label("queued"), "Queued");
    }

    #[test]
    fn a_queued_accounts_park_start_item_reads_start() {
        assert_eq!(action_label(Liveness::Queued), "Start");
    }

    #[test]
    fn a_queued_accounts_park_start_item_is_insensitive() {
        assert!(!action_sensitive(Liveness::Queued));
    }

    #[test]
    fn a_queued_accounts_name_is_dimmed_under_the_existing_not_running_rule() {
        let queued = name_markup(
            "Farm",
            Liveness::Queued,
            Visibility::InSlot(SlotId::FIRST),
            false,
        );
        let parked = name_markup(
            "Farm",
            Liveness::Parked,
            Visibility::InSlot(SlotId::FIRST),
            false,
        );

        assert_eq!(queued, parked);
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
        let plain = SessionId::new("session-0001");
        book.add(&plain, "Plain", "https://example.test/plain");
        let awake = SessionId::new("session-0002");
        book.add(&awake, "Awake", "https://example.test/awake");
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
