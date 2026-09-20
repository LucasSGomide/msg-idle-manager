//! What a game account is: the identity the program mints for it, the two
//! things the user typed, and the book of accounts a workspace holds.
//!
//! A workspace's accounts have exactly one order — [`SessionBook::sessions`]
//! — and the screen shows one page of that order at a time, the page size set
//! by the layout. Nothing about where an account sits is stored: it is always
//! derived from its position in the order, the layout's slot count, and which
//! position is focused.

use std::collections::HashMap;

use crate::layout::{Layout, MoveOutcome, SlotId};
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
/// Never stored: it is [`SessionBook::placement`]'s derived answer, worked out
/// from an account's position in the order, the layout's slot count, and the
/// focused position (code standards rule 1) — there is no state where a slot
/// and a visibility could disagree, because there is no second field to
/// disagree with.
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
/// created with, whether it is running, and the view settings it carries from
/// its preset. Carries no seat of its own — [`SessionBook::placement`] derives
/// where it sits from its position in the book's order.
#[derive(Debug, Clone)]
pub struct Session {
    id: SessionId,
    display_name: String,
    start_address: String,
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

/// The application's sessions in the order they sit in, together with the
/// current layout and the position of the focused account.
///
/// A workspace's whole seating is these three things and nothing else
/// (`FR.22.1`, `FR.22.2`): with `k = layout.slot_count()`, the shown page is
/// `focused / k`, and every session's slot is derived from where it sits in
/// `sessions` relative to that page — never stored, so it can never drift from
/// the order (code standards rule 1). The book is the only place a
/// [`SessionId`] is minted, and every identifier it returns is distinct from
/// every one already in it.
#[derive(Debug)]
pub struct SessionBook {
    sessions: Vec<Session>,
    layout: Layout,
    focused: usize,
}

impl SessionBook {
    /// A new book with no sessions, arranged for the single-slot layout.
    #[must_use]
    pub fn new() -> Self {
        Self {
            sessions: Vec::new(),
            layout: Layout::Single,
            focused: 0,
        }
    }

    /// Rebuilds a book from a saved [`Workspace`].
    ///
    /// The accounts come back in the order the workspace held them, which is
    /// the one order this book keeps. One saved as running becomes
    /// [`Liveness::Queued`] — nothing is running yet — and one saved as parked
    /// stays [`Liveness::Parked`], costing nothing. The saved layout becomes
    /// active and the saved focused position is clamped to `0` when the book
    /// is empty, else below `sessions.len()` — a hand-edited file naming a
    /// position past the end of its own list never leaves the book focused
    /// nowhere. The workspace's `id`, `name` and `is_expanded` are not this
    /// book's to keep — [`crate::WorkspaceBook`] carries those alongside the
    /// book it restores here — and minting is likewise
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

