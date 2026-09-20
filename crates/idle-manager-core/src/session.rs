//! What a game account is: the identity the program mints for it, the two
//! things the user typed, and whether it currently occupies a slot.

use std::collections::HashMap;

use crate::layout::{
    Layout, MoveOutcome, Placement, SlotId, arrange, bring_into_focus, move_into_slot,
};
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

/// The fixed identifier of the built-in Ungrouped workspace, compared by
/// [`WorkspaceId::is_ungrouped`] rather than by a display name, since a
/// display name can be renamed and this one never is.
const UNGROUPED_ID: &str = "ungrouped";

/// The identifier the program mints for a workspace.
///
/// A newtype so it can never be passed where a [`SessionId`] belongs — both
/// are strings to the compiler until made otherwise (code standards rule 2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    /// Wraps an existing identifier string. [`crate::WorkspaceBook`] is the
    /// only place a fresh one is minted; this is for adapters and tests that
    /// already hold an id.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        WorkspaceId(value.into())
    }

    /// The fixed id of the built-in workspace every account starts in. Never
    /// minted, never renamed, never removed.
    #[must_use]
    pub fn ungrouped() -> Self {
        WorkspaceId(UNGROUPED_ID.to_owned())
    }

    /// Whether this is the built-in Ungrouped workspace's id.
    #[must_use]
    pub fn is_ungrouped(&self) -> bool {
        self.0 == UNGROUPED_ID
    }

    /// The identifier as a string slice, for use as a path component or a
    /// file key.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// The one rule an account name must satisfy: the surrounding whitespace
/// trimmed away, or `None` when nothing is left.
///
/// Shared by the add-game dialog and [`SessionBook::rename`] (`FR.13.3`), so
/// the two can never enforce a different rule.
#[must_use]
pub fn account_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

/// The one rule a workspace name must satisfy: the surrounding whitespace
/// trimmed away, or `None` when nothing is left.
///
/// Shared by every caller that names or renames a workspace
/// (`WorkspaceBook::create_workspace`, `WorkspaceBook::rename_workspace`), the
/// same shape as [`account_name`].
#[must_use]
pub fn workspace_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
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
    /// Whether this account's pages get a WebGL context, copied from its preset
    /// at creation (`FR.19.7`). A typed-address account gets `true` — the
    /// engine's own default — since there is no preset to say otherwise.
    is_webgl_enabled: bool,
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

    /// Whether this account's pages should be given a WebGL context, copied
    /// from its preset at creation (`FR.19.7`).
    #[must_use]
    pub fn is_webgl_enabled(&self) -> bool {
        self.is_webgl_enabled
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
    /// whole resolution rule, in one place (`FR.12.1`). In [`Layout::Mobile`]
    /// it is always [`ZoomLevel::DEFAULT`]: the slot's pixel size *is* the
    /// phone's viewport, and any other size would change the page's viewport
    /// out from under the game (Remote Access `FR.3.2`).
    #[must_use]
    pub fn zoom_for(&self, layout: Layout) -> ZoomLevel {
        if layout == Layout::Mobile {
            return ZoomLevel::DEFAULT;
        }
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

/// Where one account sat in an [`Arrangement`]: its visibility then, and the
/// slot it would return to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Seat {
    visibility: Visibility,
    remembered: Option<SlotId>,
}

/// One workspace's seating as a value that can be put back later: its layout,
/// its focused slot, and where every account sat and would return to. Taken
/// by [`SessionBook::arrangement`] before mobile mode rearranges the book and
/// handed back to [`SessionBook::restore_arrangement`] when the mode ends, so
/// leaving puts back exactly what entering found (Remote Access `FR.3.5`).
///
/// Carries the visibility as well as the remembered slot because the
/// placement pass alone cannot reproduce a seating: an account displaced
/// off-grid by a focus swap still remembers the slot it lost, and a slot
/// freed by a removal stays empty rather than being refilled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Arrangement {
    layout: Layout,
    focused: SlotId,
    seats: HashMap<SessionId, Seat>,
}

