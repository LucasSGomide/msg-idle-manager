//! Maps a Win32 virtual-key code to the `gdk::Key` the window's own keyboard
//! shortcuts already match, so `WebView2`'s `AcceleratorKeyPressed` (roadmap
//! item 12 task 03) and the GTK key controller (`window/imp.rs`) can never
//! disagree about what a key means.
//!
//! Pure and dependency-free on purpose, same reasoning as `profile_name.rs`
//! and `host_bounds.rs`: `make windows-check` only compiles and lints the
//! Windows target, it never runs what it compiles, so this is the one part
//! of the mapping a Linux `cargo test` can actually execute.

use gtk4::gdk;

/// The Win32 virtual-key codes the window's shortcuts care about (the
/// [Virtual-Key Codes] winuser.h names), spelled out here rather than pulled
/// from the `windows` crate's own constants so this module stays
/// dependency-free and testable on Linux — `web_engine/webview2/ffi.rs` is
/// the one place that checks a real `AcceleratorKeyPressed` event's virtual
/// key against these.
///
/// [Virtual-Key Codes]: https://learn.microsoft.com/en-us/windows/win32/inputdev/virtual-key-codes
mod vk {
    /// `VK_OEM_PLUS`: the `=`/`+` key on a US keyboard.
    pub(super) const OEM_PLUS: u32 = 0xBB;
    /// `VK_ADD`: the numeric keypad `+`.
    pub(super) const ADD: u32 = 0x6B;
    /// `VK_OEM_MINUS`: the `-`/`_` key on a US keyboard.
    pub(super) const OEM_MINUS: u32 = 0xBD;
    /// `VK_SUBTRACT`: the numeric keypad `-`.
    pub(super) const SUBTRACT: u32 = 0x6D;
    /// `VK_0`: the top-row `0`, the same code as the ASCII digit.
    pub(super) const KEY_0: u32 = 0x30;
    /// `VK_NUMPAD0`: the numeric keypad `0`.
    pub(super) const NUMPAD0: u32 = 0x60;
    /// `VK_F5`.
    pub(super) const F5: u32 = 0x74;
    /// `VK_R`, the same code as the ASCII letter, per Win32 convention.
    pub(super) const KEY_R: u32 = 0x52;
    /// `VK_TAB`.
    pub(super) const TAB: u32 = 0x09;
    /// `VK_B`, the same code as the ASCII letter, per Win32 convention.
    pub(super) const KEY_B: u32 = 0x42;
    /// `VK_P`, likewise.
    pub(super) const KEY_P: u32 = 0x50;
    /// `VK_S`, likewise.
    pub(super) const KEY_S: u32 = 0x53;
    /// `VK_1`: the top-row `1`, the same code as the ASCII digit.
    pub(super) const KEY_1: u32 = 0x31;
    /// `VK_2`: the top-row `2`.
    pub(super) const KEY_2: u32 = 0x32;
    /// `VK_4`: the top-row `4`.
    pub(super) const KEY_4: u32 = 0x34;
}