        let sessions: Vec<Session> = accounts
            .into_iter()
            .map(|account| Session {
                id: account.id,
                display_name: account.display_name,
                start_address: account.start_address,
                liveness: match account.liveness {
                    SavedLiveness::Running => Liveness::Queued,
                    SavedLiveness::Parked => Liveness::Parked,
                },
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

        let focused = focused.min(sessions.len().saturating_sub(1));

        Self {
            sessions,
            layout,
            focused,
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
    /// zoom, identity and keep-awake flag, plus the active layout and the
    /// focused position. A [`Liveness::Starting`] or [`Liveness::Queued`]
    /// account is reported as [`SavedLiveness::Running`]: those describe a
    /// moment, not a wish.
    #[must_use]
    pub fn workspace(&self, id: WorkspaceId, name: String, is_expanded: bool) -> Workspace {
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
                is_kept_awake: session.is_kept_awake,
                browser_identity: session.browser_identity.clone(),
                zoom: session.preset_zoom,
            })
            .collect();

        Workspace {
            id,
            name,
            focused: self.focused,
            is_expanded,
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

    /// Switches to `layout`. Nothing else changes: the page shown is
    /// recomputed from the unchanged focused position, which is what keeps
    /// the focused account on screen across a layout switch (`FR.22.2`
    /// refining `FR.3.2`).
    pub fn set_layout(&mut self, layout: Layout) {
        self.layout = layout;
    }

    /// The page currently shown, counting from zero.
    #[must_use]
    pub fn page(&self) -> usize {
        self.page_of(self.focused)
    }

    /// How many pages this book's order divides into at the current layout —
    /// `0` for an empty book, otherwise `sessions.len()` divided by the
    /// layout's slot count, rounded up so a part-empty last page still counts.
    #[must_use]
    pub fn page_count(&self) -> usize {
        if self.sessions.is_empty() {
            0
        } else {
            self.sessions.len().div_ceil(self.layout.slot_count())
        }
    }

    /// Where `id`'s account sits: [`Visibility::InSlot`] with its slot on the
    /// shown page, [`Visibility::OffGrid`] on any other page, or `None` if
    /// this book does not hold `id` at all.
    #[must_use]
    pub fn placement(&self, id: &SessionId) -> Option<Visibility> {
        let position = self.sessions.iter().position(|s| &s.id == id)?;
        if self.page_of(position) == self.page() {
            Some(Visibility::InSlot(self.slot_of(position)))
        } else {
            Some(Visibility::OffGrid)
        }
    }

    /// The slot the focused account sits in on the shown page.
    #[must_use]
    pub fn focused_slot(&self) -> SlotId {
        self.slot_of(self.focused)
    }

    /// The account sitting in the focused position, or `None` for an empty
    /// book. The one place the shell asks which account a keyboard gesture
    /// acts on.
    #[must_use]
    pub fn focused_session(&self) -> Option<&Session> {
        self.sessions.get(self.focused)
    }

    /// Focuses whichever account sits in `slot` on the shown page, and reports
    /// whether that position holds an account. `false`, changing nothing, for
    /// a slot outside the layout or a trailing slot the order does not reach —
    /// a click on an empty slot leaves the focus where it was.
    pub fn focus_slot(&mut self, slot: SlotId) -> bool {
        if !self.layout.contains(slot) {
            return false;
        }
        let target = self.page() * self.layout.slot_count() + slot.index();
        if target < self.sessions.len() {
            self.focused = target;
            true
        } else {
            false
        }
    }

    /// Focuses `id`'s account and reports whether the shown page changed.
    /// Leaves the book untouched for an id this book does not hold.
    pub fn focus_session(&mut self, id: &SessionId) -> bool {
        let Some(position) = self.sessions.iter().position(|s| &s.id == id) else {
            return false;
        };
        let page_before = self.page();
        self.focused = position;
        self.page() != page_before
    }

    /// Moves focus to the next account in order, wrapping from the last back
    /// to the first, and reports whether it moved. `false`, changing nothing,
    /// for a book of one account or none — there is nowhere else to walk to
    /// (`FR.23.1`). Stepping past the end of a page turns to the next page on
    /// its own, since the page is derived from the focused position alone.
    pub fn focus_next(&mut self) -> bool {
        if self.sessions.len() <= 1 {
            return false;
        }
        self.focused = (self.focused + 1) % self.sessions.len();
        true
    }

    /// Turns to the first position of the next page, wrapping from the last
    /// page back to the first, and reports whether it moved. `false`,
    /// changing nothing, for a book of one page or none (`FR.22.5`).
    pub fn next_page(&mut self) -> bool {
        let Some(pages) = self.pages_to_step_between() else {
            return false;
        };
        self.step_page((self.page() + 1) % pages);
        true
    }

    /// Turns to the first position of the previous page, wrapping from the
    /// first page back to the last, otherwise exactly
    /// [`SessionBook::next_page`].
    pub fn previous_page(&mut self) -> bool {
        let Some(pages) = self.pages_to_step_between() else {
            return false;
        };
        self.step_page((self.page() + pages - 1) % pages);
        true
    }

    /// The page count, when there is more than one page to turn between —
    /// `None` for a book of one page or none, the shared no-op case for
    /// [`SessionBook::next_page`] and [`SessionBook::previous_page`].
    fn pages_to_step_between(&self) -> Option<usize> {
        let pages = self.page_count();
        (pages > 1).then_some(pages)
    }

    /// Lands the focused position on `page`'s first slot.
    fn step_page(&mut self, page: usize) {
        self.focused = page * self.layout.slot_count();
    }

    /// Adds a new account created from a typed address under `id`, appends it
    /// to the end of the order, and focuses it — the shown page turns to it
    /// and nothing is displaced (`FR.22.1`).
    ///
    /// `id` is minted by the caller — [`crate::WorkspaceBook`], from the one
    /// counter shared by every workspace — never by this book (code standards
    /// rule 2). It carries no browser identity and the default zoom —
    /// everything a preset would have supplied falls back here.
    pub fn add(&mut self, id: &SessionId, display_name: &str, start_address: &str) {
        self.sessions.push(Session {
            id: id.clone(),
            display_name: display_name.to_owned(),
            start_address: start_address.to_owned(),
            liveness: Liveness::Live,
            is_kept_awake: false,
            browser_identity: None,
            is_webgl_enabled: true,
            preset_zoom: ZoomLevel::DEFAULT,
            remembered_zoom: RememberedZoom::new(),
        });
        self.focused = self.sessions.len() - 1;
    }

    /// Adds a new account under `id`, playing `preset`'s game under the name
    /// `account_name`, appends and focuses it exactly as [`SessionBook::add`]
    /// does, and copies the preset's start address, zoom and browser identity
    /// onto it.
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
        self.sessions.push(Session {
            id: id.clone(),
            display_name: account_name.to_owned(),
            start_address: preset.start_address.clone(),
            liveness: Liveness::Live,
            is_kept_awake: false,
            browser_identity: preset.browser_identity.clone(),
            is_webgl_enabled: preset.webgl_enabled,
            preset_zoom: preset.zoom,
            remembered_zoom: RememberedZoom::new(),
        });
        self.focused = self.sessions.len() - 1;

        self.set_keep_awake(id, preset.keep_awake_default);
        self.mark_started(id);
    }

    /// Park `session`: its liveness becomes [`Liveness::Parked`]. Parking an
    /// already-parked session changes nothing. Returns the session's liveness
    /// after the call; an id not in the book changes nothing and returns
    /// [`Liveness::Live`].
    pub fn park(&mut self, session: &SessionId) -> Liveness {
        self.set_liveness(session, Liveness::Parked)
    }

    /// Unpark `session`: a parked *or* queued session becomes
    /// [`Liveness::Starting`], because no page has painted yet — a queued
    /// account is one the start queue has just reached. A live or
    /// already-starting session is left as it is. Returns the session's
    /// liveness after the call.
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

    /// Queue `session`: a parked session becomes [`Liveness::Queued`], the new
    /// step toward being brought back one at a time by the start queue
    /// (`Start all`, `FR.24.2`). Any other liveness is left as it is. Returns
    /// the session's liveness after the call; an id not in the book changes
    /// nothing and returns [`Liveness::Live`].
    pub fn queue(&mut self, session: &SessionId) -> Liveness {
        let Some(current) = self.liveness_of(session) else {
            return Liveness::Live;
        };
        if current == Liveness::Parked {
            self.set_liveness(session, Liveness::Queued)
        } else {
            current
        }
    }

    /// End `session`'s starting interval once the shell reports its first
    /// paint: [`Liveness::Starting`] becomes [`Liveness::Live`]. Any other
    /// state is left as it is. Returns the session's liveness after the call.
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
    /// `account`'s display name and nothing else: its id, liveness, keep-awake
    /// flag, the book's focused position and the order of its sessions are
    /// untouched (`FR.13.2`, `FR.13.5`). Two accounts may share a name, since
    /// [`SessionBook::add`] already allows that.
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
    /// locked (Remote Access `FR.3.2`). Liveness is untouched — a zoom is not
    /// a reload (`FR.11.5`).
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

    /// Removes `id` from this book and returns it. The focused position keeps
    /// the same account focused unless `id` was that account, in which case it
    /// clamps to the new last position (`0` for an emptied book). `None` for
    /// an id this book does not hold.
    pub fn take(&mut self, id: &SessionId) -> Option<Session> {
        let index = self.sessions.iter().position(|s| &s.id == id)?;
        let focused_before = self.focused_session().map(|s| s.id.clone());
        let taken = self.sessions.remove(index);
        self.keep_focus_on(focused_before);
        Some(taken)
    }

    /// Appends an arriving `session` to the end of this book's list and
    /// leaves the focused position exactly as it was — an account moved into
    /// a hidden workspace never changes what that workspace shows. Every
    /// field on `session` — liveness, keep-awake, remembered zoom — is
    /// carried over unchanged (`FR.17.3`).
    pub fn adopt(&mut self, session: Session) {
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

    /// Move `account` to `target`, a slot on the shown page, and report what
    /// happened (`FR.22.4`).
    ///
    /// The source position is `account`'s index; the target position is
    /// `page() * k + target.index()`. An occupied target trades the two `Vec`
    /// entries directly ([`MoveOutcome::Swapped`]); a target past the end of
    /// the order removes the account from its own position and pushes it to
    /// the end ([`MoveOutcome::Filled`]). [`MoveOutcome::Unchanged`], with the
    /// book untouched, when the target is the source, is not a slot the
    /// layout has, or `account` is unknown or not on the shown page — it
    /// cannot be on another page since only the shown page has places, but
    /// the book's own answer is trusted over the caller's. Either real move
    /// keeps every other page's accounts at their own positions, since both
    /// edits stay inside the shown page's slice of the order, and moves focus
    /// with the dragged account (`FR.14.3`). No liveness, keep-awake flag or
    /// remembered zoom is read or written (`FR.14.8`).
    pub fn move_to_slot(&mut self, account: &SessionId, target: SlotId) -> MoveOutcome {
        if !self.layout.contains(target) {
            return MoveOutcome::Unchanged;
        }
        let Some(source) = self.sessions.iter().position(|s| &s.id == account) else {
            return MoveOutcome::Unchanged;
        };
        if self.page_of(source) != self.page() {
            return MoveOutcome::Unchanged;
        }

        let destination = self.page() * self.layout.slot_count() + target.index();
        if destination == source {
            return MoveOutcome::Unchanged;
        }

        let focused_before = self.focused_session().map(|s| s.id.clone());

        let outcome = if destination < self.sessions.len() {
            let with = self.sessions[destination].id.clone();
            self.sessions.swap(source, destination);
            MoveOutcome::Swapped { with }
        } else {
            let session = self.sessions.remove(source);
            self.sessions.push(session);
            MoveOutcome::Filled
        };

        self.keep_focus_on(focused_before);
        outcome
    }

    /// Moves the focused position to wherever the account that held it —
    /// `id_before` — now sits, so a reorder or a removal never changes which
    /// account is focused unless that account is gone, in which case the
    /// position clamps to `0` when the book is empty, else below
    /// `sessions.len()`. Run after every mutation that can move or drop an
    /// account (`take`, `move_to_slot`).
    fn keep_focus_on(&mut self, id_before: Option<SessionId>) {
        if let Some(id) = id_before
            && let Some(position) = self.sessions.iter().position(|s| s.id == id)
        {
            self.focused = position;
            return;
        }
        self.focused = self.focused.min(self.sessions.len().saturating_sub(1));
    }

    /// The page a position falls on, at the book's current layout.
    fn page_of(&self, position: usize) -> usize {
        position / self.layout.slot_count()
    }

    /// The slot a position occupies within its own page.
    fn slot_of(&self, position: usize) -> SlotId {
        SlotId::new(position % self.layout.slot_count())
    }

    /// The focused position, raw — for [`crate::WorkspaceBook`]'s mobile-mode
    /// snapshot alone, which must save and restore an exact position rather
    /// than the slot a particular layout happens to derive it into.
    #[must_use]
    pub(crate) fn focused_position(&self) -> usize {
        self.focused
    }

    /// Restores a focused position saved earlier by
    /// [`SessionBook::focused_position`], clamped the same way every mutation
    /// clamps it in case an account gone since made it invalid.
    pub(crate) fn restore_focus(&mut self, position: usize) {
        self.focused = position.min(self.sessions.len().saturating_sub(1));
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

    fn session<'a>(book: &'a SessionBook, id: &SessionId) -> &'a Session {
        book.sessions()
            .iter()
            .find(|session| session.id() == id)
            .expect("session present")
    }

    fn liveness_of(book: &SessionBook, id: &SessionId) -> Liveness {
        session(book, id).liveness()
    }

    fn is_kept_awake(book: &SessionBook, id: &SessionId) -> bool {
        session(book, id).is_kept_awake()
    }

    fn order(book: &SessionBook) -> Vec<SessionId> {
        book.sessions().iter().map(|s| s.id().clone()).collect()
    }

    #[test]
    fn a_minted_identifier_is_distinct_from_every_existing_one() {
        let mut book = SessionBook::new();

        let first = add(&mut book, "One", "https://example.test/one");
        let second = add(&mut book, "Two", "https://example.test/two");

        assert_ne!(first, second);
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

    /// Acceptance: four accounts in `SideBySide` with the third focused —
    /// `page()` is `1`, `page_count()` is `2`, `placement` answers `InSlot(0)`
    /// and `InSlot(1)` for the third and fourth and `OffGrid` for the first
    /// two.
    #[test]
    fn four_accounts_in_side_by_side_with_the_third_focused_show_the_second_page() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");
        let three = add(&mut book, "Three", "https://example.test/three");
        let four = add(&mut book, "Four", "https://example.test/four");
        book.focus_session(&three);

        assert_eq!(
            (
                book.page(),
                book.page_count(),
                book.placement(&three),
                book.placement(&four),
                book.placement(&one),
                book.placement(&two),
            ),
            (
                1,
                2,
                Some(Visibility::InSlot(SlotId::new(0))),
                Some(Visibility::InSlot(SlotId::new(1))),
                Some(Visibility::OffGrid),
                Some(Visibility::OffGrid),
            ),
        );
    }

    /// Acceptance: four accounts in `SideBySide` starting on the first,
    /// `focus_next` four times visits the second, third, fourth and first,
    /// with `page()` reading `0, 1, 1, 0`.
    #[test]
    fn focus_next_walks_four_accounts_in_side_by_side_turning_pages_as_it_goes() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");
        let three = add(&mut book, "Three", "https://example.test/three");
        let four = add(&mut book, "Four", "https://example.test/four");
        book.focus_session(&one);

        let mut visited = Vec::new();
        let mut pages = Vec::new();
        for _ in 0..4 {
            book.focus_next();
            visited.push(book.focused_session().map(Session::id).cloned());
            pages.push(book.page());
        }

        assert_eq!(
            (visited, pages),
            (
                vec![Some(two), Some(three), Some(four), Some(one)],
                vec![0, 1, 1, 0],
            ),
        );
    }

    /// Acceptance: `focus_next` on a book of one account and on an empty book
    /// returns `false` and changes nothing.
    #[test]
    fn focus_next_on_a_book_of_one_or_no_accounts_returns_false_and_changes_nothing() {
        let mut one_book = SessionBook::new();
        let only = add(&mut one_book, "One", "https://example.test/one");
        let mut empty_book = SessionBook::new();

        let moved_one = one_book.focus_next();
        let moved_empty = empty_book.focus_next();

        assert_eq!(
            (
                moved_one,
                one_book.focused_session().map(Session::id),
                moved_empty,
            ),
            (false, Some(&only), false),
        );
    }

    /// Acceptance: `next_page` lands on the first position of the next page
    /// and wraps from the last page to the first; `previous_page` wraps from
    /// the first page to the last.
    #[test]
    fn next_page_and_previous_page_wrap_at_the_ends() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        let three = add(&mut book, "Three", "https://example.test/three");
        add(&mut book, "Four", "https://example.test/four");
        book.focus_session(&one);

        let moved_forward = book.next_page();
        let after_forward = book.focused_session().map(Session::id).cloned();
        let wrapped_forward = book.next_page();
        let after_wrap_forward = book.focused_session().map(Session::id).cloned();
        let wrapped_back = book.previous_page();
        let after_wrap_back = book.focused_session().map(Session::id).cloned();

        assert_eq!(
            (
                moved_forward,
                after_forward,
                wrapped_forward,
                after_wrap_forward,
                wrapped_back,
                after_wrap_back,
            ),
            (
                true,
                Some(three.clone()),
                true,
                Some(one),
                true,
                Some(three),
            ),
        );
    }

