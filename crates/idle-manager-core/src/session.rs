//! What a game account is: the identity the program mints for it, the two
//! things the user typed, and whether it currently occupies a slot.

use std::collections::HashMap;

use crate::layout::{Layout, Placement, SlotId, arrange, bring_into_focus};
use crate::preset::{Preset, ZoomLevel};
use crate::workspace::{Account, SavedLiveness, Workspace};

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

/// Whether a session's rendering process is running, stopped, or on its way up.
///
/// A second switch entirely independent of [`Visibility`]: every combination is
/// legal, because whether an account costs memory and whether it holds a place
/// on screen are two separate choices the user makes (code standards rule 1).
/// `Starting` is the interval between unparking and the shell's first paint —
/// modelling it in the domain rather than as a flag on a button is what makes a
/// double-press impossible in every caller at once. `Queued` is one step
/// earlier still: the user wants the account up and nothing has started it yet,
/// which is the state every running account is restored into so the shell can
/// bring them back one at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    /// The rendering process is running.
    Live,
    /// The rendering process is stopped; the session's data on disk is untouched.
    Parked,
    /// No page has painted yet — either just unparked, or reloading after a
    /// change that only takes effect on the next load, such as keep-awake.
    Starting,
    /// The user wants this account running and its turn has not come: restored
    /// from a saved workspace, waiting for the start queue. Distinct from
    /// `Starting`, which means a view already exists.
    Queued,
}

/// The sizes an account's owner has actually chosen, at most one per
/// arrangement, holding nothing for an arrangement never adjusted.
///
/// A named type rather than a bare `HashMap` so the [`crate::ZoomMemory`] port
/// can say what it carries (naming rules 6, 9). Empty is the normal state of a
/// fresh account, whose size follows its preset baseline (`FR.12.1`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RememberedZoom(HashMap<Layout, ZoomLevel>);

impl RememberedZoom {
    /// A set with nothing chosen for any arrangement.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The chosen size for `layout`, or `None` when its owner has never changed
    /// the size in that arrangement.
    #[must_use]
    pub fn get(&self, layout: Layout) -> Option<ZoomLevel> {
        self.0.get(&layout).copied()
    }

    /// Records `zoom` as the chosen size for `layout`, replacing any earlier
    /// choice for it.
    pub fn set(&mut self, layout: Layout, zoom: ZoomLevel) {
        self.0.insert(layout, zoom);
    }

    /// Drops the chosen size for `layout`; a no-op when none was chosen.
    pub fn clear(&mut self, layout: Layout) {
        self.0.remove(&layout);
    }

    /// Whether no size has been chosen for any arrangement.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Each chosen `(arrangement, size)` pair, in no particular order — for an
    /// adapter writing the set to disk.
    pub fn entries(&self) -> impl Iterator<Item = (Layout, ZoomLevel)> + '_ {
        self.0.iter().map(|(layout, zoom)| (*layout, *zoom))
    }
}

impl FromIterator<(Layout, ZoomLevel)> for RememberedZoom {
    fn from_iter<I: IntoIterator<Item = (Layout, ZoomLevel)>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// One game account: its minted identity, the name and start address it was
/// created with, where it currently sits, and the view settings it carries from
/// its preset.
#[derive(Debug, Clone)]
pub struct Session {
    id: SessionId,
    display_name: String,
    start_address: String,
    visibility: Visibility,
    liveness: Liveness,
    is_kept_awake: bool,
    browser_identity: Option<String>,
    /// The size the game's file asked for, copied at creation and never changed
    /// by a gesture — the baseline `Ctrl`+`0` returns to (`FR.11.2`).
    preset_zoom: ZoomLevel,
    /// The sizes the owner has chosen since, at most one per arrangement.
    /// Empty until a gesture records one (`FR.12.1`).
    remembered_zoom: RememberedZoom,
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

    /// Whether this session is running, parked, or starting up.
    #[must_use]
    pub fn liveness(&self) -> Liveness {
        self.liveness
    }

    /// Whether this session must keep running at full speed even while the
    /// shell believes nobody is looking at it.
    #[must_use]
    pub fn is_kept_awake(&self) -> bool {
        self.is_kept_awake
    }

    /// The identity the engine should present for this account, or `None` for
    /// the engine's own. Copied from the preset at creation; a typed-address
    /// account carries `None` (`FR.10.5`).
    #[must_use]
    pub fn browser_identity(&self) -> Option<&str> {
        self.browser_identity.as_deref()
    }

    /// The size the game's file asked for, copied at creation and never changed
    /// by a gesture. A typed-address account carries [`ZoomLevel::DEFAULT`].
    /// This is the baseline, not necessarily the size drawn now — for that ask
    /// [`Session::zoom_for`].
    #[must_use]
    pub fn preset_zoom(&self) -> ZoomLevel {
        self.preset_zoom
    }

    /// The size to draw this account's page at in `layout`: the size its owner
    /// chose for that arrangement if there is one, the baseline otherwise. The
    /// whole resolution rule, in one place (`FR.12.1`).
    #[must_use]
    pub fn zoom_for(&self, layout: Layout) -> ZoomLevel {
        self.remembered_zoom.get(layout).unwrap_or(self.preset_zoom)
    }

