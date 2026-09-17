//! Arrangements, the slots they expose, and the pure placement pass that
//! decides where every session goes when the layout or the session list
//! changes.

use std::collections::HashMap;

use crate::session::{SessionId, Visibility};

/// One of the three ways the window divides itself between sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
    pub const fn slot_count(self) -> usize {
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

/// What a [`bring_into_focus`] call did, beyond the placement it produced.
///
/// An enum of the three things that can happen, never a boolean beside an
/// optional displaced id — that would make a fourth, meaningless state
/// representable (code standards rule 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The session was off-grid and the focused slot was taken: `displaced` is
    /// the session that gave the slot up and is now off-grid.
    Swapped {
        /// The session pushed out of the focused slot.
        displaced: SessionId,
    },
    /// The session was off-grid and the focused slot was empty: it moved in and
    /// nothing was pushed out.
    Filled,
    /// The session already held a slot: focus moved to that slot and every
    /// visibility is unchanged.
    Focused,
}

/// Where every session sits after one is brought into the focused slot, and
/// which of the three things that took.
///
/// Total by design: [`Self::visibility`] carries every session the call was
/// given, not just the one or two that moved, so the sidebar and the grid can
/// each redraw from it without their pictures drifting apart.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    visibility: HashMap<SessionId, Visibility>,
    focused: SlotId,
    outcome: Outcome,
}

impl Placement {
    /// The visibility of every session after the call, keyed by id.
    #[must_use]
    pub fn visibility(&self) -> &HashMap<SessionId, Visibility> {
        &self.visibility
    }

    /// The slot that is focused after the call.
    #[must_use]
    pub fn focused(&self) -> SlotId {
        self.focused
    }

    /// Which of the three things the call did.
    #[must_use]
    pub fn outcome(&self) -> &Outcome {
        &self.outcome
    }
}

/// Bring `session` into `focused`, given where every session currently sits.
///
/// One outcome per [`Outcome`] variant: `session` already holds a slot, so only
/// the focus moves and no visibility changes; `session` is off-grid and
/// `focused` is empty, so it fills it; `session` is off-grid and `focused` is
/// taken, so the two trade places. The returned [`Placement`] always names
/// every session in `current`. A `session` absent from `current` is left alone.
/// Reads no clock and no randomness.
#[must_use]
pub(crate) fn bring_into_focus<S: std::hash::BuildHasher>(
    current: &HashMap<SessionId, Visibility, S>,
    focused: SlotId,
    session: &SessionId,
) -> Placement {
    let mut visibility: HashMap<SessionId, Visibility> = current
        .iter()
        .map(|(id, seat)| (id.clone(), *seat))
        .collect();

    match current.get(session).copied() {
        // Already on screen: the focus follows it there, the seats do not move.
        Some(Visibility::InSlot(slot)) => Placement {
            visibility,
            focused: slot,
            outcome: Outcome::Focused,
        },
        // Out of sight: it takes the focused slot, trading with whoever holds it.
        Some(Visibility::OffGrid) => {
            let displaced = current
                .iter()
                .find_map(|(id, seat)| (*seat == Visibility::InSlot(focused)).then(|| id.clone()));

            visibility.insert(session.clone(), Visibility::InSlot(focused));
            let outcome = match displaced {
                Some(id) => {
                    visibility.insert(id.clone(), Visibility::OffGrid);
                    Outcome::Swapped { displaced: id }
                }
                None => Outcome::Filled,
            };

            Placement {
                visibility,
                focused,
                outcome,
            }
        }
        // Not a session in this book: nothing to do.
        None => Placement {
            visibility,
            focused,
            outcome: Outcome::Focused,
        },
    }
}

/// What a [`move_into_slot`] call did to the arrangement.
///
/// A separate type from [`Outcome`] on purpose: that type's `Swapped` means
/// the displaced session leaves the grid entirely, which a place-to-place
/// move never does — reusing it would make one variant mean two different
/// things (code standards rule 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveOutcome {
    /// `target` held another session, which now sits in the mover's old slot.
    Swapped {
        /// The session that held `target` and now sits where the mover was.
        with: SessionId,
    },
    /// `target` was empty; the mover now sits there and nothing else moved.
    Filled,
    /// Nothing changed: `target` is the mover's own slot, is not a slot the
    /// layout has, or the mover is off-grid or unknown.
    Unchanged,
}

/// Where every session sits after one is moved to a slot, and which of the
/// three [`MoveOutcome`] cases that was.
///
/// Total by design, like [`Placement`]: [`Self::visibility`] carries every
/// session the call was given, not just the one or two that moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Move {
    visibility: HashMap<SessionId, Visibility>,
    outcome: MoveOutcome,
}

impl Move {
    /// The visibility of every session after the call, keyed by id.
    #[must_use]
    pub(crate) fn visibility(&self) -> &HashMap<SessionId, Visibility> {
        &self.visibility
    }

