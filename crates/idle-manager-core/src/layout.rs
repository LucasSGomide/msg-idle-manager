//! Arrangements and the slots they expose.

use crate::session::SessionId;

/// One of the four ways the window divides itself between sessions.
///
/// `Mobile` is a layout and nothing more (Remote Access `FR.3.1`): one slot the
/// shape of a phone screen, entered and left through
/// [`crate::WorkspaceBook::enter_mobile_mode`] and never written to disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Layout {
    /// One session fills the window.
    #[default]
    Single,
    /// Two sessions side by side.
    SideBySide,
    /// Four sessions in a two-by-two grid.
    Grid,
    /// One session in a slot the size of a phone's viewport, centred on the
    /// window background; every other session is off-grid (roadmap item 13).
    Mobile,
}

impl Layout {
    /// How many slots this layout shows at once: 1, 2 or 4 — `Mobile` shows
    /// one, like `Single`.
    #[must_use]
    pub const fn slot_count(self) -> usize {
        match self {
            Layout::Single | Layout::Mobile => 1,
            Layout::SideBySide => 2,
            Layout::Grid => 4,
        }
    }

    /// The slots this layout exposes, lowest-numbered first.
    pub fn slots(self) -> impl Iterator<Item = SlotId> {
        (0..self.slot_count()).map(SlotId::new)
    }

    /// Whether `slot` is one of the slots this layout exposes.
    #[must_use]
    pub fn contains(self, slot: SlotId) -> bool {
        slot.index() < self.slot_count()
    }
}

/// A numbered visible position in the current layout, counted from zero.
///
/// A distinct type from every other identifier so a slot number can never be
/// passed where a session identifier belongs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SlotId(usize);

impl SlotId {
    /// The lowest slot, present in every layout.
    pub const FIRST: SlotId = SlotId(0);

    /// Wraps a zero-based slot number.
    #[must_use]
    pub fn new(index: usize) -> Self {
        SlotId(index)
    }

    /// The zero-based slot number.
    #[must_use]
    pub fn index(self) -> usize {
        self.0
    }
}

/// What a [`crate::SessionBook::move_to_slot`] call did.
///
/// A named reason rather than a bare boolean, so a caller can tell a swap from
/// a fill without inspecting the book itself (code standards rule 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveOutcome {
    /// The target position held another session, which now sits where the
    /// mover was.
    Swapped {
        /// The session that held the target position and now sits where the
        /// mover was.
        with: SessionId,
    },
    /// The target position was empty; the mover now sits there and nothing
    /// else moved.
    Filled,
    /// Nothing changed: the target is the mover's own position, is not a
    /// position the layout has, or the mover is unknown or not on the shown
    /// page.
    Unchanged,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_layout_exposes_its_own_slot_count() {
        let counts: Vec<usize> = [
            Layout::Single,
            Layout::SideBySide,
            Layout::Grid,
            Layout::Mobile,
        ]
        .into_iter()
        .map(Layout::slot_count)
        .collect();

        assert_eq!(counts, vec![1, 2, 4, 1]);
    }
}