    /// The sizes this account's owner has chosen, per arrangement — for an
    /// adapter about to write them down (`FR.12.5`). Empty until a gesture
    /// records one.
    #[must_use]
    pub fn remembered_zoom(&self) -> &RememberedZoom {
        &self.remembered_zoom
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

    /// Rebuilds a book from a saved [`Workspace`].
    ///
    /// The accounts come back in the order the workspace held them. One saved
    /// as running becomes [`Liveness::Queued`] — nothing is running yet — and
    /// one saved as parked stays [`Liveness::Parked`], costing nothing. The
    /// saved layout becomes active and each account's remembered slot is
    /// seeded, so visibility is normalised through the same placement a layout
    /// switch uses: an account whose saved slot the layout cannot show lands
    /// off-grid and returns to that slot when a layout with it is chosen
    /// (`FR.3.2`). Identifier minting resumes above the highest restored id, so
    /// the next account added collides with none of them.
    #[must_use]
    pub fn restore(workspace: Workspace) -> Self {
        let Workspace { accounts, layout } = workspace;

        let minted = accounts
            .iter()
            .filter_map(|account| account.id.as_str().strip_prefix("session-"))
            .filter_map(|suffix| suffix.parse::<u64>().ok())
            .max()
            .unwrap_or(0);

        let mut remembered: HashMap<SessionId, SlotId> = HashMap::new();
        for account in &accounts {
            let slot = account.remembered_slot.or(match account.visibility {
                Visibility::InSlot(slot) => Some(slot),
                Visibility::OffGrid => None,
            });
            if let Some(slot) = slot {
                remembered.insert(account.id.clone(), slot);
            }
        }

        let order: Vec<SessionId> = accounts.iter().map(|account| account.id.clone()).collect();
        let placement = arrange(layout, &order, &remembered);

        let sessions = accounts
            .into_iter()
            .map(|account| Session {
                visibility: placement
                    .get(&account.id)
                    .copied()
                    .unwrap_or(Visibility::OffGrid),
                liveness: match account.liveness {
                    SavedLiveness::Running => Liveness::Queued,
                    SavedLiveness::Parked => Liveness::Parked,
                },
                id: account.id,
                display_name: account.display_name,
                start_address: account.start_address,
                is_kept_awake: account.is_kept_awake,
                browser_identity: account.browser_identity,
                preset_zoom: account.zoom,
                remembered_zoom: RememberedZoom::new(),
            })
            .collect();

        Self {
            sessions,
            layout,
            focused: SlotId::FIRST,
            remembered,
            minted,
        }
    }

    /// The sessions, in the order they were added.
    #[must_use]
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// The whole arrangement as one value, ready to be saved.
    ///
    /// Accounts in add order, each carrying its name, address, zoom, identity,
    /// keep-awake flag, and where it sits, plus the active layout. A
    /// [`Liveness::Starting`] or [`Liveness::Queued`] account is reported as
    /// [`SavedLiveness::Running`]: those describe a moment, not a wish.
    #[must_use]
    pub fn workspace(&self) -> Workspace {
        let accounts = self
            .sessions
            .iter()
            .map(|session| Account {
                id: session.id.clone(),
                display_name: session.display_name.clone(),
                start_address: session.start_address.clone(),
                liveness: match session.liveness {
                    Liveness::Parked => SavedLiveness::Parked,
                    Liveness::Live | Liveness::Starting | Liveness::Queued => {
                        SavedLiveness::Running
                    }
                },
                visibility: session.visibility,
                remembered_slot: self.remembered.get(&session.id).copied(),
                is_kept_awake: session.is_kept_awake,
                browser_identity: session.browser_identity.clone(),
                zoom: session.preset_zoom,
            })
            .collect();

        Workspace {
            accounts,
            layout: self.layout,
        }
    }

    /// The queued accounts in book order, and nothing else.
    ///
    /// The order the start queue brings restored accounts back in. Parked
    /// accounts are absent entirely — that is the whole point of parking
    /// surviving a restart (`FR.8.2`).
    #[must_use]
    pub fn start_order(&self) -> Vec<SessionId> {
        self.sessions
            .iter()
            .filter(|session| session.liveness == Liveness::Queued)
            .map(|session| session.id.clone())
            .collect()
    }

    /// How many accounts are running right now — the count the sidebar footer
    /// shows beside the aggregate memory figure (`FR.7.1`).
    ///
    /// Only [`Liveness::Live`] accounts: a parked account is running nothing, a
    /// queued or starting one has no painted page yet. This comes from the book
    /// and never from the kernel — asking the operating system how many
    /// rendering processes exist would answer a different question and answer it
    /// worse, because a process that has died and not been reaped is not an
    /// account anybody is running.
    #[must_use]
    pub fn live_session_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|session| session.liveness == Liveness::Live)
            .count()
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

    /// Mints an identifier for a new account created from a typed address,
    /// places it, and returns the id.
    ///
    /// The session takes the lowest-numbered free slot of the current layout.
    /// If every slot is occupied it takes the focused slot, and that slot's
    /// previous occupant becomes [`Visibility::OffGrid`]. It carries no browser
    /// identity and the default zoom — everything a preset would have supplied
    /// falls back here.
    pub fn add(&mut self, display_name: &str, start_address: &str) -> SessionId {
        let id = self.mint_id();
        let visibility = self.place(&id);

        self.sessions.push(Session {
            id: id.clone(),
            display_name: display_name.to_owned(),
            start_address: start_address.to_owned(),
            visibility,
            liveness: Liveness::Live,
            is_kept_awake: false,
            browser_identity: None,
            preset_zoom: ZoomLevel::DEFAULT,
            remembered_zoom: RememberedZoom::new(),
        });

        id
    }