    /// Acceptance: `next_page` and `previous_page` return `false` on a
    /// one-page book, and on three accounts in `SideBySide` `next_page` from
    /// page `0` focuses the third account alone on page `1`.
    #[test]
    fn next_page_and_previous_page_are_no_ops_on_a_single_page_and_next_page_lands_alone_on_a_partial_page()
     {
        let mut single_page = SessionBook::new();
        add(&mut single_page, "One", "https://example.test/one");

        let next_on_single_page = single_page.next_page();
        let previous_on_single_page = single_page.previous_page();

        let mut three_book = SessionBook::new();
        three_book.set_layout(Layout::SideBySide);
        let one = add(&mut three_book, "One", "https://example.test/one");
        add(&mut three_book, "Two", "https://example.test/two");
        let three = add(&mut three_book, "Three", "https://example.test/three");
        three_book.focus_session(&one);

        let moved = three_book.next_page();

        assert_eq!(
            (
                next_on_single_page,
                previous_on_single_page,
                moved,
                three_book.focused_session().map(Session::id),
            ),
            (false, false, true, Some(&three)),
        );
    }

    /// Acceptance: switching from `Grid` to `Single` with the third of four
    /// focused answers `InSlot(0)` for that account, and switching back to
    /// `Grid` answers a slot for all four again.
    #[test]
    fn switching_layout_keeps_the_focused_account_on_screen_and_back_shows_everyone_again() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");
        let three = add(&mut book, "Three", "https://example.test/three");
        let four = add(&mut book, "Four", "https://example.test/four");
        book.focus_session(&three);

