//! What a game account is: the identity the program mints for it, the two
//! things the user typed, and whether it currently occupies a slot.

use std::collections::HashMap;

use crate::layout::{Layout, SlotId, arrange};

/// The identifier the program mints for a session.
///
/// Names the session's folder on disk, so it never changes once assigned and a
/// rename of the display name moves no files.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Wraps an existing identifier string. The book is the only place that
    /// mints new ones; this is for adapters and tests that already hold an id.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        SessionId(value.into())
    }

    /// The identifier as a string slice, for use as a path component.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Whether a session is shown in a numbered slot or is out of sight.
///
/// One field, three-free: there is no "hidden" boolean beside an optional slot,
/// because that would make a fourth, meaningless state representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Occupying a visible slot in the current layout.
    InSlot(SlotId),
    /// Running, but not shown anywhere in the window.
    OffGrid,
}

/// One game account: its minted identity, the name and start address the user
/// gave, and where it currently sits.
#[derive(Debug, Clone)]
pub struct Session {
    id: SessionId,
    display_name: String,
    start_address: String,
    visibility: Visibility,
}

impl Session {
    /// The minted identifier. Stable for the session's whole life.
    #[must_use]
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// The name the user typed for this account.
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    /// The address the game starts at.
    #[must_use]
    pub fn start_address(&self) -> &str {
        &self.start_address
    }

    /// Where this session currently sits.
    #[must_use]
    pub fn visibility(&self) -> Visibility {
        self.visibility
    }
}

/// The application's sessions in the order they were added, together with the
/// current layout and the slot a full-grid addition displaces.
///
/// The book is the only place a [`SessionId`] is minted, and every identifier
/// it returns is distinct from every one already in it.
#[derive(Debug)]
pub struct SessionBook {
    sessions: Vec<Session>,
    layout: Layout,
    focused: SlotId,
    remembered: HashMap<SessionId, SlotId>,
    minted: u64,
}

impl SessionBook {
    /// A new book with no sessions, arranged for the single-slot layout.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            layout: Layout::Single,
            focused: SlotId::FIRST,
            remembered: HashMap::new(),
            minted: 0,
        }
    }

    /// The sessions, in the order they were added.
    #[must_use]
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// The layout the book is currently arranged for.
    #[must_use]
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// The slot a full-grid addition displaces.
    #[must_use]
    pub fn focused(&self) -> SlotId {
        self.focused
    }

    /// Focus `slot`, so the next addition into a full grid lands there. A slot
    /// outside the current layout is ignored.
    pub fn set_focused(&mut self, slot: SlotId) {
        if self.layout.contains(slot) {
            self.focused = slot;
        }
    }

    /// Mints an identifier for a new account, places it, and returns the id.
    ///
    /// The session takes the lowest-numbered free slot of the current layout.
    /// If every slot is occupied it takes the focused slot, and that slot's
    /// previous occupant becomes [`Visibility::OffGrid`].
    pub fn add(&mut self, display_name: &str, start_address: &str) -> SessionId {
        self.minted += 1;
        let id = SessionId(format!("session-{:04}", self.minted));

        let occupied = self.occupied_slots();
        let free = self.layout.slots().find(|slot| !occupied.contains(slot));

        let visibility = if let Some(slot) = free {
            Visibility::InSlot(slot)
        } else {
            let target = self.focused;
            if let Some(displaced) = self
                .sessions
                .iter_mut()
                .find(|session| session.visibility == Visibility::InSlot(target))
            {
                displaced.visibility = Visibility::OffGrid;
            }
            Visibility::InSlot(target)
        };

        if let Visibility::InSlot(slot) = visibility {
            self.remembered.insert(id.clone(), slot);
        }

        self.sessions.push(Session {
            id: id.clone(),
            display_name: display_name.to_owned(),
            start_address: start_address.to_owned(),
            visibility,
        });

        id
    }

    /// Switches to `layout` and re-places every session.
    ///
    /// A session whose slot still exists stays in it. A session whose slot is
    /// gone becomes [`Visibility::OffGrid`] with its slot remembered, so
    /// growing the layout again returns it there — or to the lowest free slot
    /// if another session has since taken it. Display names and start addresses
    /// are untouched.
    pub fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
        if !layout.contains(self.focused) {
            self.focused = SlotId::FIRST;
        }

        let order: Vec<SessionId> = self
            .sessions
            .iter()
            .map(|session| session.id.clone())
            .collect();
        let placement = arrange(layout, &order, &self.remembered);

        for session in &mut self.sessions {
            let visibility = placement
                .get(&session.id)
                .copied()
                .unwrap_or(Visibility::OffGrid);
            session.visibility = visibility;
            if let Visibility::InSlot(slot) = visibility {
                self.remembered.insert(session.id.clone(), slot);
            }
        }
    }

    fn occupied_slots(&self) -> Vec<SlotId> {
        self.sessions
            .iter()
            .filter_map(|session| match session.visibility {
                Visibility::InSlot(slot) => Some(slot),
                Visibility::OffGrid => None,
            })
            .collect()
    }
}