    /// Mints an identifier for a new account playing `preset`'s game under the
    /// name `account_name`, places it exactly as [`SessionBook::add`] does, and
    /// copies the preset's start address, zoom and browser identity onto it.
    ///
    /// The preset's keep-awake default is applied through
    /// [`SessionBook::set_keep_awake`] — the one path that sets that flag —
    /// rather than written directly. That call moves a live account to
    /// [`Liveness::Starting`], so this follows it with
    /// [`SessionBook::mark_started`]: a brand-new account has no page to reload,
    /// its first view is built carrying the setting already, and it ends
    /// [`Liveness::Live`] like any other. Both are existing transitions; no
    /// field is written behind their backs.
    pub fn add_from_preset(&mut self, account_name: &str, preset: &Preset) -> SessionId {
        let id = self.mint_id();
        let visibility = self.place(&id);

        self.sessions.push(Session {
            id: id.clone(),
            display_name: account_name.to_owned(),
            start_address: preset.start_address.clone(),
            visibility,
            liveness: Liveness::Live,
            is_kept_awake: false,
            browser_identity: preset.browser_identity.clone(),
            preset_zoom: preset.zoom,
            remembered_zoom: RememberedZoom::new(),
        });

        self.set_keep_awake(&id, preset.keep_awake_default);
        self.mark_started(&id);

        id
    }

    fn mint_id(&mut self) -> SessionId {
        self.minted += 1;
        SessionId(format!("session-{:04}", self.minted))
    }

    /// Places `id` into the lowest free slot of the current layout, or into the
    /// focused slot with its occupant displaced off-grid when the grid is full,
    /// and remembers the slot it landed in.
    fn place(&mut self, id: &SessionId) -> Visibility {
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

        visibility
    }

    /// Park `session`: its liveness becomes [`Liveness::Parked`], and its
    /// visibility is left exactly as it was — a parked account keeps its place.
    /// Parking an already-parked session changes nothing. Returns the session's
    /// liveness after the call; an id not in the book changes nothing and
    /// returns [`Liveness::Live`].
    pub fn park(&mut self, session: &SessionId) -> Liveness {
        self.set_liveness(session, Liveness::Parked)
    }

    /// Unpark `session`: a parked *or* queued session becomes
    /// [`Liveness::Starting`], because no page has painted yet — a queued
    /// account is one the start queue has just reached. A live or
    /// already-starting session is left as it is. Visibility is never touched.
    /// Returns the session's liveness after the call.
    pub fn unpark(&mut self, session: &SessionId) -> Liveness {
        let Some(current) = self.liveness_of(session) else {
            return Liveness::Live;
        };
        if current == Liveness::Parked || current == Liveness::Queued {
            self.set_liveness(session, Liveness::Starting)
        } else {
            current
        }
    }

    /// End `session`'s starting interval once the shell reports its first
    /// paint: [`Liveness::Starting`] becomes [`Liveness::Live`]. Any other
    /// state is left as it is. Visibility is never touched. Returns the
    /// session's liveness after the call.
    pub fn mark_started(&mut self, session: &SessionId) -> Liveness {
        let Some(current) = self.liveness_of(session) else {
            return Liveness::Live;
        };
        if current == Liveness::Starting {
            self.set_liveness(session, Liveness::Live)
        } else {
            current
        }
    }

    /// Sets `session`'s keep-awake flag to `value` and reports whether that
    /// changed anything.
    ///
    /// The shell's reaction to a change is a page reload, which is expensive
    /// and throws away whatever the page was doing, so the caller must be able
    /// to skip it for a set that set nothing. A change that lands on a live
    /// session also moves it to [`Liveness::Starting`], because the shell
    /// reloads its page and that is exactly the "no page has painted yet"
    /// interval; a parked or already-starting session keeps its liveness, since
    /// it has no page to reload. An id not in the book changes nothing and
    /// returns `false`.
    pub fn set_keep_awake(&mut self, session: &SessionId, value: bool) -> bool {
        let Some(s) = self.sessions.iter_mut().find(|s| &s.id == session) else {
            return false;
        };
        if s.is_kept_awake == value {
            return false;
        }

        s.is_kept_awake = value;
        if s.liveness == Liveness::Live {
            s.liveness = Liveness::Starting;
        }

        true
    }

    /// Installs a whole set of remembered sizes onto `account`, replacing
    /// whatever it held, for the one caller that has just read one off disk.
    /// An id the book does not hold changes nothing (`FR.12.8`).
    pub fn restore_zoom(&mut self, account: &SessionId, remembered: RememberedZoom) {
        if let Some(session) = self.sessions.iter_mut().find(|s| &s.id == account) {
            session.remembered_zoom = remembered;
        }
    }

    /// Steps `account`'s size one step larger for the book's **current**
    /// layout, records it against that arrangement only, and returns the new
    /// size. `None` for an id the book does not hold. Liveness and visibility
    /// are untouched — a zoom is not a reload (`FR.11.5`).
    pub fn zoom_in(&mut self, account: &SessionId) -> Option<ZoomLevel> {
        self.step_zoom(account, ZoomLevel::stepped_in)
    }

    /// Steps `account`'s size one step smaller for the book's current layout,
    /// otherwise exactly [`SessionBook::zoom_in`].
    pub fn zoom_out(&mut self, account: &SessionId) -> Option<ZoomLevel> {
        self.step_zoom(account, ZoomLevel::stepped_out)
    }

    fn step_zoom(
        &mut self,
        account: &SessionId,
        step: impl FnOnce(ZoomLevel) -> ZoomLevel,
    ) -> Option<ZoomLevel> {
        let layout = self.layout;
        let session = self.sessions.iter_mut().find(|s| &s.id == account)?;
        let stepped = step(session.zoom_for(layout));
        session.remembered_zoom.set(layout, stepped);
        Some(stepped)
    }

