//! The one shortcut table (`FR.23.3`): turns a key and its modifiers into one
//! of the window's keyboard shortcuts, or nothing. Kept pure — no widget, no
//! [`super::Window`] access — so [`shortcut_for`] is unit tested with no
//! display (code standards rule 25). `Window::run_shortcut` in
//! `window/imp.rs` is the half that acts on what this decides; both the
//! Linux GTK controller and, on Windows, `EngineHost`'s accelerator
//! subscription (roadmap item 12 task 03) run through the same two
//! functions, so the two engines can never disagree about what a key means
//! (architecture rule 8, code standards rule 9).

use gtk::gdk;
use gtk4 as gtk;
use idle_manager_core::Layout;

use super::imp::ZoomStep;

/// One of the window's keyboard shortcuts, as [`shortcut_for`] names it.
/// Running one is [`super::imp::Window::run_shortcut`]'s job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Shortcut {
    /// Reload the focused view: `F5` or `Ctrl`+`R`.
    Reload,
    /// Step the focused account's zoom: `Ctrl` plus a zoom key.
    Zoom(ZoomStep),
    /// Move the focus to the next account, turning the page at its end and
    /// wrapping from the last account of the last page to the first:
    /// `Shift`+`Tab` (`FR.23.1`).
    NextAccount,
    /// Show the sidebar's next workspace that holds an account, on the page
    /// and slot it was left on: `Ctrl`+`Tab` (`FR.23.2`).
    NextWorkspace,
    /// Show or hide the sidebar: `Ctrl`+`B` (`FR.26.1`).
    ToggleSidebar,
    /// Arrange the shown workspace for this many games: `Ctrl`+`1`, `Ctrl`+`2`
    /// and `Ctrl`+`4` (`FR.26.2`). One variant for the three chords, carrying
    /// the [`Layout`] itself, so `Window::run_shortcut`'s match cannot answer
    /// two of them and forget the third. [`Layout::Mobile`] is never produced
    /// here: the phone arrangement keeps its toggle and gains no key, because
    /// the only digit left beside `1` `2` `4` is `Ctrl`+`3`, which would say
    /// "the third arrangement" while its neighbours say "this many games".
    Arrange(Layout),
    /// Park the focused account: `Ctrl`+`P` (`FR.26.3`). Always means
    /// *parked*, never "the other one" — see [`Shortcut::StartFocused`].
    ParkFocused,
    /// Start the focused account: `Ctrl`+`S` (`FR.26.3`).
    ///
    /// Park and start are two chords rather than one that flips, which
    /// departs from design rule 2's one-control shape and is recorded as a
    /// narrowing of it in rule 20. Neither of rule 2's reasons survives a
    /// keyboard: the direction that does not apply is a silent no-op, so
    /// there is no wrong state to reach, and a key occupies none of the
    /// scarce trailing edge rule 6 is about. What a flip key would cost is
    /// that nothing says which direction the press is about to take, while
    /// liveness moves on its own — queued, then starting, then live —
    /// between deciding and pressing. Two idempotent chords cannot invert
    /// under the reader's hand, and the row's dot already names which of them
    /// is live.
    StartFocused,
    /// Park every account in the shown workspace: `Ctrl`+`Shift`+`P`, the
    /// heading menu's `Park all` (`FR.26.4`, `FR.24.1`).
    ParkWorkspace,
    /// Start every parked account in the shown workspace:
    /// `Ctrl`+`Shift`+`S`, the heading menu's `Start all` (`FR.26.4`,
    /// `FR.24.2`).
    StartWorkspace,
}