/// The `gdk::Key` a Win32 virtual-key code means to the window's own
/// shortcuts, or `None` for a key the window does not bind.
///
/// Win32 reports the *unshifted* virtual key, so `Ctrl`+`Shift`+`P` arrives
/// here as [`vk::KEY_P`] and leaves as `gdk::Key::p` with `SHIFT_MASK` set on
/// the modifiers built beside it, where GDK on X11 and Wayland would have
/// delivered the shifted keyval `gdk::Key::P`. That asymmetry stops here:
/// `window::shortcut_for` matches both letter cases and takes the direction
/// from the modifier bit, never from the keyval's case, which is what lets
/// one table serve both engines (`FR.26.7`, code standards rule 18).
///
/// `VK_3` is absent for the same reason `Ctrl`+`3` maps to nothing on Linux —
/// the phone arrangement has no key (`FR.26.2`).
pub(crate) fn gdk_key_for_virtual_key(virtual_key: u32) -> Option<gdk::Key> {
    match virtual_key {
        vk::OEM_PLUS => Some(gdk::Key::plus),
        vk::ADD => Some(gdk::Key::KP_Add),
        vk::OEM_MINUS => Some(gdk::Key::minus),
        vk::SUBTRACT => Some(gdk::Key::KP_Subtract),
        vk::KEY_0 => Some(gdk::Key::_0),
        vk::NUMPAD0 => Some(gdk::Key::KP_0),
        vk::F5 => Some(gdk::Key::F5),
        vk::KEY_R => Some(gdk::Key::r),
        vk::TAB => Some(gdk::Key::Tab),
        vk::KEY_B => Some(gdk::Key::b),
        vk::KEY_P => Some(gdk::Key::p),
        vk::KEY_S => Some(gdk::Key::s),
        vk::KEY_1 => Some(gdk::Key::_1),
        vk::KEY_2 => Some(gdk::Key::_2),
        vk::KEY_4 => Some(gdk::Key::_4),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_zoom_shortcut_virtual_key_maps_to_the_keyval_the_window_matches() {
        let mapped = [
            vk::OEM_PLUS,
            vk::ADD,
            vk::OEM_MINUS,
            vk::SUBTRACT,
            vk::KEY_0,
            vk::NUMPAD0,
        ]
        .map(gdk_key_for_virtual_key);

        assert_eq!(
            mapped,
            [
                Some(gdk::Key::plus),
                Some(gdk::Key::KP_Add),
                Some(gdk::Key::minus),
                Some(gdk::Key::KP_Subtract),
                Some(gdk::Key::_0),
                Some(gdk::Key::KP_0),
            ]
        );
    }

    #[test]
    fn an_unbound_virtual_key_maps_to_none() {
        assert_eq!(gdk_key_for_virtual_key(0x41), None); // 'A'
    }

    #[test]
    fn tab_virtual_key_maps_to_the_keyval_the_window_matches() {
        assert_eq!(gdk_key_for_virtual_key(vk::TAB), Some(gdk::Key::Tab));
    }

    #[test]
    fn every_window_chord_virtual_key_maps_to_the_keyval_the_window_matches() {
        let mapped = [
            vk::KEY_B,
            vk::KEY_P,
            vk::KEY_S,
            vk::KEY_1,
            vk::KEY_2,
            vk::KEY_4,
        ]
        .map(gdk_key_for_virtual_key);

        assert_eq!(
            mapped,
            [
                Some(gdk::Key::b),
                Some(gdk::Key::p),
                Some(gdk::Key::s),
                Some(gdk::Key::_1),
                Some(gdk::Key::_2),
                Some(gdk::Key::_4),
            ]
        );
    }

    /// The phone arrangement has no key, on either engine (`FR.26.2`).
    #[test]
    fn the_digit_three_virtual_key_maps_to_none() {
        assert_eq!(gdk_key_for_virtual_key(0x33), None); // 'VK_3'
    }

    /// The property that makes one shortcut table serve both engines: what
    /// Win32 delivers, run through the mapping, decides the same shortcut
    /// that the keyval GDK delivers on Linux decides — including for the two
    /// chords where the direction lives in the `Shift` bit and the letter
    /// case differs between the platforms.
    #[test]
    fn the_mapped_keyvals_decide_the_same_shortcuts_the_linux_keyvals_do() {
        use crate::window::shortcut_for;
        use gtk4::gdk::ModifierType;

        let ctrl = ModifierType::CONTROL_MASK;
        let ctrl_shift = ModifierType::CONTROL_MASK | ModifierType::SHIFT_MASK;

        let from_windows: Vec<_> = [
            (vk::KEY_B, ctrl),
            (vk::KEY_1, ctrl),
            (vk::KEY_2, ctrl),
            (vk::KEY_4, ctrl),
            (vk::KEY_P, ctrl),
            (vk::KEY_S, ctrl),
            (vk::KEY_P, ctrl_shift),
            (vk::KEY_S, ctrl_shift),
        ]
        .into_iter()
        .map(|(key, modifiers)| {
            gdk_key_for_virtual_key(key).and_then(|key| shortcut_for(key, modifiers))
        })
        .collect();

        // The same eight chords as a Linux keyboard delivers them: the two
        // shifted ones arrive here with the *shifted* keyval.
        let from_linux: Vec<_> = [
            (gdk::Key::b, ctrl),
            (gdk::Key::_1, ctrl),
            (gdk::Key::_2, ctrl),
            (gdk::Key::_4, ctrl),
            (gdk::Key::p, ctrl),
            (gdk::Key::s, ctrl),
            (gdk::Key::P, ctrl_shift),
            (gdk::Key::S, ctrl_shift),
        ]
        .into_iter()
        .map(|(key, modifiers)| shortcut_for(key, modifiers))
        .collect();

        assert!(from_windows.iter().all(Option::is_some));
        assert_eq!(from_windows, from_linux);
    }
}