    /// Drops `account`'s remembered size for the book's current layout and
    /// returns the baseline the game file supplied. Entries for the other
    /// arrangements are left in place. `None` for an id the book does not hold
    /// (`FR.11.2`).
    pub fn reset_zoom(&mut self, account: &SessionId) -> Option<ZoomLevel> {
        let layout = self.layout;
        let session = self.sessions.iter_mut().find(|s| &s.id == account)?;
        session.remembered_zoom.clear(layout);
        Some(session.preset_zoom)
    }

    /// The account sitting in the focused slot, or `None` when that slot holds
    /// nothing. The one place the shell asks which account a keyboard gesture
    /// acts on.
    #[must_use]
    pub fn focused_session(&self) -> Option<&Session> {
        let focused = Visibility::InSlot(self.focused);
        self.sessions
            .iter()
            .find(|session| session.visibility == focused)
    }

    fn liveness_of(&self, session: &SessionId) -> Option<Liveness> {
        self.sessions
            .iter()
            .find(|s| &s.id == session)
            .map(Session::liveness)
    }

    fn set_liveness(&mut self, session: &SessionId, liveness: Liveness) -> Liveness {
        match self.sessions.iter_mut().find(|s| &s.id == session) {
            Some(s) => {
                s.liveness = liveness;
                liveness
            }
            None => Liveness::Live,
        }
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

    /// Bring `session` into the focused slot and report where every session now
    /// sits.
    ///
    /// If `session` already holds a slot, focus moves there and nothing else
    /// changes. If the focused slot is empty, `session` fills it. If it is
    /// taken, `session` and the slot's occupant trade places, the occupant
    /// going off-grid. A `session` not in the book leaves the book untouched.
    /// See [`crate::Outcome`] for the three cases the returned [`Placement`]
    /// distinguishes.
    pub fn focus_session(&mut self, session: &SessionId) -> Placement {
        let current: HashMap<SessionId, Visibility> = self
            .sessions
            .iter()
            .map(|s| (s.id.clone(), s.visibility))
            .collect();

        let placement = bring_into_focus(&current, self.focused, session);

        self.focused = placement.focused();
        for s in &mut self.sessions {
            let Some(visibility) = placement.visibility().get(&s.id).copied() else {
                continue;
            };
            s.visibility = visibility;
            if let Visibility::InSlot(slot) = visibility {
                self.remembered.insert(s.id.clone(), slot);
            }
        }

        placement
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
    fn focusing_a_visible_session_moves_the_books_focus_to_its_slot() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        book.add("One", "https://example.test/one");
        let two = book.add("Two", "https://example.test/two");

        book.focus_session(&two);

        assert_eq!(book.focused(), SlotId::new(1));
    }

    #[test]
    fn focusing_an_off_grid_session_swaps_it_into_the_focused_slot_in_the_book() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let first = book.add("One", "https://example.test/one");
        book.add("Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));
        let third = book.add("Three", "https://example.test/three");

        book.focus_session(&first);

        let seats: Vec<Visibility> = [&first, &third]
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
            seats,
            vec![Visibility::InSlot(SlotId::new(0)), Visibility::OffGrid],
        );
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

    fn liveness_of(book: &SessionBook, id: &SessionId) -> Liveness {
        book.sessions()
            .iter()
            .find(|session| session.id() == id)
            .expect("session present")
            .liveness()
    }

    fn visibility_of(book: &SessionBook, id: &SessionId) -> Visibility {
        book.sessions()
            .iter()
            .find(|session| session.id() == id)
            .expect("session present")
            .visibility()
    }

    #[test]
    fn a_session_added_to_the_book_starts_live() {
        let mut book = SessionBook::new();

        let id = book.add("One", "https://example.test/one");

        assert_eq!(liveness_of(&book, &id), Liveness::Live);
    }

    #[test]
    fn parking_a_live_session_returns_parked() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");

        let state = book.park(&id);

        assert_eq!(state, Liveness::Parked);
    }

    #[test]
    fn parking_a_session_leaves_its_visibility_unchanged() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        let visibility = visibility_of(&book, &id);

        book.park(&id);

        assert_eq!(visibility_of(&book, &id), visibility);
    }

    #[test]
    fn unparking_a_parked_session_returns_starting() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.park(&id);

        let state = book.unpark(&id);