/// The shortcut `key` under `modifiers` names, or `None` for every other key.
///
/// Every check below reads one named bit with [`gdk::ModifierType::contains`]
/// rather than comparing `modifiers` for exact equality, which is what
/// [`gtk::accelerator_get_default_mod_mask`] masking is really for: Caps
/// Lock, Num Lock or any other lock bit riding along on a real chord sits in
/// a bit this function never inspects, so it can never spoil a match without
/// the function ever needing to be called. That matters here specifically
/// because it asserts GTK has been initialized (`gtk4-0.11.4/src/auto/functions.rs`),
/// which a unit test must not require (code standards rule 25) — calling it
/// from this pure function would make [`shortcut_for`]'s own tests need a
/// display. GTK delivers `Shift`+`Tab` as the keyval `ISO_Left_Tab` on X11,
/// Wayland and Windows alike, so every mapping that reads `Tab` also accepts
/// `ISO_Left_Tab` and `KP_Tab` (`FR.23.3`).
pub(crate) fn shortcut_for(key: gdk::Key, modifiers: gdk::ModifierType) -> Option<Shortcut> {
    let ctrl = modifiers.contains(gdk::ModifierType::CONTROL_MASK);
    let shift = modifiers.contains(gdk::ModifierType::SHIFT_MASK);
    let is_tab_key = matches!(
        key,
        gdk::Key::Tab | gdk::Key::ISO_Left_Tab | gdk::Key::KP_Tab
    );

    if key == gdk::Key::F5 || (ctrl && key == gdk::Key::r) {
        return Some(Shortcut::Reload);
    }

    if ctrl && let Some(step) = zoom_step_for(key) {
        return Some(Shortcut::Zoom(step));
    }

    if shift && is_tab_key {
        return Some(Shortcut::NextAccount);
    }

    if ctrl && is_tab_key {
        return Some(Shortcut::NextWorkspace);
    }

    if ctrl && matches!(key, gdk::Key::b | gdk::Key::B) {
        return Some(Shortcut::ToggleSidebar);
    }

    if ctrl && let Some(layout) = layout_for(key) {
        return Some(Shortcut::Arrange(layout));
    }

    // Both letter cases in both arms, with `shift` — not the keyval's case —
    // choosing the direction. GDK delivers the shifted keyval `P` for
    // `Ctrl`+`Shift`+`P` on X11 and Wayland, while Win32's
    // `AcceleratorKeyPressed` reports the unshifted virtual key for the same
    // chord (`web_engine/virtual_key.rs`). The modifier bit is the one
    // discriminator both engines agree on.
    if ctrl && matches!(key, gdk::Key::p | gdk::Key::P) {
        return Some(if shift {
            Shortcut::ParkWorkspace
        } else {
            Shortcut::ParkFocused
        });
    }

    if ctrl && matches!(key, gdk::Key::s | gdk::Key::S) {
        return Some(if shift {
            Shortcut::StartWorkspace
        } else {
            Shortcut::StartFocused
        });
    }

    None
}

/// Whether holding `shortcut`'s keys down should run it again.
///
/// True for exactly two: `Reload` and `Zoom`. Holding `Ctrl`+`-` to zoom out
/// several steps, or `F5` to hammer a page that will not load, is the gesture
/// rather than a misfire. Everything else runs once per press — stepping
/// through every workspace, folding the sidebar back and forth or parking a
/// workspace repeatedly are all things a held key would do by accident and
/// nobody would ask for (`FR.23.3`, `FR.26.6`). Pure and matched
/// exhaustively, so a new [`Shortcut`] variant has to answer this question
/// before it compiles.
pub(crate) fn repeats_while_held(shortcut: Shortcut) -> bool {
    match shortcut {
        Shortcut::Reload | Shortcut::Zoom(_) => true,
        Shortcut::NextAccount
        | Shortcut::NextWorkspace
        | Shortcut::ToggleSidebar
        | Shortcut::Arrange(_)
        | Shortcut::ParkFocused
        | Shortcut::StartFocused
        | Shortcut::ParkWorkspace
        | Shortcut::StartWorkspace => false,
    }
}

/// The arrangement a digit names under the control modifier, or `None` for any
/// other key.
///
/// Both the top-row digit and its keypad twin are accepted, for the same
/// reason [`zoom_step_for`] accepts `KP_0` beside `_0`: which one a keyboard
/// delivers is the keyboard's business, not the window's. `3` is absent
/// deliberately — see [`Shortcut::Arrange`].
fn layout_for(key: gdk::Key) -> Option<Layout> {
    match key {
        gdk::Key::_1 | gdk::Key::KP_1 => Some(Layout::Single),
        gdk::Key::_2 | gdk::Key::KP_2 => Some(Layout::SideBySide),
        gdk::Key::_4 | gdk::Key::KP_4 => Some(Layout::Grid),
        _ => None,
    }
}