        book.set_layout(Layout::Single);
        let while_single = book.placement(&three);

        book.set_layout(Layout::Grid);
        let while_grid_again = [&one, &two, &three, &four].map(|id| book.placement(id));

        assert_eq!(
            (while_single, while_grid_again),
            (
                Some(Visibility::InSlot(SlotId::FIRST)),
                [
                    Some(Visibility::InSlot(SlotId::new(0))),
                    Some(Visibility::InSlot(SlotId::new(1))),
                    Some(Visibility::InSlot(SlotId::new(2))),
                    Some(Visibility::InSlot(SlotId::new(3))),
                ],
            ),
        );
    }

    /// Acceptance: `focus_slot` on a trailing slot past the end of the order
    /// returns `false` and leaves `focused_session()` unchanged.
    #[test]
    fn focus_slot_past_the_end_of_the_order_changes_nothing() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");

        let changed = book.focus_slot(SlotId::new(3));

        assert_eq!(
            (changed, book.focused_session().map(Session::id)),
            (false, Some(&one)),
        );
    }

    /// Acceptance: `add` and `add_from_preset` append to the order and focus
    /// the newcomer; `adopt` appends and leaves `focused_session()` as it was.
    #[test]
    fn add_and_add_from_preset_focus_the_newcomer_while_adopt_leaves_focus_alone() {
        let mut book = SessionBook::new();
        let one = add(&mut book, "One", "https://example.test/one");

        let two = add(&mut book, "Two", "https://example.test/two");
        let focus_after_add = book.focused_session().map(Session::id).cloned();

        let three = add_from_preset(&mut book, "Alt", &a_preset());
        let focus_after_add_from_preset = book.focused_session().map(Session::id).cloned();

        let mut source = SessionBook::new();
        let arriving_id = add(&mut source, "Filler", "https://example.test/filler");
        let arriving = source.take(&arriving_id).expect("just added to source");
        book.adopt(arriving);
        let focus_after_adopt = book.focused_session().map(Session::id).cloned();

        assert_eq!(
            (
                focus_after_add,
                focus_after_add_from_preset,
                focus_after_adopt,
            ),
            (Some(two), Some(three.clone()), Some(three)),
        );
        assert!(order(&book).contains(&one));
    }

    /// Acceptance: taking the account before the focused one keeps
    /// `focused_session()` the same account; taking the focused last account
    /// clamps to the new last.
    #[test]
    fn taking_an_account_keeps_focus_on_the_same_account_unless_it_is_the_one_taken() {
        let mut book = SessionBook::new();
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");
        let three = add(&mut book, "Three", "https://example.test/three");
        book.focus_session(&three);

        book.take(&one);
        let focus_after_removing_before = book.focused_session().map(Session::id).cloned();

        book.take(&three);
        let focus_after_removing_focused_last = book.focused_session().map(Session::id).cloned();

        assert_eq!(
            (
                focus_after_removing_before,
                focus_after_removing_focused_last
            ),
            (Some(three), Some(two)),
        );
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
            is_kept_awake: false,
            browser_identity: None,
            zoom: ZoomLevel::DEFAULT,
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
    fn a_focused_position_past_the_restored_orders_end_clamps_to_the_last_account() {
        let workspace = Workspace {
            accounts: vec![
                saved_account("session-0001", "A"),
                saved_account("session-0002", "B"),
            ],
            focused: 9,
            layout: Layout::Single,
            ..Workspace::default()
        };

        let book = SessionBook::restore(workspace);

        assert_eq!(
            book.focused_session().map(Session::id),
            Some(&SessionId::new("session-0002")),
        );
    }

    #[test]
    fn restoring_an_empty_workspace_with_a_nonzero_focused_position_stays_at_zero() {
        let workspace = Workspace {
            accounts: Vec::new(),
            focused: 3,
            layout: Layout::default(),
            ..Workspace::default()
        };

        let book = SessionBook::restore(workspace);

        assert!(book.focused_session().is_none());
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
                    ..saved_account("session-0001", "A")
                },
                Account {
                    liveness: SavedLiveness::Parked,
                    browser_identity: Some("UA/2".to_owned()),
                    ..saved_account("session-0002", "B")
                },
            ],
            focused: 1,
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
    fn unparking_a_parked_session_returns_starting() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.park(&id);

        let state = book.unpark(&id);

        assert_eq!(state, Liveness::Starting);
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
    fn queuing_a_parked_session_returns_queued() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.park(&id);

        let state = book.queue(&id);

        assert_eq!(state, Liveness::Queued);
    }

    #[test]
    fn queuing_a_live_session_returns_live_and_changes_nothing_else() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        let placement = book.placement(&id);

        let state = book.queue(&id);

        assert_eq!(
            (state, liveness_of(&book, &id), book.placement(&id)),
            (Liveness::Live, Liveness::Live, placement)
        );
    }

    #[test]
    fn parking_an_already_parked_session_returns_parked_and_changes_nothing_else() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.park(&id);
        let placement = book.placement(&id);

        let state = book.park(&id);

        assert_eq!((state, book.placement(&id)), (Liveness::Parked, placement));
    }

    #[test]
    fn unparking_a_live_session_returns_live_and_changes_nothing_else() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        let placement = book.placement(&id);

        let state = book.unpark(&id);

        assert_eq!(
            (state, liveness_of(&book, &id), book.placement(&id)),
            (Liveness::Live, Liveness::Live, placement)
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
    fn keep_awake_survives_a_layout_change_that_moves_the_account_between_pages() {
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
    fn a_step_leaves_the_accounts_liveness_and_placement_exactly_as_they_were() {
        let mut book = SessionBook::new();
        let id = add(&mut book, "One", "https://example.test/one");
        book.park(&id);
        let before = (liveness_of(&book, &id), book.placement(&id));

        book.zoom_in(&id);

        assert_eq!((liveness_of(&book, &id), book.placement(&id)), before);
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
    fn focused_session_returns_the_account_at_the_focused_position() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");

        assert_eq!(book.focused_session().map(Session::id), Some(&two));
    }

    #[test]
    fn focused_session_is_none_for_an_empty_book() {
        let book = SessionBook::new();

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
        book.focus_session(&first);
        let before = (
            liveness_of(&book, &first),
            book.placement(&first),
            is_kept_awake(&book, &first),
            book.focused_session().map(Session::id).cloned(),
            order(&book),
        );

        book.rename(&first, "  New name  ");

        let after = (
            liveness_of(&book, &first),
            book.placement(&first),
            is_kept_awake(&book, &first),
            book.focused_session().map(Session::id).cloned(),
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
        let before = book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true);

        book.rename(&id, "   ");

        assert_eq!(
            book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true),
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
        let before = book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true);

        book.rename(&unknown, "New name");

        assert_eq!(
            book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true),
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

    fn snapshot(book: &SessionBook) -> (Workspace, usize) {
        (
            book.workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true),
            book.focused_position(),
        )
    }

    /// Acceptance (part 1 of 4): `move_to_slot` onto an occupied slot returns
    /// `Swapped` and trades exactly those two positions.
    #[test]
    fn moving_onto_an_occupied_slot_returns_swapped_and_trades_exactly_those_two_positions() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");
        let three = add(&mut book, "Three", "https://example.test/three");

        let outcome = book.move_to_slot(&one, SlotId::new(1));

        assert_eq!(
            (
                outcome,
                book.placement(&one),
                book.placement(&two),
                book.placement(&three),
            ),
            (
                MoveOutcome::Swapped { with: two.clone() },
                Some(Visibility::InSlot(SlotId::new(1))),
                Some(Visibility::InSlot(SlotId::new(0))),
                Some(Visibility::InSlot(SlotId::new(2))),
            ),
        );
    }

    /// Acceptance (part 2 of 4): onto a position past the order's end,
    /// `move_to_slot` returns `Filled` and moves the account to the end.
    #[test]
    fn moving_onto_a_position_past_the_orders_end_returns_filled_and_moves_the_account_to_the_end()
    {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");

        let outcome = book.move_to_slot(&one, SlotId::new(2));

        assert_eq!(
            (outcome, order(&book), book.placement(&one)),
            (
                MoveOutcome::Filled,
                vec![two, one],
                Some(Visibility::InSlot(SlotId::new(1))),
            ),
        );
    }

    /// Acceptance (part 3 of 4): a move on the shown page never touches
    /// another page's order or placement.
    #[test]
    fn a_move_on_the_shown_page_leaves_every_other_pages_order_and_placement_untouched() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        let three = add(&mut book, "Three", "https://example.test/three");
        let four = add(&mut book, "Four", "https://example.test/four");
        book.focus_session(&one);
        let other_page_before = (
            order(&book)[2..].to_vec(),
            book.placement(&three),
            book.placement(&four),
        );

        book.move_to_slot(&one, SlotId::new(1));

        let other_page_after = (
            order(&book)[2..].to_vec(),
            book.placement(&three),
            book.placement(&four),
        );
        assert_eq!(other_page_after, other_page_before);
    }

    /// Acceptance (part 4 of 4): a move keeps focus on the account that had
    /// it, wherever the move lands it.
    #[test]
    fn a_move_keeps_focus_on_the_same_account_wherever_it_lands() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        book.focus_session(&one);

        book.move_to_slot(&one, SlotId::new(1));

        assert_eq!(book.focused_session().map(Session::id), Some(&one));
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
    fn moving_an_off_page_account_returns_unchanged_and_leaves_the_book_equal() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        let three = add(&mut book, "Three", "https://example.test/three");
        book.focus_session(&three);
        assert_eq!(book.placement(&one), Some(Visibility::OffGrid));
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
    fn a_real_move_reorders_the_two_traded_positions_only() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::SideBySide);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");

        book.move_to_slot(&one, SlotId::new(1));

        assert_eq!(order(&book), vec![two, one]);
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
    fn restoring_from_the_workspace_of_a_moved_book_reproduces_the_same_order() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        add(&mut book, "One", "https://example.test/one");
        add(&mut book, "Two", "https://example.test/two");
        add(&mut book, "Three", "https://example.test/three");
        let one = book.sessions()[0].id().clone();

        book.move_to_slot(&one, SlotId::new(2));

        let before = order(&book);

        let restored = SessionBook::restore(book.workspace(
            WorkspaceId::ungrouped(),
            "Ungrouped".to_owned(),
            true,
        ));

        assert_eq!(order(&restored), before);
    }

    #[test]
    fn take_removes_the_session_leaving_others_in_place() {
        let mut book = SessionBook::new();
        book.set_layout(Layout::Grid);
        let one = add(&mut book, "One", "https://example.test/one");
        let two = add(&mut book, "Two", "https://example.test/two");

        let taken = book.take(&one).expect("one is in the book");

        assert_eq!(
            (
                taken.id().clone(),
                book.sessions().iter().any(|s| s.id() == &one),
                book.placement(&two),
            ),
            (one, false, Some(Visibility::InSlot(SlotId::new(0)))),
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
    fn adopt_appends_to_the_end_of_the_order() {
        let mut book = SessionBook::new();
        add(&mut book, "One", "https://example.test/one");
        let mut source = SessionBook::new();
        add(&mut source, "Filler", "https://example.test/filler");
        let two = add(&mut source, "Two", "https://example.test/two");
        let arriving = source.take(&two).expect("two is in the source book");

        book.adopt(arriving);

        assert_eq!(book.sessions().last().map(Session::id), Some(&two));
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