        assert_eq!(state, Liveness::Starting);
    }

    #[test]
    fn unparking_a_session_leaves_its_visibility_unchanged() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.park(&id);
        let visibility = visibility_of(&book, &id);

        book.unpark(&id);

        assert_eq!(visibility_of(&book, &id), visibility);
    }

    #[test]
    fn ending_the_starting_interval_returns_live() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.park(&id);
        book.unpark(&id);

        let state = book.mark_started(&id);

        assert_eq!(state, Liveness::Live);
    }

    #[test]
    fn parking_an_already_parked_session_returns_parked_and_changes_nothing_else() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.park(&id);
        let visibility = visibility_of(&book, &id);

        let state = book.park(&id);

        assert_eq!(
            (state, visibility_of(&book, &id)),
            (Liveness::Parked, visibility)
        );
    }

    #[test]
    fn unparking_a_live_session_returns_live_and_changes_nothing_else() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        let visibility = visibility_of(&book, &id);

        let state = book.unpark(&id);

        assert_eq!(
            (state, liveness_of(&book, &id), visibility_of(&book, &id)),
            (Liveness::Live, Liveness::Live, visibility)
        );
    }

    #[test]
    fn the_live_session_count_is_the_number_of_running_accounts() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        book.add("One", "https://example.test/one");
        book.add("Two", "https://example.test/two");

        assert_eq!(book.live_session_count(), 2);
    }

    #[test]
    fn a_parked_account_is_not_counted_among_the_live_sessions() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = book.add("One", "https://example.test/one");
        book.add("Two", "https://example.test/two");

        book.park(&one);

        assert_eq!(book.live_session_count(), 1);
    }

    fn is_kept_awake(book: &SessionBook, id: &SessionId) -> bool {
        book.sessions()
            .iter()
            .find(|session| session.id() == id)
            .expect("session present")
            .is_kept_awake()
    }

    #[test]
    fn an_account_added_to_the_book_starts_with_keep_awake_off() {
        let mut book = SessionBook::new();

        let id = book.add("One", "https://example.test/one");

        assert!(!is_kept_awake(&book, &id));
    }

    #[test]
    fn turning_keep_awake_on_for_an_account_that_had_it_off_returns_true() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");

        let changed = book.set_keep_awake(&id, true);

        assert!(changed);
    }

    #[test]
    fn turning_keep_awake_on_for_an_account_that_had_it_off_leaves_the_flag_on() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");

        book.set_keep_awake(&id, true);

        assert!(is_kept_awake(&book, &id));
    }

    #[test]
    fn setting_keep_awake_to_its_current_value_reports_no_change() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");

        let changed = book.set_keep_awake(&id, false);

        assert!(!changed);
    }

    #[test]
    fn setting_keep_awake_to_its_current_value_leaves_liveness_unchanged() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.park(&id);
        let liveness = liveness_of(&book, &id);

        book.set_keep_awake(&id, false);

        assert_eq!(liveness_of(&book, &id), liveness);
    }

    #[test]
    fn turning_keep_awake_on_for_a_live_account_leaves_it_starting() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");

        book.set_keep_awake(&id, true);

        assert_eq!(liveness_of(&book, &id), Liveness::Starting);
    }

    #[test]
    fn turning_keep_awake_on_for_a_parked_account_leaves_it_parked() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.park(&id);

        book.set_keep_awake(&id, true);

        assert_eq!(liveness_of(&book, &id), Liveness::Parked);
    }

    #[test]
    fn keep_awake_survives_a_layout_change_that_moves_the_account_between_slots() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let id = book.add("One", "https://example.test/one");
        book.add("Two", "https://example.test/two");
        book.add("Three", "https://example.test/three");
        book.add("Four", "https://example.test/four");
        book.set_keep_awake(&id, true);

        book.set_layout(Layout::SideBySide);
        book.set_layout(Layout::Grid);

        assert!(is_kept_awake(&book, &id));
    }

    #[test]
    fn keep_awake_survives_being_pushed_off_grid_and_brought_back_into_focus() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let first = book.add("One", "https://example.test/one");
        book.add("Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));
        book.add("Three", "https://example.test/three");
        book.set_keep_awake(&first, true);

        book.focus_session(&first);

        assert!(is_kept_awake(&book, &first));
    }

    #[test]
    fn keep_awake_survives_being_parked() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.set_keep_awake(&id, true);

        book.park(&id);

        assert!(is_kept_awake(&book, &id));
    }

    #[test]
    fn keep_awake_survives_being_unparked() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.set_keep_awake(&id, true);
        book.park(&id);

        book.unpark(&id);

        assert!(is_kept_awake(&book, &id));
    }

    #[test]
    fn setting_keep_awake_on_an_unknown_id_reports_no_change() {
        let mut book = SessionBook::new();
        let unknown = SessionId::new("session-9999");

        let changed = book.set_keep_awake(&unknown, true);

        assert!(!changed);
    }

    #[test]
    fn setting_keep_awake_on_an_unknown_id_leaves_the_book_untouched() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        let unknown = SessionId::new("session-9999");

        book.set_keep_awake(&unknown, true);

        assert!(!is_kept_awake(&book, &id));
    }

    fn session<'a>(book: &'a SessionBook, id: &SessionId) -> &'a Session {
        book.sessions()
            .iter()
            .find(|session| session.id() == id)
            .expect("session present")
    }

    fn a_preset() -> Preset {
        Preset {
            id: crate::PresetId::new("huntera"),
            display_name: "Huntera".to_owned(),
            start_address: "https://huntera.test/".to_owned(),
            browser_identity: Some("PresetUA/1.0".to_owned()),
            zoom: ZoomLevel::new(0.8).expect("0.8 is an accepted multiplier"),
            keep_awake_default: false,
        }
    }

    #[test]
    fn an_account_from_a_preset_carries_the_typed_name_not_the_games_display_name() {
        let mut book = SessionBook::new();

        let id = book.add_from_preset("Alt", &a_preset());

        assert_eq!(session(&book, &id).display_name(), "Alt");
    }

    #[test]
    fn an_account_from_a_preset_carries_the_presets_address_zoom_and_identity() {
        let mut book = SessionBook::new();
        let preset = a_preset();

        let id = book.add_from_preset("Alt", &preset);

        let account = session(&book, &id);
        assert_eq!(
            (
                account.start_address(),
                account.preset_zoom(),
                account.browser_identity(),
            ),
            (
                "https://huntera.test/",
                ZoomLevel::new(0.8).expect("accepted"),
                Some("PresetUA/1.0"),
            ),
        );
    }

    #[test]
    fn an_account_from_a_preset_with_no_identity_carries_none() {
        let mut book = SessionBook::new();
        let preset = Preset {
            browser_identity: None,
            ..a_preset()
        };

        let id = book.add_from_preset("Alt", &preset);

        assert_eq!(session(&book, &id).browser_identity(), None);
    }

    #[test]
    fn an_account_from_a_preset_whose_keep_awake_default_is_on_starts_kept_awake() {
        let mut book = SessionBook::new();
        let preset = Preset {
            keep_awake_default: true,
            ..a_preset()
        };

        let id = book.add_from_preset("Alt", &preset);

        assert!(session(&book, &id).is_kept_awake());
    }

    #[test]
    fn an_account_from_a_preset_with_keep_awake_off_is_live() {
        let mut book = SessionBook::new();
        let preset = Preset {
            keep_awake_default: false,
            ..a_preset()
        };

        let id = book.add_from_preset("Alt", &preset);

        assert_eq!(session(&book, &id).liveness(), Liveness::Live);
    }

    #[test]
    fn an_account_from_a_preset_with_keep_awake_on_is_live() {
        let mut book = SessionBook::new();
        let preset = Preset {
            keep_awake_default: true,
            ..a_preset()
        };

        let id = book.add_from_preset("Alt", &preset);

        assert_eq!(session(&book, &id).liveness(), Liveness::Live);
    }

    #[test]
    fn an_account_from_a_preset_takes_the_lowest_free_slot_like_a_typed_one() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        book.add("One", "https://example.test/one");

        let id = book.add_from_preset("Alt", &a_preset());

        assert_eq!(
            session(&book, &id).visibility(),
            Visibility::InSlot(SlotId::new(1))
        );
    }

    #[test]
    fn an_account_from_a_preset_into_a_full_grid_displaces_the_focused_slot() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let first = book.add("One", "https://example.test/one");
        book.add("Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));

        book.add_from_preset("Alt", &a_preset());

        assert_eq!(session(&book, &first).visibility(), Visibility::OffGrid);
    }

    #[test]
    fn an_account_from_a_typed_address_carries_no_identity_and_the_default_zoom() {
        let mut book = SessionBook::new();

        let id = book.add("One", "https://example.test/one");

        let account = session(&book, &id);
        assert_eq!(
            (account.browser_identity(), account.preset_zoom()),
            (None, ZoomLevel::DEFAULT),
        );
    }

    fn saved_account(id: &str, name: &str) -> Account {
        Account {
            id: SessionId::new(id),
            display_name: name.to_owned(),
            start_address: format!("https://example.test/{id}"),
            liveness: SavedLiveness::Running,
            visibility: Visibility::OffGrid,
            remembered_slot: None,
            is_kept_awake: false,
            browser_identity: None,
            zoom: ZoomLevel::DEFAULT,
        }
    }

    fn in_slot(account: Account, slot: usize) -> Account {
        Account {
            visibility: Visibility::InSlot(SlotId::new(slot)),
            remembered_slot: Some(SlotId::new(slot)),
            ..account
        }
    }

    #[test]
    fn restoring_a_workspace_yields_its_accounts_in_order_with_their_saved_settings() {
        let workspace = Workspace {
            accounts: vec![
                Account {
                    is_kept_awake: true,
                    browser_identity: Some("PresetUA/1.0".to_owned()),
                    zoom: ZoomLevel::new(0.8).expect("0.8 is an accepted multiplier"),
                    ..saved_account("session-0001", "Main")
                },
                saved_account("session-0002", "Alt"),
            ],
            layout: Layout::SideBySide,
        };

        let book = SessionBook::restore(workspace);

        let first = &book.sessions()[0];
        assert_eq!(
            (
                book.sessions()
                    .iter()
                    .map(Session::display_name)
                    .collect::<Vec<_>>(),
                first.start_address(),
                first.preset_zoom(),
                first.browser_identity(),
                first.is_kept_awake(),
                book.layout(),
            ),
            (
                vec!["Main", "Alt"],
                "https://example.test/session-0001",
                ZoomLevel::new(0.8).expect("0.8 is an accepted multiplier"),
                Some("PresetUA/1.0"),
                true,
                Layout::SideBySide,
            ),
        );
    }

    #[test]
    fn an_account_saved_as_running_comes_back_queued() {
        let workspace = Workspace {
            accounts: vec![Account {
                liveness: SavedLiveness::Running,
                ..saved_account("session-0001", "Main")
            }],
            layout: Layout::Single,
        };

        let book = SessionBook::restore(workspace);

        assert_eq!(book.sessions()[0].liveness(), Liveness::Queued);
    }

    #[test]
    fn an_account_saved_as_parked_comes_back_parked() {
        let workspace = Workspace {
            accounts: vec![Account {
                liveness: SavedLiveness::Parked,
                ..saved_account("session-0001", "Main")
            }],
            layout: Layout::Single,
        };

        let book = SessionBook::restore(workspace);

        assert_eq!(book.sessions()[0].liveness(), Liveness::Parked);
    }

    #[test]
    fn the_start_order_lists_the_queued_accounts_in_workspace_order() {
        let workspace = Workspace {
            accounts: vec![
                Account {
                    liveness: SavedLiveness::Running,
                    ..saved_account("session-0001", "A")
                },
                Account {
                    liveness: SavedLiveness::Parked,
                    ..saved_account("session-0002", "B")
                },
                Account {
                    liveness: SavedLiveness::Running,
                    ..saved_account("session-0003", "C")
                },
            ],
            layout: Layout::Grid,
        };

        let book = SessionBook::restore(workspace);

        let order = book.start_order();

        let ids: Vec<&str> = order.iter().map(SessionId::as_str).collect();
        assert_eq!(ids, vec!["session-0001", "session-0003"]);
    }

    #[test]
    fn a_parked_account_is_absent_from_the_start_order() {
        let workspace = Workspace {
            accounts: vec![Account {
                liveness: SavedLiveness::Parked,
                ..saved_account("session-0001", "Parked")
            }],
            layout: Layout::Single,
        };

        let book = SessionBook::restore(workspace);

        assert!(book.start_order().is_empty());
    }

    #[test]
    fn an_account_whose_saved_slot_the_layout_cannot_show_comes_back_off_grid_then_reclaims_it() {
        let workspace = Workspace {
            accounts: vec![
                in_slot(saved_account("session-0001", "A"), 0),
                in_slot(saved_account("session-0002", "B"), 1),
            ],
            layout: Layout::Single,
        };
        let mut book = SessionBook::restore(workspace);
        let while_single = book.sessions()[1].visibility();

        book.set_layout(Layout::SideBySide);

        assert_eq!(
            (while_single, book.sessions()[1].visibility()),
            (Visibility::OffGrid, Visibility::InSlot(SlotId::new(1))),
        );
    }

    #[test]
    fn an_account_added_after_a_restore_gets_an_identifier_distinct_from_every_restored_one() {
        let workspace = Workspace {
            accounts: vec![
                saved_account("session-0001", "A"),
                saved_account("session-0007", "B"),
            ],
            layout: Layout::Single,
        };
        let mut book = SessionBook::restore(workspace);

        let fresh = book.add("C", "https://example.test/c");

        let restored: Vec<SessionId> = book
            .sessions()
            .iter()
            .take(2)
            .map(|s| s.id().clone())
            .collect();
        assert!(!restored.contains(&fresh));
    }

    #[test]
    fn the_workspace_read_off_a_restored_book_reproduces_the_one_it_was_restored_from() {
        let workspace = Workspace {
            accounts: vec![
                Account {
                    liveness: SavedLiveness::Running,
                    is_kept_awake: true,
                    zoom: ZoomLevel::new(1.2).expect("1.2 is an accepted multiplier"),
                    ..in_slot(saved_account("session-0001", "A"), 0)
                },
                Account {
                    liveness: SavedLiveness::Parked,
                    browser_identity: Some("UA/2".to_owned()),
                    ..in_slot(saved_account("session-0002", "B"), 1)
                },
            ],
            layout: Layout::SideBySide,
        };

        let restored = SessionBook::restore(workspace.clone());

        assert_eq!(restored.workspace(), workspace);
    }

    #[test]
    fn unparking_a_queued_account_returns_starting() {
        let workspace = Workspace {
            accounts: vec![Account {
                liveness: SavedLiveness::Running,
                ..saved_account("session-0001", "Queued")
            }],
            layout: Layout::Single,
        };
        let mut book = SessionBook::restore(workspace);
        let id = book.sessions()[0].id().clone();

        let state = book.unpark(&id);

        assert_eq!(state, Liveness::Starting);
    }

    #[test]
    fn a_starting_account_is_written_to_the_workspace_as_running() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.set_keep_awake(&id, true);

        let saved = book.workspace();

        assert_eq!(saved.accounts[0].liveness, SavedLiveness::Running);
    }

    fn accepted(multiplier: f64) -> ZoomLevel {
        ZoomLevel::new(multiplier).expect("an accepted multiplier")
    }

    #[test]
    fn zoom_for_returns_the_baseline_for_a_layout_with_no_remembered_size() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");

        assert_eq!(
            session(&book, &id).zoom_for(Layout::Single),
            ZoomLevel::DEFAULT
        );
    }

    #[test]
    fn zoom_for_returns_the_remembered_size_for_the_one_layout_that_has_one() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.restore_zoom(&id, [(Layout::Grid, accepted(1.5))].into_iter().collect());

        let session = session(&book, &id);

        assert_eq!(
            (
                session.zoom_for(Layout::Grid),
                session.zoom_for(Layout::Single),
                session.zoom_for(Layout::SideBySide),
            ),
            (accepted(1.5), ZoomLevel::DEFAULT, ZoomLevel::DEFAULT),
        );
    }

    #[test]
    fn preset_zoom_still_answers_the_game_files_value_after_a_remembered_size_is_installed() {
        let mut book = SessionBook::new();
        let id = book.add_from_preset(
            "Alt",
            &Preset {
                zoom: accepted(0.8),
                ..a_preset()
            },
        );
        book.restore_zoom(&id, [(Layout::Single, accepted(2.0))].into_iter().collect());

        assert_eq!(session(&book, &id).preset_zoom(), accepted(0.8));
    }

    #[test]
    fn restore_zoom_installs_a_whole_remembered_map_onto_the_named_account() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");

        book.restore_zoom(
            &id,
            [
                (Layout::Single, accepted(0.5)),
                (Layout::Grid, accepted(2.0)),
            ]
            .into_iter()
            .collect(),
        );

        let session = session(&book, &id);
        assert_eq!(
            (
                session.zoom_for(Layout::Single),
                session.zoom_for(Layout::Grid),
            ),
            (accepted(0.5), accepted(2.0)),
        );
    }

    #[test]
    fn restore_zoom_changes_nothing_for_an_id_the_book_does_not_hold() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");

        book.restore_zoom(
            &SessionId::new("session-9999"),
            [(Layout::Grid, accepted(2.0))].into_iter().collect(),
        );

        assert_eq!(
            session(&book, &id).zoom_for(Layout::Grid),
            ZoomLevel::DEFAULT
        );
    }

    #[test]
    fn the_workspace_an_account_produces_still_carries_its_baseline_zoom() {
        let mut book = SessionBook::new();
        let id = book.add_from_preset(
            "Alt",
            &Preset {
                zoom: accepted(1.2),
                ..a_preset()
            },
        );
        book.restore_zoom(&id, [(Layout::Single, accepted(3.0))].into_iter().collect());

        assert_eq!(book.workspace().accounts[0].zoom, accepted(1.2));
    }

    #[test]
    fn zoom_in_returns_the_stepped_size_and_records_it_against_the_current_layout() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");

        let stepped = book.zoom_in(&id).expect("the account is in the book");

        assert_eq!(
            (stepped, session(&book, &id).zoom_for(Layout::Single)),
            (
                ZoomLevel::DEFAULT.stepped_in(),
                ZoomLevel::DEFAULT.stepped_in()
            ),
        );
    }

    #[test]
    fn zoom_out_returns_the_stepped_down_size_and_records_it_the_same_way() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");

        let stepped = book.zoom_out(&id).expect("the account is in the book");

        assert_eq!(
            (stepped, session(&book, &id).zoom_for(Layout::Single)),
            (
                ZoomLevel::DEFAULT.stepped_out(),
                ZoomLevel::DEFAULT.stepped_out(),
            ),
        );
    }

    #[test]
    fn a_step_records_nothing_for_the_two_layouts_that_are_not_current() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let id = book.add("One", "https://example.test/one");

        book.zoom_in(&id);

        let session = session(&book, &id);
        assert_eq!(
            (
                session.zoom_for(Layout::Single),
                session.zoom_for(Layout::SideBySide),
            ),
            (ZoomLevel::DEFAULT, ZoomLevel::DEFAULT),
        );
    }

    #[test]
    fn a_step_at_the_ranges_edge_returns_and_records_the_clamped_size() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.restore_zoom(
            &id,
            [(Layout::Single, accepted(ZoomLevel::MAX))]
                .into_iter()
                .collect(),
        );

        let stepped = book.zoom_in(&id).expect("the account is in the book");

        assert_eq!(
            (stepped, session(&book, &id).zoom_for(Layout::Single)),
            (accepted(ZoomLevel::MAX), accepted(ZoomLevel::MAX)),
        );
    }

    #[test]
    fn reset_zoom_drops_the_current_entry_keeps_the_others_and_returns_the_baseline() {
        let mut book = SessionBook::new();
        let id = book.add_from_preset(
            "Alt",
            &Preset {
                zoom: accepted(0.8),
                ..a_preset()
            },
        );
        book.restore_zoom(
            &id,
            [
                (Layout::Single, accepted(2.0)),
                (Layout::Grid, accepted(1.5)),
            ]
            .into_iter()
            .collect(),
        );

        let baseline = book.reset_zoom(&id).expect("the account is in the book");

        let session = session(&book, &id);
        assert_eq!(
            (
                baseline,
                session.zoom_for(Layout::Single),
                session.zoom_for(Layout::Grid),
            ),
            (accepted(0.8), accepted(0.8), accepted(1.5)),
        );
    }

    #[test]
    fn the_zoom_transitions_return_none_for_an_id_the_book_does_not_hold() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        let unknown = SessionId::new("session-9999");

        let outcomes = (
            book.zoom_in(&unknown),
            book.zoom_out(&unknown),
            book.reset_zoom(&unknown),
        );

        assert_eq!(
            (outcomes, session(&book, &id).zoom_for(Layout::Single),),
            ((None, None, None), ZoomLevel::DEFAULT),
        );
    }

    #[test]
    fn a_step_leaves_the_accounts_liveness_and_visibility_exactly_as_they_were() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        book.park(&id);
        let before = (liveness_of(&book, &id), visibility_of(&book, &id));

        book.zoom_in(&id);

        assert_eq!((liveness_of(&book, &id), visibility_of(&book, &id)), before,);
    }

    #[test]
    fn switching_layout_changes_what_zoom_for_answers_without_changing_a_stored_size() {
        let mut book = SessionBook::new();
        let id = book.add("One", "https://example.test/one");
        let chosen = book.zoom_in(&id).expect("the account is in the book");

        book.set_layout(Layout::Grid);
        let while_grid = session(&book, &id).zoom_for(Layout::Grid);
        book.set_layout(Layout::Single);

        assert_eq!(
            (while_grid, session(&book, &id).zoom_for(Layout::Single)),
            (ZoomLevel::DEFAULT, chosen),
        );
    }

    #[test]
    fn focused_session_returns_the_account_in_the_focused_slot() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        book.add("One", "https://example.test/one");
        let two = book.add("Two", "https://example.test/two");
        book.set_focused(SlotId::new(1));

        assert_eq!(book.focused_session().map(Session::id), Some(&two));
    }

    #[test]
    fn focused_session_is_none_when_the_focused_slot_holds_nothing() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        book.add("One", "https://example.test/one");
        book.set_focused(SlotId::new(1));

        assert!(book.focused_session().is_none());
    }

    #[test]
    fn restoring_an_empty_workspace_leaves_an_empty_book() {
        let book = SessionBook::restore(Workspace {
            accounts: Vec::new(),
            layout: Layout::default(),
        });

        assert_eq!(
            (
                book.sessions().len(),
                book.start_order().len(),
                book.layout()
            ),
            (0, 0, Layout::default()),
        );
    }
}
