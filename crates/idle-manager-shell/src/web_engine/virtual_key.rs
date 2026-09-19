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
}

/// The `gdk::Key` a Win32 virtual-key code means to the window's own
/// shortcuts, or `None` for a key the window does not bind.
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
}