/// The application's sessions in the order they sit in, together with the
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
        }
    }

    /// Rebuilds a book from a saved [`Workspace`].
    ///
    /// The accounts come back in the order the workspace held them. One saved
    /// as running becomes [`Liveness::Queued`] — nothing is running yet — and
    /// one saved as parked stays [`Liveness::Parked`], costing nothing. The
    /// saved layout and focused place become active and each account's
    /// remembered slot is seeded, so visibility is normalised through the same
    /// placement a layout switch uses: an account whose saved slot the layout
    /// cannot show lands off-grid and returns to that slot when a layout with
    /// it is chosen (`FR.3.2`). The workspace's `id`, `name` and `is_expanded`
    /// are not this book's to keep — [`crate::WorkspaceBook`] carries those
    /// alongside the book it restores here — and minting is likewise
    /// [`crate::WorkspaceBook`]'s job now, from one counter shared by every
    /// workspace.
    #[must_use]
    pub fn restore(workspace: Workspace) -> Self {
        let Workspace {
            accounts,
            layout,
            focused,
            ..
        } = workspace;

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
                // The workspace does not persist WebGL yet; a restored account
                // returns on the engine's default and a preset that disables it
                // re-applies on the next add. TODO(05): persist it on Account.
                is_webgl_enabled: true,
                preset_zoom: account.zoom,
                remembered_zoom: RememberedZoom::new(),
            })
            .collect();

        Self {
            sessions,
            layout,
            focused: focused_or_first(focused, layout),
            remembered,
        }
    }

    /// The sessions, in the order they sit in.
    #[must_use]
    pub fn sessions(&self) -> &[Session] {
        &self.sessions
    }

    /// The session named `id`, or `None` if this book does not hold it. The
    /// one place a caller looks a session up by id, replacing the
    /// `sessions().iter().find(...)` every reader used to repeat.
    #[must_use]
    pub fn session(&self, id: &SessionId) -> Option<&Session> {
        self.sessions.iter().find(|session| &session.id == id)
    }

    /// The whole arrangement as one value, ready to be saved as workspace
    /// `id` named `name`, expanded or not per `is_expanded` — the three facts
    /// this book does not itself carry, supplied by
    /// [`crate::WorkspaceBook`], which owns them.
    ///
    /// Accounts in the order they sit in, each carrying its name, address,
    /// zoom, identity, keep-awake flag, and where it sits, plus the active
    /// layout and focused place. A [`Liveness::Starting`] or
    /// [`Liveness::Queued`] account is reported as [`SavedLiveness::Running`]:
    /// those describe a moment, not a wish.
    #[must_use]
    pub fn workspace(&self, id: WorkspaceId, name: String, is_expanded: bool) -> Workspace {
        self.workspace_as(&self.arrangement(), id, name, is_expanded)
    }

    /// [`SessionBook::workspace`] with `arrangement` in place of the live
    /// seating — what mobile mode saves, so the file never records
    /// [`Layout::Mobile`] and a relaunch opens in the arrangement from before
    /// (Remote Access `FR.3.4`). Every field that is not a seat — name,
    /// address, liveness, keep-awake, identity, zoom — is still the live one,
    /// so a park made while the mode is on is written. An account
    /// `arrangement` does not know is seated as
    /// [`SessionBook::restore_arrangement`] would seat it.
    #[must_use]
    pub(crate) fn workspace_as(
        &self,
        arrangement: &Arrangement,
        id: WorkspaceId,
        name: String,
        is_expanded: bool,
    ) -> Workspace {
        let seats = self.seats_under(arrangement);
        let accounts = self
            .sessions
            .iter()
            .map(|session| {
                let seat = seats.get(&session.id).copied().unwrap_or(Seat {
                    visibility: Visibility::OffGrid,
                    remembered: None,
                });
                Account {
                    id: session.id.clone(),
                    display_name: session.display_name.clone(),
                    start_address: session.start_address.clone(),
                    liveness: match session.liveness {
                        Liveness::Parked => SavedLiveness::Parked,
                        Liveness::Live | Liveness::Starting | Liveness::Queued => {
                            SavedLiveness::Running
                        }
                    },
                    visibility: seat.visibility,
                    remembered_slot: seat.remembered,
                    is_kept_awake: session.is_kept_awake,
                    browser_identity: session.browser_identity.clone(),
                    zoom: session.preset_zoom,
                }
            })
            .collect();

        Workspace {
            id,
            name,
            focused: focused_or_first(arrangement.focused, arrangement.layout),
            is_expanded,
            accounts,
            layout: arrangement.layout,
        }
    }

    /// This book's seating right now, as a value [`SessionBook::restore_arrangement`]
    /// puts back later — taken before mobile mode rearranges the book
    /// (Remote Access `FR.3.5`).
    #[must_use]
    pub(crate) fn arrangement(&self) -> Arrangement {
        Arrangement {
            layout: self.layout,
            focused: self.focused,
            seats: self.current_seats(),
        }
    }

    /// Puts `arrangement` back: its layout, its focused slot, and every
    /// account it knows in the seat it had. An account removed since is
    /// skipped; an account added since keeps the slot it holds when the
    /// layout has it and nobody returning claims it, and otherwise takes the
    /// lowest free slot or goes off-grid — the placement pass's own rule
    /// (Remote Access `FR.3.5`). Liveness, keep-awake and zoom are untouched.
    pub(crate) fn restore_arrangement(&mut self, arrangement: &Arrangement) {
        let seats = self.seats_under(arrangement);

        self.layout = arrangement.layout;
        self.focused = focused_or_first(arrangement.focused, arrangement.layout);
        for session in &mut self.sessions {
            let Some(seat) = seats.get(&session.id) else {
                continue;
            };
            session.visibility = seat.visibility;
            match seat.remembered {
                Some(slot) => {
                    self.remembered.insert(session.id.clone(), slot);
                }
                None => {
                    self.remembered.remove(&session.id);
                }
            }
        }
    }

    fn current_seats(&self) -> HashMap<SessionId, Seat> {
        self.sessions
            .iter()
            .map(|session| {
                (
                    session.id.clone(),
                    Seat {
                        visibility: session.visibility,
                        remembered: self.remembered.get(&session.id).copied(),
                    },
                )
            })
            .collect()
    }

    /// Where every session of this book sits under `arrangement`: a session
    /// the arrangement knows sits exactly where it did; one it does not —
    /// added since it was taken — keeps its own slot if `arrangement`'s
    /// layout has it and no returning session holds it, else the lowest free
    /// slot, else off-grid. In session order, so two newcomers never land in
    /// one slot.
    fn seats_under(&self, arrangement: &Arrangement) -> HashMap<SessionId, Seat> {
        let mut taken: Vec<SlotId> = self
            .sessions
            .iter()
            .filter_map(|session| arrangement.seats.get(&session.id))
            .filter_map(|seat| match seat.visibility {
                Visibility::InSlot(slot) => Some(slot),
                Visibility::OffGrid => None,
            })
            .collect();

        let mut seats = HashMap::with_capacity(self.sessions.len());
        for session in &self.sessions {
            if let Some(seat) = arrangement.seats.get(&session.id) {
                seats.insert(session.id.clone(), *seat);
                continue;
            }

            let kept = match session.visibility {
                Visibility::InSlot(slot)
                    if arrangement.layout.contains(slot) && !taken.contains(&slot) =>
                {
                    Some(slot)
                }
                Visibility::InSlot(_) | Visibility::OffGrid => arrangement
                    .layout
                    .slots()
                    .find(|slot| !taken.contains(slot)),
            };
            let seat = match kept {
                Some(slot) => {
                    taken.push(slot);
                    Seat {
                        visibility: Visibility::InSlot(slot),
                        remembered: Some(slot),
                    }
                }
                None => Seat {
                    visibility: Visibility::OffGrid,
                    remembered: self.remembered.get(&session.id).copied(),
                },
            };
            seats.insert(session.id.clone(), seat);
        }

        seats
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

    /// Adds a new account created from a typed address under `id`, and places
    /// it.
    ///
    /// `id` is minted by the caller — [`crate::WorkspaceBook`], from the one
    /// counter shared by every workspace — never by this book (code standards
    /// rule 2). The session takes the lowest-numbered free slot of the current
    /// layout. If every slot is occupied it takes the focused slot, and that
    /// slot's previous occupant becomes [`Visibility::OffGrid`]. It carries no
    /// browser identity and the default zoom — everything a preset would have
    /// supplied falls back here.
    pub fn add(&mut self, id: &SessionId, display_name: &str, start_address: &str) {
        let visibility = self.place(id);

        self.sessions.push(Session {
            id: id.clone(),
            display_name: display_name.to_owned(),
            start_address: start_address.to_owned(),
            visibility,
            liveness: Liveness::Live,
            is_kept_awake: false,
            browser_identity: None,
            is_webgl_enabled: true,
            preset_zoom: ZoomLevel::DEFAULT,
            remembered_zoom: RememberedZoom::new(),
        });
    }

    /// Adds a new account under `id`, playing `preset`'s game under the name
    /// `account_name`, places it exactly as [`SessionBook::add`] does, and
    /// copies the preset's start address, zoom and browser identity onto it.
    ///
    /// `id` is minted by the caller, as [`SessionBook::add`] documents. The
    /// preset's keep-awake default is applied through
    /// [`SessionBook::set_keep_awake`] — the one path that sets that flag —
    /// rather than written directly. That call moves a live account to
    /// [`Liveness::Starting`], so this follows it with
    /// [`SessionBook::mark_started`]: a brand-new account has no page to reload,
    /// its first view is built carrying the setting already, and it ends
    /// [`Liveness::Live`] like any other. Both are existing transitions; no
    /// field is written behind their backs.
    pub fn add_from_preset(&mut self, id: &SessionId, account_name: &str, preset: &Preset) {
        let visibility = self.place(id);

        self.sessions.push(Session {
            id: id.clone(),
            display_name: account_name.to_owned(),
            start_address: preset.start_address.clone(),
            visibility,
            liveness: Liveness::Live,
            is_kept_awake: false,
            browser_identity: preset.browser_identity.clone(),
            is_webgl_enabled: preset.webgl_enabled,
            preset_zoom: preset.zoom,
            remembered_zoom: RememberedZoom::new(),
        });

        self.set_keep_awake(id, preset.keep_awake_default);
        self.mark_started(id);
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

    /// Renames `account` to `name`, applying [`account_name`]'s rule, and
    /// reports whether anything was stored.
    ///
    /// `false` for an unknown id or a name that is empty once trimmed, and the
    /// book is left exactly as it was. Otherwise the trimmed name replaces
    /// `account`'s display name and nothing else: its id, liveness,
    /// visibility, keep-awake flag, the book's focused slot and the order of
    /// its sessions are untouched (`FR.13.2`, `FR.13.5`). Two accounts may
    /// share a name, since [`SessionBook::add`] already allows that.
    pub fn rename(&mut self, account: &SessionId, name: &str) -> bool {
        let Some(trimmed) = account_name(name) else {
            return false;
        };
        let Some(session) = self.sessions.iter_mut().find(|s| &s.id == account) else {
            return false;
        };

        session.display_name = trimmed;
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
    /// size. `None` for an id the book does not hold, and `None` with nothing
    /// recorded while the layout is [`Layout::Mobile`], where the size is
    /// locked (Remote Access `FR.3.2`). Liveness and visibility are untouched
    /// — a zoom is not a reload (`FR.11.5`).
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
        if layout == Layout::Mobile {
            return None;
        }
        let session = self.sessions.iter_mut().find(|s| &s.id == account)?;
        let stepped = step(session.zoom_for(layout));
        session.remembered_zoom.set(layout, stepped);
        Some(stepped)
    }

    /// Drops `account`'s remembered size for the book's current layout and
    /// returns the baseline the game file supplied. Entries for the other
    /// arrangements are left in place. `None` for an id the book does not hold
    /// (`FR.11.2`), and `None` with nothing changed in [`Layout::Mobile`],
    /// exactly as [`SessionBook::zoom_in`].
    pub fn reset_zoom(&mut self, account: &SessionId) -> Option<ZoomLevel> {
        let layout = self.layout;
        if layout == Layout::Mobile {
            return None;
        }
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

    /// Removes `id` from this book, together with its remembered slot, and
    /// returns it. Every other session's visibility is left exactly as it
    /// was, so the freed place stays empty (`FR.17.8`, `FR.21.5`). `None` for
    /// an id this book does not hold.
    pub fn take(&mut self, id: &SessionId) -> Option<Session> {
        let index = self.sessions.iter().position(|s| &s.id == id)?;
        self.remembered.remove(id);
        Some(self.sessions.remove(index))
    }

    /// Appends an arriving `session` to the end of this book's list, seated in
    /// the lowest free slot of this book's current layout if one is free and
    /// off-grid otherwise. Never displaces an already-seated session
    /// (`FR.17.3`). Every other field — liveness, keep-awake, remembered
    /// zoom — is carried over exactly as `session` held it.
    pub fn adopt(&mut self, mut session: Session) {
        let occupied = self.occupied_slots();
        let free = self.layout.slots().find(|slot| !occupied.contains(slot));

        session.visibility = match free {
            Some(slot) => {
                self.remembered.insert(session.id.clone(), slot);
                Visibility::InSlot(slot)
            }
            None => Visibility::OffGrid,
        };

        self.sessions.push(session);
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

    /// Move `account` to `target` and report what happened.
    ///
    /// [`MoveOutcome::Unchanged`] when `target` is `account`'s own slot, is not
    /// a slot the current layout has, or `account` is off-grid or unknown —
    /// the book is left exactly as it was, not even its session order. On a
    /// real move both accounts' new slots are remembered, so a later layout
    /// switch returns them there (`FR.3.2`). Focus is a slot index, so keeping
    /// it on the account that had it means: if the focused slot was
    /// `account`'s, it becomes `target`; if it was `target` and the result is
    /// [`MoveOutcome::Swapped`], it becomes `account`'s old slot; otherwise it
    /// is unchanged (`FR.14.3`). `sessions` is then reordered to read like the
    /// window (`FR.14.7`). No liveness, keep-awake flag or remembered zoom is
    /// read or written (`FR.14.8`).
    pub fn move_to_slot(&mut self, account: &SessionId, target: SlotId) -> MoveOutcome {
        let current: HashMap<SessionId, Visibility> = self
            .sessions
            .iter()
            .map(|s| (s.id.clone(), s.visibility))
            .collect();

        let moved = move_into_slot(&current, self.layout, account, target);
        if *moved.outcome() == MoveOutcome::Unchanged {
            return MoveOutcome::Unchanged;
        }

        for s in &mut self.sessions {
            let Some(visibility) = moved.visibility().get(&s.id).copied() else {
                continue;
            };
            s.visibility = visibility;
            if let Visibility::InSlot(slot) = visibility {
                self.remembered.insert(s.id.clone(), slot);
            }
        }

        let Some(Visibility::InSlot(source)) = current.get(account).copied() else {
            unreachable!("a real move only happens when `account` holds a slot");
        };

        if self.focused == source {
            self.focused = target;
        } else if self.focused == target && matches!(moved.outcome(), MoveOutcome::Swapped { .. }) {
            self.focused = source;
        }

        self.reorder_by_placement();

        moved.outcome().clone()
    }

    /// Reorders `sessions` to read like the window: [`Visibility::InSlot`]
    /// accounts first, by slot index, then off-grid accounts in their
    /// existing relative order. A stable sort, so an unchanged move, a
    /// rename, a layout switch or a focus change never calls this and never
    /// disturbs the order (`FR.14.7`).
    fn reorder_by_placement(&mut self) {
        self.sessions
            .sort_by_key(|session| match session.visibility {
                Visibility::InSlot(slot) => (0, slot.index()),
                Visibility::OffGrid => (1, 0),
            });
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

/// `focused` if `layout` actually has that slot, [`SlotId::FIRST`] otherwise —
/// a hand-edited file naming a focused place outside the saved layout falls
/// back rather than leaving the book focused nowhere.
fn focused_or_first(focused: SlotId, layout: Layout) -> SlotId {
    if layout.contains(focused) {
        focused
    } else {
        SlotId::FIRST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mints a sequential id from how many sessions `book` already holds and
    /// adds a typed-address account under it — minting moved to
    /// [`crate::WorkspaceBook`], so these tests mint their own the same simple
    /// way, valid because none of them ever removes a session before adding
    /// another.
    fn add(book: &mut SessionBook, display_name: &str, start_address: &str) -> SessionId {
        let id = SessionId::new(format!("session-{:04}", book.sessions().len() + 1));
        book.add(&id, display_name, start_address);
        id
    }

    /// [`add`], for a preset-based account.
    fn add_from_preset(book: &mut SessionBook, account_name: &str, preset: &Preset) -> SessionId {
        let id = SessionId::new(format!("session-{:04}", book.sessions().len() + 1));
        book.add_from_preset(&id, account_name, preset);
        id
    }

    #[test]
    fn a_minted_identifier_is_distinct_from_every_existing_one() {
        let mut book = SessionBook::new();

        let first = add(&mut book, "One", "https://example.test/one");
        let second = add(&mut book, "Two", "https://example.test/two");

        assert_ne!(first, second);
    }

    #[test]
    fn a_session_added_with_a_slot_free_takes_the_lowest_numbered_one() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);

        add(&mut book, "One", "https://example.test/one");
        let second = add(&mut book, "Two", "https://example.test/two");

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
        let first = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));

        add(&mut book, "Three", "https://example.test/three");

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
        add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));

        let third = add(&mut book, "Three", "https://example.test/three");

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
        let first = add(&mut book, "One", "https://example.test/one");
        let second = add(&mut book, "Two", "https://example.test/two");
        add(&mut book, "Three", "https://example.test/three");
        add(&mut book, "Four", "https://example.test/four");

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
        add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        let third = add(&mut book, "Three", "https://example.test/three");
        let fourth = add(&mut book, "Four", "https://example.test/four");

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
        add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        let third = add(&mut book, "Three", "https://example.test/three");
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
        add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));
        let third = add(&mut book, "Three", "https://example.test/three");

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
        add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");

        book.focus_session(&two);

        assert_eq!(book.focused(), SlotId::new(1));
    }

    #[test]
    fn focusing_an_off_grid_session_swaps_it_into_the_focused_slot_in_the_book() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let first = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));
        let third = add(&mut book, "Three", "https://example.test/three");

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
        add(&mut book, "Main account", "https://example.test/game");
        add(&mut book, "Alt account", "https://example.test/game?alt");

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

        let id = add(&mut book, "One", "https://example.test/one");

        assert_eq!(liveness_of(&book, &id), Liveness::Live);
    }

    #[test]
    fn parking_a_live_session_returns_parked() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");

        let state = book.park(&id);

        assert_eq!(state, Liveness::Parked);
    }

    #[test]
    fn parking_a_session_leaves_its_visibility_unchanged() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        let visibility = visibility_of(&book, &id);

        book.park(&id);

        assert_eq!(visibility_of(&book, &id), visibility);
    }

    #[test]
    fn unparking_a_parked_session_returns_starting() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.park(&id);

        let state = book.unpark(&id);

        assert_eq!(state, Liveness::Starting);
    }

    #[test]
    fn unparking_a_session_leaves_its_visibility_unchanged() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.park(&id);
        let visibility = visibility_of(&book, &id);

        book.unpark(&id);

        assert_eq!(visibility_of(&book, &id), visibility);
    }

    #[test]
    fn ending_the_starting_interval_returns_live() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.park(&id);
        book.unpark(&id);

        let state = book.mark_started(&id);

        assert_eq!(state, Liveness::Live);
    }

    #[test]
    fn parking_an_already_parked_session_returns_parked_and_changes_nothing_else() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
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
        let id = add(&mut book, "One", "https://example.test/one");
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
        add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");

        assert_eq!(book.live_session_count(), 2);
    }

    #[test]
    fn a_parked_account_is_not_counted_among_the_live_sessions() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");

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

        let id = add(&mut book, "One", "https://example.test/one");

        assert!(!is_kept_awake(&book, &id));
    }

    #[test]
    fn turning_keep_awake_on_for_an_account_that_had_it_off_returns_true() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");

        let changed = book.set_keep_awake(&id, true);

        assert!(changed);
    }

    #[test]
    fn turning_keep_awake_on_for_an_account_that_had_it_off_leaves_the_flag_on() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");

        book.set_keep_awake(&id, true);

        assert!(is_kept_awake(&book, &id));
    }

    #[test]
    fn setting_keep_awake_to_its_current_value_reports_no_change() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");

        let changed = book.set_keep_awake(&id, false);

        assert!(!changed);
    }

    #[test]
    fn setting_keep_awake_to_its_current_value_leaves_liveness_unchanged() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.park(&id);
        let liveness = liveness_of(&book, &id);

        book.set_keep_awake(&id, false);

        assert_eq!(liveness_of(&book, &id), liveness);
    }

    #[test]
    fn turning_keep_awake_on_for_a_live_account_leaves_it_starting() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");

        book.set_keep_awake(&id, true);

        assert_eq!(liveness_of(&book, &id), Liveness::Starting);
    }

    #[test]
    fn turning_keep_awake_on_for_a_parked_account_leaves_it_parked() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.park(&id);

        book.set_keep_awake(&id, true);

        assert_eq!(liveness_of(&book, &id), Liveness::Parked);
    }

    #[test]
    fn keep_awake_survives_a_layout_change_that_moves_the_account_between_slots() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let id = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        add(&mut book, "Three", "https://example.test/three");
        add(&mut book, "Four", "https://example.test/four");
        book.set_keep_awake(&id, true);

        book.set_layout(Layout::SideBySide);
        book.set_layout(Layout::Grid);

        assert!(is_kept_awake(&book, &id));
    }

    #[test]
    fn keep_awake_survives_being_pushed_off_grid_and_brought_back_into_focus() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let first = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));
        add(&mut book, "Three", "https://example.test/three");
        book.set_keep_awake(&first, true);

        book.focus_session(&first);

        assert!(is_kept_awake(&book, &first));
    }

    #[test]
    fn keep_awake_survives_being_parked() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.set_keep_awake(&id, true);

        book.park(&id);

        assert!(is_kept_awake(&book, &id));
    }

    #[test]
    fn keep_awake_survives_being_unparked() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
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
        let id = add(&mut book, "One", "https://example.test/one");
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
            webgl_enabled: true,
        }
    }

    #[test]
    fn an_account_from_a_preset_carries_the_typed_name_not_the_games_display_name() {
        let mut book = SessionBook::new();

        let id = add_from_preset(&mut book, "Alt", &a_preset());

        assert_eq!(session(&book, &id).display_name(), "Alt");
    }

    #[test]
    fn an_account_from_a_preset_carries_the_presets_address_zoom_and_identity() {
        let mut book = SessionBook::new();
        let preset = a_preset();

        let id = add_from_preset(&mut book, "Alt", &preset);

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

        let id = add_from_preset(&mut book, "Alt", &preset);

        assert_eq!(session(&book, &id).browser_identity(), None);
    }

    #[test]
    fn an_account_from_a_preset_with_webgl_disabled_carries_that_forward() {
        let mut book = SessionBook::new();
        let preset = Preset {
            webgl_enabled: false,
            ..a_preset()
        };

        let id = add_from_preset(&mut book, "Alt", &preset);

        assert!(!session(&book, &id).is_webgl_enabled());
    }

    #[test]
    fn a_typed_address_account_gets_a_webgl_context() {
        let mut book = SessionBook::new();

        let id = add(&mut book, "Typed", "https://example.test/typed");

        assert!(session(&book, &id).is_webgl_enabled());
    }

    #[test]
    fn an_account_from_a_preset_whose_keep_awake_default_is_on_starts_kept_awake() {
        let mut book = SessionBook::new();
        let preset = Preset {
            keep_awake_default: true,
            ..a_preset()
        };

        let id = add_from_preset(&mut book, "Alt", &preset);

        assert!(session(&book, &id).is_kept_awake());
    }

    #[test]
    fn an_account_from_a_preset_with_keep_awake_off_is_live() {
        let mut book = SessionBook::new();
        let preset = Preset {
            keep_awake_default: false,
            ..a_preset()
        };

        let id = add_from_preset(&mut book, "Alt", &preset);

        assert_eq!(session(&book, &id).liveness(), Liveness::Live);
    }

    #[test]
    fn an_account_from_a_preset_with_keep_awake_on_is_live() {
        let mut book = SessionBook::new();
        let preset = Preset {
            keep_awake_default: true,
            ..a_preset()
        };

        let id = add_from_preset(&mut book, "Alt", &preset);

        assert_eq!(session(&book, &id).liveness(), Liveness::Live);
    }

    #[test]
    fn an_account_from_a_preset_takes_the_lowest_free_slot_like_a_typed_one() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        add(&mut book, "One", "https://example.test/one");

        let id = add_from_preset(&mut book, "Alt", &a_preset());

        assert_eq!(
            session(&book, &id).visibility(),
            Visibility::InSlot(SlotId::new(1))
        );
    }

    #[test]
    fn an_account_from_a_preset_into_a_full_grid_displaces_the_focused_slot() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let first = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));

        add_from_preset(&mut book, "Alt", &a_preset());

        assert_eq!(session(&book, &first).visibility(), Visibility::OffGrid);
    }

    #[test]
    fn an_account_from_a_typed_address_carries_no_identity_and_the_default_zoom() {
        let mut book = SessionBook::new();

        let id = add(&mut book, "One", "https://example.test/one");

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
            ..Workspace::default()
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
            ..Workspace::default()
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
            ..Workspace::default()
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
            ..Workspace::default()
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
            ..Workspace::default()
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
            ..Workspace::default()
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
            ..Workspace::default()
        };
        let mut book = SessionBook::restore(workspace);

        let fresh = add(&mut book, "C", "https://example.test/c");

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
            ..Workspace::default()
        };

        let restored = SessionBook::restore(workspace.clone());

        assert_eq!(
            restored.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true),
            workspace
        );
    }

    #[test]
    fn unparking_a_queued_account_returns_starting() {
        let workspace = Workspace {
            accounts: vec![Account {
                liveness: SavedLiveness::Running,
                ..saved_account("session-0001", "Queued")
            }],
            layout: Layout::Single,
            ..Workspace::default()
        };
        let mut book = SessionBook::restore(workspace);
        let id = book.sessions()[0].id().clone();

        let state = book.unpark(&id);

        assert_eq!(state, Liveness::Starting);
    }

    #[test]
    fn a_starting_account_is_written_to_the_workspace_as_running() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.set_keep_awake(&id, true);

        let saved = book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true);

        assert_eq!(saved.accounts[0].liveness, SavedLiveness::Running);
    }

    fn accepted(multiplier: f64) -> ZoomLevel {
        ZoomLevel::new(multiplier).expect("an accepted multiplier")
    }

    #[test]
    fn zoom_for_returns_the_baseline_for_a_layout_with_no_remembered_size() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");

        assert_eq!(
            session(&book, &id).zoom_for(Layout::Single),
            ZoomLevel::DEFAULT
        );
    }

    #[test]
    fn zoom_for_returns_the_remembered_size_for_the_one_layout_that_has_one() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
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
    fn zoom_for_mobile_is_the_default_size_despite_a_stored_single_override() {
        let mut book = SessionBook::new();
        let id = add_from_preset(
            &mut book,
            "Alt",
            &Preset {
                zoom: accepted(0.8),
                ..a_preset()
            },
        );
        book.restore_zoom(&id, [(Layout::Single, accepted(2.0))].into_iter().collect());

        assert_eq!(
            session(&book, &id).zoom_for(Layout::Mobile),
            ZoomLevel::DEFAULT
        );
    }

    #[test]
    fn zoom_in_in_mobile_returns_none_and_writes_no_mobile_key() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.set_layout(Layout::Mobile);

        let stepped = book.zoom_in(&id);

        assert_eq!(
            (stepped, session(&book, &id).remembered_zoom().is_empty()),
            (None, true),
        );
    }

    #[test]
    fn zoom_out_and_reset_in_mobile_return_none_and_leave_the_other_arrangements_alone() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.restore_zoom(&id, [(Layout::Grid, accepted(1.5))].into_iter().collect());
        book.set_layout(Layout::Mobile);

        let outcomes = (book.zoom_out(&id), book.reset_zoom(&id));

        assert_eq!(
            (
                outcomes,
                session(&book, &id).remembered_zoom().get(Layout::Grid)
            ),
            ((None, None), Some(accepted(1.5))),
        );
    }

    #[test]
    fn preset_zoom_still_answers_the_game_files_value_after_a_remembered_size_is_installed() {
        let mut book = SessionBook::new();
        let id = add_from_preset(
            &mut book,
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
        let id = add(&mut book, "One", "https://example.test/one");

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
        let id = add(&mut book, "One", "https://example.test/one");

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
        let id = add_from_preset(
            &mut book,
            "Alt",
            &Preset {
                zoom: accepted(1.2),
                ..a_preset()
            },
        );
        book.restore_zoom(&id, [(Layout::Single, accepted(3.0))].into_iter().collect());

        assert_eq!(
            book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true)
                .accounts[0]
                .zoom,
            accepted(1.2)
        );
    }

    #[test]
    fn zoom_in_returns_the_stepped_size_and_records_it_against_the_current_layout() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");

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
        let id = add(&mut book, "One", "https://example.test/one");

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
        let id = add(&mut book, "One", "https://example.test/one");

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
        let id = add(&mut book, "One", "https://example.test/one");
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
        let id = add_from_preset(
            &mut book,
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
        let id = add(&mut book, "One", "https://example.test/one");
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
        let id = add(&mut book, "One", "https://example.test/one");
        book.park(&id);
        let before = (liveness_of(&book, &id), visibility_of(&book, &id));

        book.zoom_in(&id);

        assert_eq!((liveness_of(&book, &id), visibility_of(&book, &id)), before,);
    }

    #[test]
    fn switching_layout_changes_what_zoom_for_answers_without_changing_a_stored_size() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
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
        add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(1));

        assert_eq!(book.focused_session().map(Session::id), Some(&two));
    }

    #[test]
    fn focused_session_is_none_when_the_focused_slot_holds_nothing() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        add(&mut book, "One", "https://example.test/one");
        book.set_focused(SlotId::new(1));

        assert!(book.focused_session().is_none());
    }

    #[test]
    fn restoring_an_empty_workspace_leaves_an_empty_book() {
        let book = SessionBook::restore(Workspace {
            accounts: Vec::new(),
            layout: Layout::default(),
            ..Workspace::default()
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

    #[test]
    fn account_name_trims_surrounding_whitespace() {
        assert_eq!(
            account_name("  Main account  "),
            Some("Main account".to_owned())
        );
    }

    #[test]
    fn account_name_is_none_for_an_empty_or_whitespace_only_string() {
        assert_eq!((account_name(""), account_name("   ")), (None, None));
    }

    #[test]
    fn renaming_with_a_padded_name_stores_the_trimmed_name_and_returns_true() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");

        let stored = book.rename(&id, "  New name  ");

        assert_eq!(
            (stored, session(&book, &id).display_name()),
            (true, "New name"),
        );
    }

    #[test]
    fn renaming_changes_only_the_display_name() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let first = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(1));
        let order = |book: &SessionBook| -> Vec<SessionId> {
            book.sessions().iter().map(|s| s.id().clone()).collect()
        };
        let before = (
            liveness_of(&book, &first),
            visibility_of(&book, &first),
            is_kept_awake(&book, &first),
            book.focused(),
            order(&book),
        );

        book.rename(&first, "  New name  ");

        let after = (
            liveness_of(&book, &first),
            visibility_of(&book, &first),
            is_kept_awake(&book, &first),
            book.focused(),
            order(&book),
        );
        assert_eq!(after, before);
    }

    #[test]
    fn renaming_with_an_empty_or_whitespace_only_name_returns_false() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");

        let stored = book.rename(&id, "   ");

        assert!(!stored);
    }

    #[test]
    fn renaming_with_an_empty_name_leaves_the_book_unchanged() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        let before = (
            book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true),
            book.focused(),
        );

        book.rename(&id, "   ");

        assert_eq!(
            (
                book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true),
                book.focused()
            ),
            before
        );
    }

    #[test]
    fn renaming_an_unknown_id_returns_false() {
        let mut book = SessionBook::new();
        add(&mut book, "One", "https://example.test/one");
        let unknown = SessionId::new("session-9999");

        let stored = book.rename(&unknown, "New name");

        assert!(!stored);
    }

    #[test]
    fn renaming_an_unknown_id_leaves_the_book_unchanged() {
        let mut book = SessionBook::new();
        add(&mut book, "One", "https://example.test/one");
        let unknown = SessionId::new("session-9999");
        let before = (
            book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true),
            book.focused(),
        );

        book.rename(&unknown, "New name");

        assert_eq!(
            (
                book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true),
                book.focused()
            ),
            before
        );
    }

    #[test]
    fn renaming_a_parked_account_keeps_it_parked() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.park(&id);

        book.rename(&id, "New name");

        assert_eq!(liveness_of(&book, &id), Liveness::Parked);
    }

    #[test]
    fn renaming_a_queued_account_keeps_it_queued() {
        let workspace = Workspace {
            accounts: vec![Account {
                liveness: SavedLiveness::Running,
                ..saved_account("session-0001", "Queued")
            }],
            layout: Layout::Single,
            ..Workspace::default()
        };
        let mut book = SessionBook::restore(workspace);
        let id = book.sessions()[0].id().clone();

        book.rename(&id, "New name");

        assert_eq!(liveness_of(&book, &id), Liveness::Queued);
    }

    #[test]
    fn renaming_to_a_name_another_account_already_has_is_accepted() {
        let mut book = SessionBook::new();
        let first = add(&mut book, "One", "https://example.test/one");
        let second = add(&mut book, "Two", "https://example.test/two");

        let stored = book.rename(&second, "One");

        assert_eq!(
            (
                stored,
                session(&book, &first).display_name(),
                session(&book, &second).display_name(),
            ),
            (true, "One", "One"),
        );
    }

    #[test]
    fn the_workspace_after_a_rename_carries_the_new_name() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");

        book.rename(&id, "Renamed");

        assert_eq!(
            book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true)
                .accounts[0]
                .display_name,
            "Renamed"
        );
    }

    fn snapshot(book: &SessionBook) -> (Workspace, SlotId) {
        (
            book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true),
            book.focused(),
        )
    }

    #[test]
    fn moving_onto_an_occupied_slot_returns_swapped_and_trades_exactly_those_two_slots() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");
        let three = add(&mut book, "Three", "https://example.test/three");

        let outcome = book.move_to_slot(&one, SlotId::new(1));

        assert_eq!(
            (
                outcome,
                visibility_of(&book, &one),
                visibility_of(&book, &two),
                visibility_of(&book, &three),
            ),
            (
                MoveOutcome::Swapped { with: two.clone() },
                Visibility::InSlot(SlotId::new(1)),
                Visibility::InSlot(SlotId::new(0)),
                Visibility::InSlot(SlotId::new(2)),
            ),
        );
    }

    #[test]
    fn moving_onto_an_empty_slot_returns_filled_and_leaves_the_old_slot_empty() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");

        let outcome = book.move_to_slot(&one, SlotId::new(2));

        assert_eq!(
            (outcome, visibility_of(&book, &one)),
            (MoveOutcome::Filled, Visibility::InSlot(SlotId::new(2))),
        );
        assert!(
            book.sessions()
                .iter()
                .all(|s| s.visibility() != Visibility::InSlot(SlotId::new(0)))
        );
    }

    #[test]
    fn moving_onto_the_movers_own_slot_returns_unchanged_and_leaves_the_book_equal() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        let before = snapshot(&book);

        let outcome = book.move_to_slot(&one, SlotId::new(0));

        assert_eq!((outcome, snapshot(&book)), (MoveOutcome::Unchanged, before));
    }

    #[test]
    fn moving_onto_a_slot_the_layout_does_not_have_returns_unchanged_and_leaves_the_book_equal() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        let before = snapshot(&book);

        let outcome = book.move_to_slot(&one, SlotId::new(3));

        assert_eq!((outcome, snapshot(&book)), (MoveOutcome::Unchanged, before));
    }

    #[test]
    fn moving_an_off_grid_account_returns_unchanged_and_leaves_the_book_equal() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));
        add(&mut book, "Three", "https://example.test/three");
        assert_eq!(visibility_of(&book, &one), Visibility::OffGrid);
        let before = snapshot(&book);

        let outcome = book.move_to_slot(&one, SlotId::new(1));

        assert_eq!((outcome, snapshot(&book)), (MoveOutcome::Unchanged, before));
    }

    #[test]
    fn moving_an_unknown_id_returns_unchanged_and_leaves_the_book_equal() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        let unknown = SessionId::new("session-9999");
        let before = snapshot(&book);

        let outcome = book.move_to_slot(&unknown, SlotId::new(1));

        assert_eq!((outcome, snapshot(&book)), (MoveOutcome::Unchanged, before));
    }

    #[test]
    fn a_swap_where_focus_was_on_the_movers_slot_moves_focus_to_the_target() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));

        book.move_to_slot(&one, SlotId::new(1));

        assert_eq!(book.focused(), SlotId::new(1));
    }

    #[test]
    fn a_fill_where_focus_was_on_the_movers_slot_moves_focus_to_the_target() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));

        book.move_to_slot(&one, SlotId::new(2));

        assert_eq!(book.focused(), SlotId::new(2));
    }

    #[test]
    fn a_swap_where_focus_was_on_the_target_moves_focus_to_the_movers_old_slot() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(1));

        book.move_to_slot(&one, SlotId::new(1));

        assert_eq!(book.focused(), SlotId::new(0));
    }

    #[test]
    fn a_swap_where_focus_was_on_neither_slot_leaves_focus_unchanged() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        add(&mut book, "Three", "https://example.test/three");
        book.set_focused(SlotId::new(2));

        book.move_to_slot(&one, SlotId::new(1));

        assert_eq!(book.focused(), SlotId::new(2));
    }

    #[test]
    fn a_swap_survives_a_layout_switch_and_back() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");

        book.move_to_slot(&one, SlotId::new(1));
        book.set_layout(Layout::SideBySide);
        book.set_layout(Layout::Grid);

        assert_eq!(
            (visibility_of(&book, &one), visibility_of(&book, &two)),
            (
                Visibility::InSlot(SlotId::new(1)),
                Visibility::InSlot(SlotId::new(0)),
            ),
        );
    }

    fn order(book: &SessionBook) -> Vec<SessionId> {
        book.sessions().iter().map(|s| s.id().clone()).collect()
    }

    #[test]
    fn a_real_move_reorders_in_slot_accounts_by_slot_index_with_no_off_grid_accounts_present() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");

        book.move_to_slot(&one, SlotId::new(1));

        assert_eq!(order(&book), vec![two, one]);
    }

    #[test]
    fn a_real_move_puts_in_slot_accounts_first_then_off_grid_accounts_in_their_old_relative_order()
    {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");
        book.set_focused(SlotId::new(0));
        let three = add(&mut book, "Three", "https://example.test/three");
        // One is now off-grid, Three took slot 0, Two still holds slot 1.
        assert_eq!(order(&book), vec![one.clone(), two.clone(), three.clone()]);

        book.move_to_slot(&two, SlotId::new(0));

        assert_eq!(order(&book), vec![two, three, one]);
    }

    #[test]
    fn moving_a_parked_account_keeps_it_parked() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.park(&one);

        book.move_to_slot(&one, SlotId::new(1));

        assert_eq!(liveness_of(&book, &one), Liveness::Parked);
    }

    #[test]
    fn a_move_changes_no_accounts_keep_awake_flag() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");
        book.set_keep_awake(&two, true);

        book.move_to_slot(&one, SlotId::new(1));

        assert!(is_kept_awake(&book, &two));
    }

    #[test]
    fn a_move_changes_no_accounts_remembered_zoom() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        let chosen = book.zoom_in(&one).expect("the account is in the book");

        book.move_to_slot(&one, SlotId::new(1));

        assert_eq!(session(&book, &one).zoom_for(Layout::SideBySide), chosen);
    }

    #[test]
    fn restoring_from_the_workspace_of_a_moved_book_reproduces_the_same_slots_and_order() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        add(&mut book, "Three", "https://example.test/three");
        let one = book.sessions()[0].id().clone();

        book.move_to_slot(&one, SlotId::new(2));

        let slots_and_order = |b: &SessionBook| -> Vec<(SessionId, Visibility)> {
            b.sessions()
                .iter()
                .map(|s| (s.id().clone(), s.visibility()))
                .collect()
        };
        let before = slots_and_order(&book);

        let restored = SessionBook::restore(book.workspace(
            WorkspaceId::ungrouped(),
            "Ungrouped".to_owned(),
            true,
        ));

        assert_eq!(slots_and_order(&restored), before);
    }

    #[test]
    fn take_removes_the_session_and_its_remembered_slot_leaving_others_seated() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");

        let taken = book.take(&one).expect("one is in the book");

        assert_eq!(
            (
                taken.id().clone(),
                book.sessions().iter().any(|s| s.id() == &one),
                visibility_of(&book, &two),
            ),
            (one, false, Visibility::InSlot(SlotId::new(1))),
        );
    }

    #[test]
    fn take_on_an_unknown_id_returns_none_and_changes_nothing() {
        let mut book = SessionBook::new();
        add(&mut book, "One", "https://example.test/one");
        let before = book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true);

        let taken = book.take(&SessionId::new("session-9999"));

        assert_eq!(
            (
                taken.is_none(),
                book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true)
            ),
            (true, before),
        );
    }

    #[test]
    fn adopt_appends_to_the_end_and_seats_in_the_lowest_free_slot() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        add(&mut book, "One", "https://example.test/one");
        let mut source = SessionBook::new();
        add(&mut source, "Filler", "https://example.test/filler");
        let two = add(&mut source, "Two", "https://example.test/two");
        let arriving = source.take(&two).expect("two is in the source book");

        book.adopt(arriving);

        assert_eq!(
            (
                book.sessions().last().map(Session::id),
                book.sessions().last().map(Session::visibility),
            ),
            (Some(&two), Some(Visibility::InSlot(SlotId::new(1)))),
        );
    }

    #[test]
    fn adopt_into_a_full_book_seats_the_arrival_off_grid_without_displacing_anyone() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Single);
        let one = add(&mut book, "One", "https://example.test/one");
        let mut source = SessionBook::new();
        add(&mut source, "Filler", "https://example.test/filler");
        let two = add(&mut source, "Two", "https://example.test/two");
        let arriving = source.take(&two).expect("two is in the source book");

        book.adopt(arriving);

        assert_eq!(
            (visibility_of(&book, &one), visibility_of(&book, &two)),
            (Visibility::InSlot(SlotId::FIRST), Visibility::OffGrid),
        );
    }

    #[test]
    fn adopt_carries_liveness_and_keep_awake_over_unchanged() {
        let mut source = SessionBook::new();
        let id = add(&mut source, "One", "https://example.test/one");
        source.park(&id);
        source.set_keep_awake(&id, true);
        let arriving = source.take(&id).expect("id is in the source book");

        let mut book = SessionBook::new();
        book.adopt(arriving);

        assert_eq!(
            (liveness_of(&book, &id), is_kept_awake(&book, &id)),
            (Liveness::Parked, true),
        );
    }
}
