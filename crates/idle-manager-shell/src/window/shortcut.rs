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

    None
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
}