/// The zoom step a key names under the control modifier, or `None` for any
/// other key.
///
/// Plus, equals and keypad-add all mean "in" because which one a keyboard
/// delivers for `Ctrl`+`+` depends on its layout; minus and keypad-subtract
/// mean "out"; zero and keypad-zero mean reset. The exact set the keyboard
/// under test delivers is recorded in this item's `test-script.md` (code
/// standards rule 18).
fn zoom_step_for(key: gdk::Key) -> Option<ZoomStep> {
    match key {
        gdk::Key::plus | gdk::Key::equal | gdk::Key::KP_Add => Some(ZoomStep::In),
        gdk::Key::minus | gdk::Key::KP_Subtract => Some(ZoomStep::Out),
        gdk::Key::_0 | gdk::Key::KP_0 => Some(ZoomStep::Reset),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A raw modifier bit GDK4 no longer names — X11's `Mod2Mask`, commonly
    /// Num Lock — used to prove masking reaches bits with no `ModifierType`
    /// constant of their own, not only the named `LOCK_MASK`.
    const MOD2_MASK_BIT: u32 = 1 << 4;

    fn mod2_mask() -> gdk::ModifierType {
        gdk::ModifierType::from_bits_retain(MOD2_MASK_BIT)
    }

    #[test]
    fn f5_and_ctrl_r_map_to_reload() {
        assert_eq!(
            shortcut_for(gdk::Key::F5, gdk::ModifierType::empty()),
            Some(Shortcut::Reload)
        );
        assert_eq!(
            shortcut_for(gdk::Key::r, gdk::ModifierType::CONTROL_MASK),
            Some(Shortcut::Reload)
        );
    }

    #[test]
    fn ctrl_plus_each_zoom_key_maps_to_the_matching_zoom_step() {
        let mapped: Vec<Option<Shortcut>> = [
            gdk::Key::plus,
            gdk::Key::equal,
            gdk::Key::KP_Add,
            gdk::Key::minus,
            gdk::Key::KP_Subtract,
            gdk::Key::_0,
            gdk::Key::KP_0,
        ]
        .into_iter()
        .map(|key| shortcut_for(key, gdk::ModifierType::CONTROL_MASK))
        .collect();

        assert_eq!(
            mapped,
            vec![
                Some(Shortcut::Zoom(ZoomStep::In)),
                Some(Shortcut::Zoom(ZoomStep::In)),
                Some(Shortcut::Zoom(ZoomStep::In)),
                Some(Shortcut::Zoom(ZoomStep::Out)),
                Some(Shortcut::Zoom(ZoomStep::Out)),
                Some(Shortcut::Zoom(ZoomStep::Reset)),
                Some(Shortcut::Zoom(ZoomStep::Reset)),
            ]
        );
    }

    #[test]
    fn an_unmodified_key_with_no_meaning_here_is_none() {
        assert_eq!(shortcut_for(gdk::Key::a, gdk::ModifierType::empty()), None);
    }

    #[test]
    fn shift_plus_tab_iso_left_tab_or_kp_tab_maps_to_next_account() {
        let mapped: Vec<Option<Shortcut>> =
            [gdk::Key::Tab, gdk::Key::ISO_Left_Tab, gdk::Key::KP_Tab]
                .into_iter()
                .map(|key| shortcut_for(key, gdk::ModifierType::SHIFT_MASK))
                .collect();

        assert_eq!(
            mapped,
            vec![
                Some(Shortcut::NextAccount),
                Some(Shortcut::NextAccount),
                Some(Shortcut::NextAccount),
            ]
        );
    }

    #[test]
    fn ctrl_plus_tab_iso_left_tab_or_kp_tab_maps_to_next_workspace() {
        let mapped: Vec<Option<Shortcut>> =
            [gdk::Key::Tab, gdk::Key::ISO_Left_Tab, gdk::Key::KP_Tab]
                .into_iter()
                .map(|key| shortcut_for(key, gdk::ModifierType::CONTROL_MASK))
                .collect();

        assert_eq!(
            mapped,
            vec![
                Some(Shortcut::NextWorkspace),
                Some(Shortcut::NextWorkspace),
                Some(Shortcut::NextWorkspace),
            ]
        );
    }

    #[test]
    fn tab_alone_is_none() {
        assert_eq!(
            shortcut_for(gdk::Key::Tab, gdk::ModifierType::empty()),
            None
        );
    }

    #[test]
    fn ctrl_plus_b_in_either_case_maps_to_toggle_sidebar() {
        let mapped: Vec<Option<Shortcut>> = [gdk::Key::b, gdk::Key::B]
            .into_iter()
            .map(|key| shortcut_for(key, gdk::ModifierType::CONTROL_MASK))
            .collect();

        assert_eq!(
            mapped,
            vec![Some(Shortcut::ToggleSidebar), Some(Shortcut::ToggleSidebar)]
        );
    }

    #[test]
    fn ctrl_plus_one_two_or_four_maps_to_the_matching_arrangement() {
        let mapped: Vec<Option<Shortcut>> = [
            gdk::Key::_1,
            gdk::Key::KP_1,
            gdk::Key::_2,
            gdk::Key::KP_2,
            gdk::Key::_4,
            gdk::Key::KP_4,
        ]
        .into_iter()
        .map(|key| shortcut_for(key, gdk::ModifierType::CONTROL_MASK))
        .collect();

        assert_eq!(
            mapped,
            vec![
                Some(Shortcut::Arrange(Layout::Single)),
                Some(Shortcut::Arrange(Layout::Single)),
                Some(Shortcut::Arrange(Layout::SideBySide)),
                Some(Shortcut::Arrange(Layout::SideBySide)),
                Some(Shortcut::Arrange(Layout::Grid)),
                Some(Shortcut::Arrange(Layout::Grid)),
            ]
        );
    }

    /// The phone arrangement has no key on purpose (`FR.26.2`), and the digit
    /// that would be the obvious one to reach for is the one that must stay
    /// unbound.
    #[test]
    fn ctrl_plus_three_maps_to_nothing() {
        assert_eq!(
            shortcut_for(gdk::Key::_3, gdk::ModifierType::CONTROL_MASK),
            None
        );
        assert_eq!(
            shortcut_for(gdk::Key::KP_3, gdk::ModifierType::CONTROL_MASK),
            None
        );
    }

    #[test]
    fn the_window_chord_keys_mean_nothing_without_control() {
        let mapped: Vec<Option<Shortcut>> = [gdk::Key::b, gdk::Key::_1, gdk::Key::_2, gdk::Key::_4]
            .into_iter()
            .map(|key| shortcut_for(key, gdk::ModifierType::empty()))
            .collect();

        assert_eq!(mapped, vec![None, None, None, None]);
    }

    #[test]
    fn caps_lock_and_num_lock_added_to_a_matching_chord_still_match() {
        let noisy_shift_tab =
            gdk::ModifierType::SHIFT_MASK | gdk::ModifierType::LOCK_MASK | mod2_mask();
        let noisy_ctrl_r =
            gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::LOCK_MASK | mod2_mask();

        assert_eq!(
            shortcut_for(gdk::Key::Tab, noisy_shift_tab),
            Some(Shortcut::NextAccount)
        );
        assert_eq!(
            shortcut_for(gdk::Key::r, noisy_ctrl_r),
            Some(Shortcut::Reload)
        );
    }

    #[test]
    fn only_reload_and_zoom_repeat_while_held() {
        assert!(repeats_while_held(Shortcut::Reload));
        assert!(repeats_while_held(Shortcut::Zoom(ZoomStep::Out)));
        assert!(!repeats_while_held(Shortcut::NextAccount));
        assert!(!repeats_while_held(Shortcut::NextWorkspace));
        assert!(!repeats_while_held(Shortcut::ToggleSidebar));
        assert!(!repeats_while_held(Shortcut::Arrange(Layout::Grid)));
        assert!(!repeats_while_held(Shortcut::ParkFocused));
        assert!(!repeats_while_held(Shortcut::StartFocused));
        assert!(!repeats_while_held(Shortcut::ParkWorkspace));
        assert!(!repeats_while_held(Shortcut::StartWorkspace));
    }

    #[test]
    fn ctrl_plus_p_or_s_in_either_case_acts_on_the_focused_account() {
        let mapped: Vec<Option<Shortcut>> = [gdk::Key::p, gdk::Key::P, gdk::Key::s, gdk::Key::S]
            .into_iter()
            .map(|key| shortcut_for(key, gdk::ModifierType::CONTROL_MASK))
            .collect();

        assert_eq!(
            mapped,
            vec![
                Some(Shortcut::ParkFocused),
                Some(Shortcut::ParkFocused),
                Some(Shortcut::StartFocused),
                Some(Shortcut::StartFocused),
            ]
        );
    }

    /// The shift bit alone chooses the direction, in both letter cases: X11
    /// and Wayland deliver the shifted keyval here, Win32 the unshifted one,
    /// and one table has to answer both the same way.
    #[test]
    fn ctrl_shift_plus_p_or_s_in_either_case_acts_on_the_whole_workspace() {
        let ctrl_shift = gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK;
        let mapped: Vec<Option<Shortcut>> = [gdk::Key::p, gdk::Key::P, gdk::Key::s, gdk::Key::S]
            .into_iter()
            .map(|key| shortcut_for(key, ctrl_shift))
            .collect();

        assert_eq!(
            mapped,
            vec![
                Some(Shortcut::ParkWorkspace),
                Some(Shortcut::ParkWorkspace),
                Some(Shortcut::StartWorkspace),
                Some(Shortcut::StartWorkspace),
            ]
        );
    }

    #[test]
    fn caps_lock_and_num_lock_never_promote_ctrl_p_to_the_workspace_chord() {
        let noisy_ctrl_p =
            gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::LOCK_MASK | mod2_mask();
        let noisy_ctrl_shift_p = noisy_ctrl_p | gdk::ModifierType::SHIFT_MASK;

        assert_eq!(
            shortcut_for(gdk::Key::p, noisy_ctrl_p),
            Some(Shortcut::ParkFocused)
        );
        assert_eq!(
            shortcut_for(gdk::Key::p, noisy_ctrl_shift_p),
            Some(Shortcut::ParkWorkspace)
        );
    }

    #[test]
    fn caps_lock_and_num_lock_added_to_ctrl_b_still_toggles_the_sidebar() {
        let noisy_ctrl_b =
            gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::LOCK_MASK | mod2_mask();

        assert_eq!(
            shortcut_for(gdk::Key::b, noisy_ctrl_b),
            Some(Shortcut::ToggleSidebar)
        );
    }
}