    /// Which of the three [`MoveOutcome`] cases the call produced.
    #[must_use]
    pub(crate) fn outcome(&self) -> &MoveOutcome {
        &self.outcome
    }
}

/// Move `account` to `target`, given `layout` and where every session
/// currently sits.
///
/// [`MoveOutcome::Unchanged`] when `target` is `account`'s own slot, is not a
/// slot `layout` exposes, or `account` is off-grid or absent from `current`.
/// Otherwise `account` takes `target`: an empty target is
/// [`MoveOutcome::Filled`]; an occupied one trades places with `account`'s old
/// slot and is [`MoveOutcome::Swapped`], naming the session that now sits
/// where `account` was. The returned [`Move`] always names every session in
/// `current`. Reads no clock and no randomness.
#[must_use]
pub(crate) fn move_into_slot<S: std::hash::BuildHasher>(
    current: &HashMap<SessionId, Visibility, S>,
    layout: Layout,
    account: &SessionId,
    target: SlotId,
) -> Move {
    let visibility: HashMap<SessionId, Visibility> = current
        .iter()
        .map(|(id, seat)| (id.clone(), *seat))
        .collect();

    let moves = matches!(
        current.get(account).copied(),
        Some(Visibility::InSlot(slot)) if slot != target
    ) && layout.contains(target);

    if !moves {
        return Move {
            visibility,
            outcome: MoveOutcome::Unchanged,
        };
    }

    let mut visibility = visibility;
    let occupant = current
        .iter()
        .find_map(|(id, seat)| (*seat == Visibility::InSlot(target)).then(|| id.clone()));

    visibility.insert(account.clone(), Visibility::InSlot(target));
    let outcome = match occupant {
        Some(id) => {
            // The mover's old slot is still recorded under `current`, since
            // `visibility` has only been touched for `account` so far.
            let Some(Visibility::InSlot(source)) = current.get(account).copied() else {
                unreachable!("`moves` only holds when `account` is in a slot");
            };
            visibility.insert(id.clone(), Visibility::InSlot(source));
            MoveOutcome::Swapped { with: id }
        }
        None => MoveOutcome::Filled,
    };

    Move {
        visibility,
        outcome,
    }
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

    #[test]
    fn bringing_an_off_grid_session_into_an_occupied_focused_slot_swaps_the_two() {
        let order = ids(&["a", "b"]);
        let current = HashMap::from([
            (order[0].clone(), Visibility::OffGrid),
            (order[1].clone(), Visibility::InSlot(SlotId::new(0))),
        ]);

        let placement = bring_into_focus(&current, SlotId::new(0), &order[0]);

        assert_eq!(
            (
                placement.visibility()[&order[0]],
                placement.visibility()[&order[1]],
            ),
            (Visibility::InSlot(SlotId::new(0)), Visibility::OffGrid),
        );
    }

    #[test]
    fn a_swap_leaves_every_other_sessions_visibility_untouched() {
        let order = ids(&["a", "b", "c", "d"]);
        let current = HashMap::from([
            (order[0].clone(), Visibility::OffGrid),
            (order[1].clone(), Visibility::InSlot(SlotId::new(0))),
            (order[2].clone(), Visibility::InSlot(SlotId::new(1))),
            (order[3].clone(), Visibility::InSlot(SlotId::new(2))),
        ]);

        let placement = bring_into_focus(&current, SlotId::new(0), &order[0]);

        assert_eq!(
            (
                placement.visibility()[&order[2]],
                placement.visibility()[&order[3]],
            ),
            (
                Visibility::InSlot(SlotId::new(1)),
                Visibility::InSlot(SlotId::new(2)),
            ),
        );
    }

    #[test]
    fn bringing_an_off_grid_session_into_an_empty_focused_slot_moves_no_other_session() {
        let order = ids(&["a", "b"]);
        let current = HashMap::from([
            (order[0].clone(), Visibility::OffGrid),
            (order[1].clone(), Visibility::InSlot(SlotId::new(0))),
        ]);

        let placement = bring_into_focus(&current, SlotId::new(1), &order[0]);

        assert_eq!(
            (
                placement.visibility()[&order[0]],
                placement.visibility()[&order[1]],
            ),
            (
                Visibility::InSlot(SlotId::new(1)),
                Visibility::InSlot(SlotId::new(0)),
            ),
        );
    }

    #[test]
    fn activating_a_visible_session_moves_focus_to_its_slot() {
        let order = ids(&["a", "b"]);
        let current = HashMap::from([
            (order[0].clone(), Visibility::InSlot(SlotId::new(0))),
            (order[1].clone(), Visibility::InSlot(SlotId::new(1))),
        ]);

        let placement = bring_into_focus(&current, SlotId::new(0), &order[1]);

        assert_eq!(placement.focused(), SlotId::new(1));
    }

    #[test]
    fn activating_a_visible_session_changes_no_visibility() {
        let order = ids(&["a", "b"]);
        let current = HashMap::from([
            (order[0].clone(), Visibility::InSlot(SlotId::new(0))),
            (order[1].clone(), Visibility::InSlot(SlotId::new(1))),
        ]);

        let placement = bring_into_focus(&current, SlotId::new(0), &order[1]);

        assert_eq!(placement.visibility(), &current);
    }

    #[test]
    fn the_placement_names_every_session_and_an_outcome_for_each_case() {
        let order = ids(&["a", "b", "c"]);
        let seats = HashMap::from([
            (order[0].clone(), Visibility::OffGrid),
            (order[1].clone(), Visibility::InSlot(SlotId::new(0))),
            (order[2].clone(), Visibility::InSlot(SlotId::new(1))),
        ]);

        let swap = bring_into_focus(&seats, SlotId::new(0), &order[0]);
        let fill = bring_into_focus(&seats, SlotId::new(2), &order[0]);
        let focus = bring_into_focus(&seats, SlotId::new(0), &order[1]);

        assert_eq!(
            (
                swap.visibility().len(),
                swap.outcome().clone(),
                fill.outcome().clone(),
                focus.outcome().clone(),
            ),
            (
                3,
                Outcome::Swapped {
                    displaced: order[1].clone(),
                },
                Outcome::Filled,
                Outcome::Focused,
            ),
        );
    }

    #[test]
    fn moving_onto_an_occupied_slot_swaps_exactly_those_two_and_names_the_other_account() {
        let order = ids(&["a", "b", "c"]);
        let current = HashMap::from([
            (order[0].clone(), Visibility::InSlot(SlotId::new(0))),
            (order[1].clone(), Visibility::InSlot(SlotId::new(1))),
            (order[2].clone(), Visibility::InSlot(SlotId::new(2))),
        ]);

        let moved = move_into_slot(&current, Layout::Grid, &order[0], SlotId::new(1));

        assert_eq!(
            (
                moved.outcome().clone(),
                moved.visibility()[&order[0]],
                moved.visibility()[&order[1]],
                moved.visibility()[&order[2]],
            ),
            (
                MoveOutcome::Swapped {
                    with: order[1].clone(),
                },
                Visibility::InSlot(SlotId::new(1)),
                Visibility::InSlot(SlotId::new(0)),
                Visibility::InSlot(SlotId::new(2)),
            ),
        );
    }

    #[test]
    fn moving_onto_an_empty_slot_fills_it_and_leaves_the_source_empty() {
        let order = ids(&["a", "b"]);
        let current = HashMap::from([
            (order[0].clone(), Visibility::InSlot(SlotId::new(0))),
            (order[1].clone(), Visibility::InSlot(SlotId::new(1))),
        ]);

        let moved = move_into_slot(&current, Layout::Grid, &order[0], SlotId::new(2));

        assert_eq!(
            (moved.outcome().clone(), moved.visibility()[&order[0]]),
            (MoveOutcome::Filled, Visibility::InSlot(SlotId::new(2))),
        );
        assert!(
            moved
                .visibility()
                .values()
                .all(|seat| *seat != Visibility::InSlot(SlotId::new(0)))
        );
    }

    #[test]
    fn moving_onto_the_movers_own_slot_is_unchanged() {
        let order = ids(&["a"]);
        let current = HashMap::from([(order[0].clone(), Visibility::InSlot(SlotId::new(0)))]);

        let moved = move_into_slot(&current, Layout::Single, &order[0], SlotId::new(0));

        assert_eq!(
            (moved.outcome().clone(), moved.visibility().clone()),
            (MoveOutcome::Unchanged, current),
        );
    }

    #[test]
    fn moving_onto_a_slot_the_layout_does_not_have_is_unchanged() {
        let order = ids(&["a"]);
        let current = HashMap::from([(order[0].clone(), Visibility::InSlot(SlotId::new(0)))]);

        let moved = move_into_slot(&current, Layout::Single, &order[0], SlotId::new(3));

        assert_eq!(
            (moved.outcome().clone(), moved.visibility().clone()),
            (MoveOutcome::Unchanged, current),
        );
    }

    #[test]
    fn moving_an_off_grid_account_is_unchanged() {
        let order = ids(&["a", "b"]);
        let current = HashMap::from([
            (order[0].clone(), Visibility::OffGrid),
            (order[1].clone(), Visibility::InSlot(SlotId::new(0))),
        ]);

        let moved = move_into_slot(&current, Layout::SideBySide, &order[0], SlotId::new(1));

        assert_eq!(
            (moved.outcome().clone(), moved.visibility().clone()),
            (MoveOutcome::Unchanged, current),
        );
    }

    #[test]
    fn moving_an_unknown_id_is_unchanged() {
        let order = ids(&["a"]);
        let current = HashMap::from([(order[0].clone(), Visibility::InSlot(SlotId::new(0)))]);
        let unknown = SessionId::new("unknown");

        let moved = move_into_slot(&current, Layout::SideBySide, &unknown, SlotId::new(1));

        assert_eq!(
            (moved.outcome().clone(), moved.visibility().clone()),
            (MoveOutcome::Unchanged, current),
        );
    }
}
