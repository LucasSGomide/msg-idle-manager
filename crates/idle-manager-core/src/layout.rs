//! Arrangements, the slots they expose, and the pure placement pass that
//! decides where every session goes when the layout or the session list
//! changes.

use std::collections::HashMap;

use crate::session::{SessionId, Visibility};

/// One of the three ways the window divides itself between sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    /// One session fills the window.
    #[default]
    Single,
    /// Two sessions side by side.
    SideBySide,
    /// Four sessions in a two-by-two grid.
    Grid,
}

impl Layout {
    /// How many slots this layout shows at once: 1, 2 or 4.
    #[must_use]
    pub fn slot_count(self) -> usize {
        match self {
            Layout::Single => 1,
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

/// Where each session goes for `layout`, given the sessions in the order they
/// were added and the slot each last occupied.
///
/// Every session in `order` appears in the result exactly once. A session
/// whose remembered slot still exists and is free keeps it; the rest fill the
/// lowest-numbered free slots in add order; any that do not fit are
/// [`Visibility::OffGrid`]. The pass reads no clock and no randomness, so the
/// same arguments always produce the same map.
#[must_use]
pub fn arrange<S: std::hash::BuildHasher>(
    layout: Layout,
    order: &[SessionId],
    remembered: &HashMap<SessionId, SlotId, S>,
) -> HashMap<SessionId, Visibility> {
    let mut placement = HashMap::with_capacity(order.len());
    let mut taken = vec![false; layout.slot_count()];

    for id in order {
        let Some(slot) = remembered.get(id).copied() else {
            continue;
        };
        if layout.contains(slot) && !taken[slot.index()] {
            taken[slot.index()] = true;
            placement.insert(id.clone(), Visibility::InSlot(slot));
        }
    }

    for id in order {
        if placement.contains_key(id) {
            continue;
        }
        match taken.iter().position(|&occupied| !occupied) {
            Some(index) => {
                taken[index] = true;
                placement.insert(id.clone(), Visibility::InSlot(SlotId::new(index)));
            }
            None => {
                placement.insert(id.clone(), Visibility::OffGrid);
            }
        }
    }

    placement
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(names: &[&str]) -> Vec<SessionId> {
        names.iter().map(|name| SessionId::new(*name)).collect()
    }

    #[test]
    fn each_layout_exposes_its_own_slot_count() {
        let counts: Vec<usize> = [Layout::Single, Layout::SideBySide, Layout::Grid]
            .into_iter()
            .map(Layout::slot_count)
            .collect();

        assert_eq!(counts, vec![1, 2, 4]);
    }

    #[test]
    fn a_remembered_slot_that_still_exists_is_kept() {
        let order = ids(&["a", "b"]);
        let remembered = HashMap::from([
            (order[0].clone(), SlotId::new(1)),
            (order[1].clone(), SlotId::new(0)),
        ]);

        let placement = arrange(Layout::SideBySide, &order, &remembered);

        assert_eq!(placement[&order[0]], Visibility::InSlot(SlotId::new(1)));
        assert_eq!(placement[&order[1]], Visibility::InSlot(SlotId::new(0)));
    }

    #[test]
    fn a_session_whose_slot_is_gone_overflows_off_grid() {
        let order = ids(&["a", "b", "c", "d"]);
        let remembered = HashMap::from([
            (order[0].clone(), SlotId::new(0)),
            (order[1].clone(), SlotId::new(1)),
            (order[2].clone(), SlotId::new(2)),
            (order[3].clone(), SlotId::new(3)),
        ]);

        let placement = arrange(Layout::SideBySide, &order, &remembered);

        assert_eq!(placement[&order[2]], Visibility::OffGrid);
        assert_eq!(placement[&order[3]], Visibility::OffGrid);
    }

    #[test]
    fn a_collided_remembered_slot_falls_back_to_the_lowest_free_slot() {
        let order = ids(&["a", "b", "c"]);
        let remembered = HashMap::from([
            (order[0].clone(), SlotId::new(0)),
            (order[1].clone(), SlotId::new(1)),
            (order[2].clone(), SlotId::new(0)),
        ]);

        let placement = arrange(Layout::Grid, &order, &remembered);

        assert_eq!(placement[&order[2]], Visibility::InSlot(SlotId::new(2)));
    }

    #[test]
    fn the_same_inputs_produce_the_same_placement() {
        let order = ids(&["a", "b", "c", "d", "e"]);
        let remembered = HashMap::from([
            (order[0].clone(), SlotId::new(3)),
            (order[2].clone(), SlotId::new(1)),
        ]);

        let first = arrange(Layout::Grid, &order, &remembered);
        let second = arrange(Layout::Grid, &order, &remembered);

        assert_eq!(first, second);
    }
}