impl Default for SessionBook {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minted_identifier_is_distinct_from_every_existing_one() {
        let mut book = SessionBook::new();

        let first = book.add("One", "https://example.test/one");
        let second = book.add("Two", "https://example.test/two");

        assert_ne!(first, second);
    }

    #[test]
    fn a_session_added_with_a_slot_free_takes_the_lowest_numbered_one() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);

        book.add("One", "https://example.test/one");
        let second = book.add("Two", "https://example.test/two");

        let placed = book
            .sessions()
            .iter()
            .find(|session| session.id() == &second)
            .expect("the second session is in the book");
        assert_eq!(placed.visibility(), Visibility::InSlot(SlotId::new(1)));
    }

    #[test]
    fn a_session_added_with_every_slot_full_displaces_the_focused_slot() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let first = book.add("One", "https://example.test/one");
        book.add("Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));

        book.add("Three", "https://example.test/three");

        let displaced = book
            .sessions()
            .iter()
            .find(|session| session.id() == &first)
            .expect("the first session is still in the book");
        assert_eq!(displaced.visibility(), Visibility::OffGrid);
    }

    #[test]
    fn the_displacing_session_takes_the_focused_slot() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        book.add("One", "https://example.test/one");
        book.add("Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));

        let third = book.add("Three", "https://example.test/three");

        let placed = book
            .sessions()
            .iter()
            .find(|session| session.id() == &third)
            .expect("the third session is in the book");
        assert_eq!(placed.visibility(), Visibility::InSlot(SlotId::new(0)));
    }

    #[test]
    fn shrinking_the_layout_leaves_the_sessions_that_still_fit_in_place() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let first = book.add("One", "https://example.test/one");
        let second = book.add("Two", "https://example.test/two");
        book.add("Three", "https://example.test/three");
        book.add("Four", "https://example.test/four");

        book.set_layout(Layout::SideBySide);

        let visibilities: Vec<Visibility> = [&first, &second]
            .into_iter()
            .map(|id| {
                book.sessions()
                    .iter()
                    .find(|session| session.id() == id)
                    .expect("session present")
                    .visibility()
            })
            .collect();
        assert_eq!(
            visibilities,
            vec![
                Visibility::InSlot(SlotId::new(0)),
                Visibility::InSlot(SlotId::new(1)),
            ]
        );
    }

    #[test]
    fn shrinking_the_layout_pushes_the_sessions_that_no_longer_fit_off_grid() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        book.add("One", "https://example.test/one");
        book.add("Two", "https://example.test/two");
        let third = book.add("Three", "https://example.test/three");
        let fourth = book.add("Four", "https://example.test/four");

        book.set_layout(Layout::SideBySide);

        let visibilities: Vec<Visibility> = [&third, &fourth]
            .into_iter()
            .map(|id| {
                book.sessions()
                    .iter()
                    .find(|session| session.id() == id)
                    .expect("session present")
                    .visibility()
            })
            .collect();
        assert_eq!(visibilities, vec![Visibility::OffGrid, Visibility::OffGrid]);
    }

    #[test]
    fn growing_the_layout_returns_an_off_grid_session_to_its_remembered_slot() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        book.add("One", "https://example.test/one");
        book.add("Two", "https://example.test/two");
        let third = book.add("Three", "https://example.test/three");
        book.set_layout(Layout::SideBySide);

        book.set_layout(Layout::Grid);

        let placed = book
            .sessions()
            .iter()
            .find(|session| session.id() == &third)
            .expect("session present");
        assert_eq!(placed.visibility(), Visibility::InSlot(SlotId::new(2)));
    }

    #[test]
    fn growing_the_layout_sends_a_session_whose_remembered_slot_was_taken_to_the_lowest_free_slot()
    {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        book.add("One", "https://example.test/one");
        book.add("Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));
        let third = book.add("Three", "https://example.test/three");

        book.set_layout(Layout::Grid);

        let placed = book
            .sessions()
            .iter()
            .find(|session| session.id() == &third)
            .expect("session present");
        assert_eq!(placed.visibility(), Visibility::InSlot(SlotId::new(2)));
    }

    #[test]
    fn a_placement_pass_leaves_names_and_addresses_untouched() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        book.add("Main account", "https://example.test/game");
        book.add("Alt account", "https://example.test/game?alt");

        book.set_layout(Layout::Single);

        let named: Vec<(&str, &str)> = book
            .sessions()
            .iter()
            .map(|session| (session.display_name(), session.start_address()))
            .collect();
        assert_eq!(
            named,
            vec![
                ("Main account", "https://example.test/game"),
                ("Alt account", "https://example.test/game?alt"),
            ]
        );
    }
}
