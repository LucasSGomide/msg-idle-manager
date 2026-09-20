//! [`WorkspaceBook`]: the runtime parent of every workspace's [`SessionBook`]
//! — naming rule 9, a book of workspaces the way [`SessionBook`] is a book of
//! sessions.
//!
//! Every account belongs to exactly one workspace, each workspace is one
//! [`SessionBook`], and the built-in Ungrouped workspace always exists and is
//! always last. This is the one place a [`SessionId`] or a [`WorkspaceId`] is
//! minted, from counters shared across every workspace, so an id is unique
//! app-wide and a deleted account's number is never handed to a new one
//! (`FR.21.7`).

use std::collections::HashMap;

use crate::layout::Layout;
use crate::preset::Preset;
use crate::remote::Viewport;
use crate::session::{Liveness, SessionBook, SessionId, Visibility, WorkspaceId, workspace_name};
use crate::workspace::{Workspace, WorkspaceList};

/// What [`WorkspaceBook::focus_account`] did beyond focusing the account: it
/// switched which workspace is shown, from `from` to `to`. Distinct from
/// [`crate::Outcome`], which describes a placement inside one book — this
/// describes moving between books, which is the shell's cue to flip the
/// header's layout toggles and snap the incoming workspace's zoom, without
/// treating it as a second layout switch of its own (`FR.16.3`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switch {
    from: WorkspaceId,
    to: WorkspaceId,
}

impl Switch {
    /// The workspace that was shown before the switch.
    #[must_use]
    pub fn from(&self) -> &WorkspaceId {
        &self.from
    }

    /// The workspace shown after the switch.
    #[must_use]
    pub fn to(&self) -> &WorkspaceId {
        &self.to
    }
}

/// One named workspace [`WorkspaceBook::destinations`] offers: enough of its
/// identity for a caller to name it in a menu and move accounts into it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DestinationWorkspace {
    id: WorkspaceId,
    name: String,
}

impl DestinationWorkspace {
    /// This workspace's identifier.
    #[must_use]
    pub fn id(&self) -> &WorkspaceId {
        &self.id
    }

    /// This workspace's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Where a set of accounts could go, answered by [`WorkspaceBook::destinations`]
/// so the move menu and the add-game field ask the one question and can never
/// offer a place the book would refuse (`FR.17.2`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destinations {
    workspaces: Vec<DestinationWorkspace>,
    can_create: bool,
}

impl Destinations {
    /// Every offered workspace, in sidebar order: the named workspaces with
    /// room, then Ungrouped last — Ungrouped always has room, so it is always
    /// present.
    #[must_use]
    pub fn workspaces(&self) -> &[DestinationWorkspace] {
        &self.workspaces
    }

    /// Whether a brand new named workspace could hold this many accounts.
    #[must_use]
    pub fn can_create(&self) -> bool {
        self.can_create
    }
}

impl Default for Destinations {
    /// Nothing offered and nothing creatable — the placeholder a widget holds
    /// before the window has ever supplied a real answer.
    fn default() -> Self {
        Self {
            workspaces: Vec::new(),
            can_create: false,
        }
    }
}

/// How many accounts a named workspace may hold at once — the number of
/// places the largest arrangement has. Derived from [`Layout::Grid`] rather
/// than written as a bare `4` (code standards rule 5).
const NAMED_WORKSPACE_CAPACITY: usize = Layout::Grid.slot_count();

/// One workspace's runtime state: its identity and name, whether its sidebar
/// heading is expanded, and the book of accounts it holds. Everything a
/// [`crate::Workspace`] carries except the accounts, the layout and the focused
/// place, which live on the [`SessionBook`] itself.
#[derive(Debug)]
struct Entry {
    id: WorkspaceId,
    name: String,
    is_expanded: bool,
    book: SessionBook,
}

/// One workspace as a reader outside this module sees it: its identity, name
/// and expansion, plus the book of accounts it holds. Read-only — changing a
/// workspace goes through a [`WorkspaceBook`] method, never through a value
/// borrowed from it.
#[derive(Debug)]
pub struct WorkspaceView<'a> {
    id: &'a WorkspaceId,
    name: &'a str,
    is_expanded: bool,
    book: &'a SessionBook,
}

impl<'a> WorkspaceView<'a> {
    /// This workspace's identifier.
    #[must_use]
    pub fn id(&self) -> &'a WorkspaceId {
        self.id
    }

    /// This workspace's name.
    #[must_use]
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Whether this workspace's sidebar heading is expanded.
    #[must_use]
    pub fn is_expanded(&self) -> bool {
        self.is_expanded
    }

    /// This workspace's own book of accounts.
    #[must_use]
    pub fn book(&self) -> &'a SessionBook {
        self.book
    }
}

/// Why a [`WorkspaceBook`] operation could not be carried out — a named
/// reason rather than a bare failure, so the shell can grey out a button from
/// the same answer this book would give (code standards rules 1, 12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceRefusal {
    /// A workspace name was empty once trimmed.
    EmptyName,
    /// A workspace name matched another's, ignoring case.
    NameTaken,
    /// The destination cannot hold the whole set being added or moved.
    NoRoom,
    /// The built-in Ungrouped workspace cannot be renamed or removed.
    Ungrouped,
    /// No workspace has this identifier.
    UnknownWorkspace,
    /// No workspace holds this account.
    UnknownAccount,
}

/// Mobile mode while it is on: the phone-shaped viewport the one slot takes,
/// and the layout and focused position each workspace had before the mode
/// reached it, keyed by workspace (roadmap item 13, Remote Access
/// `FR.3.1`–`FR.3.5`). The order is untouched by the mode, so it is not part
/// of the snapshot.
///
/// `Option<MobileMode>` on the book is the whole state: off means no
/// viewport and nothing to restore, on means both, and no third combination
/// can be written (code standards rule 1).
#[derive(Debug)]
struct MobileMode {
    viewport: Viewport,
    snapshots: HashMap<WorkspaceId, (Layout, usize)>,
}

/// The runtime parent of every workspace's [`SessionBook`].
///
/// An ordered list of workspaces, each one [`SessionBook`] plus the identity,
/// name and expansion state that book does not carry itself, the active
/// (shown) workspace's id, the two counters every mint reads from, and
/// mobile mode while it is on. The built-in Ungrouped workspace always
/// exists, is never renamed or removed, and always sits last.
#[derive(Debug)]
pub struct WorkspaceBook {
    entries: Vec<Entry>,
    active: WorkspaceId,
    next_account_number: u64,
    next_workspace_number: u64,
    mobile: Option<MobileMode>,
}

impl WorkspaceBook {
    /// Rebuilds a book from a saved [`WorkspaceList`].
    ///
    /// An `active` naming no workspace in the list falls back to Ungrouped,
    /// and a list with no Ungrouped workspace gets an empty one appended as
    /// the last workspace — a hand-edited file can never leave the
    /// application with nowhere to show. Every restored account's number is
    /// checked against `next_account_number`, raising it above the highest
    /// one found, so a counter a hand edit left too low never mints a number
    /// already in use.
    #[must_use]
    pub fn restore(list: WorkspaceList) -> Self {
        let mut entries: Vec<Entry> = Vec::with_capacity(list.workspaces.len());
        let mut ungrouped: Option<Entry> = None;

        for workspace in list.workspaces {
            let id = workspace.id.clone();
            let name = workspace.name.clone();
            let is_expanded = workspace.is_expanded;
            let book = SessionBook::restore(workspace);
            let entry = Entry {
                id,
                name,
                is_expanded,
                book,
            };
            if entry.id.is_ungrouped() {
                ungrouped = Some(entry);
            } else {
                entries.push(entry);
            }
        }
        entries.push(ungrouped.unwrap_or_else(|| Entry {
            id: WorkspaceId::ungrouped(),
            name: "Ungrouped".to_owned(),
            is_expanded: true,
            book: SessionBook::new(),
        }));

        let active = entries
            .iter()
            .find(|entry| entry.id == list.active)
            .map_or_else(WorkspaceId::ungrouped, |entry| entry.id.clone());

        let highest_account_number = entries
            .iter()
            .flat_map(|entry| entry.book.sessions())
            .filter_map(|session| session.id().as_str().strip_prefix("session-"))
            .filter_map(|suffix| suffix.parse::<u64>().ok())
            .max()
            .unwrap_or(0);
        let next_account_number = list.next_account_number.max(highest_account_number + 1);

        Self {
            entries,
            active,
            next_account_number,
            next_workspace_number: list.next_workspace_number,
            mobile: None,
        }
    }

    /// The whole book as one value, ready to be saved.
    ///
    /// While mobile mode is on, every workspace the mode has reached is
    /// written with the layout and focused position it had before — so the
    /// file never records [`Layout::Mobile`] and a relaunch opens in the
    /// ordinary layout (Remote Access `FR.3.4`). The order is untouched by
    /// the mode, so it is always the live one; everything else about those
    /// accounts, a park made meanwhile included, is written live too.
    #[must_use]
    pub fn saved(&self) -> WorkspaceList {
        let workspaces = self
            .entries
            .iter()
            .map(|entry| {
                let mut workspace =
                    entry
                        .book
                        .workspace(entry.id.clone(), entry.name.clone(), entry.is_expanded);
                if let Some((layout, focused)) = self
                    .mobile
                    .as_ref()
                    .and_then(|mode| mode.snapshots.get(&entry.id))
                {
                    workspace.layout = *layout;
                    workspace.focused = *focused;
                }
                workspace
            })
            .collect();

        WorkspaceList {
            workspaces,
            active: self.active.clone(),
            next_account_number: self.next_account_number,
            next_workspace_number: self.next_workspace_number,
        }
    }

    /// The book of the workspace currently shown.
    ///
    /// # Panics
    ///
    /// Never: `active` always names a workspace this book holds, an
    /// invariant every constructor and mutator maintains.
    #[must_use]
    pub fn active(&self) -> &SessionBook {
        self.book(&self.active)
            .expect("active always names a workspace this book holds")
    }

    /// The book of the workspace currently shown, for the one caller still
    /// reaching directly into it: the shell's window, for a placement, focus
    /// or layout method [`WorkspaceBook`] does not yet forward itself. Later
    /// items narrow this to the specific forwarding methods they add.
    ///
    /// # Panics
    ///
    /// Never, for the same reason as [`WorkspaceBook::active`].
    pub fn active_mut(&mut self) -> &mut SessionBook {
        let active = self.active.clone();
        self.book_mut(&active)
            .expect("active always names a workspace this book holds")
    }

    /// The identifier of the workspace currently shown.
    #[must_use]
    pub fn active_id(&self) -> &WorkspaceId {
        &self.active
    }

    /// Every workspace, in sidebar order — Ungrouped always last.
    pub fn workspaces(&self) -> impl Iterator<Item = WorkspaceView<'_>> {
        self.entries.iter().map(|entry| WorkspaceView {
            id: &entry.id,
            name: &entry.name,
            is_expanded: entry.is_expanded,
            book: &entry.book,
        })
    }

    /// Every workspace's own book, mutably, in sidebar order — for the one
    /// caller that must apply the same change to every workspace's book at
    /// once, such as installing a restored account's remembered zoom before
    /// any holder is built. Identity, name and expansion are not exposed
    /// here; a caller that needs those too should read them from
    /// [`WorkspaceBook::workspaces`] first.
    pub fn workspaces_mut(&mut self) -> impl Iterator<Item = &mut SessionBook> {
        self.entries.iter_mut().map(|entry| &mut entry.book)
    }

    fn book(&self, id: &WorkspaceId) -> Option<&SessionBook> {
        self.entries
            .iter()
            .find(|entry| &entry.id == id)
            .map(|entry| &entry.book)
    }

    fn book_mut(&mut self, id: &WorkspaceId) -> Option<&mut SessionBook> {
        self.entries
            .iter_mut()
            .find(|entry| &entry.id == id)
            .map(|entry| &mut entry.book)
    }

    /// The book of whichever workspace holds `id`, or `None` if no workspace
    /// does.
    fn book_holding_mut(&mut self, id: &SessionId) -> Option<&mut SessionBook> {
        self.entries
            .iter_mut()
            .map(|entry| &mut entry.book)
            .find(|book| book.session(id).is_some())
    }

    fn mint_account_id(&mut self) -> SessionId {
        let number = self.next_account_number;
        self.next_account_number += 1;
        SessionId::new(format!("session-{number:04}"))
    }

    fn mint_workspace_id(&mut self) -> WorkspaceId {
        let number = self.next_workspace_number;
        self.next_workspace_number += 1;
        WorkspaceId::new(format!("workspace-{number:04}"))
    }

    /// Whether any workspace holds `id`.
    fn holds_account(&self, id: &SessionId) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.book.session(id).is_some())
    }

    /// Whether `name` matches another workspace's name, ignoring case —
    /// Ungrouped included, and the workspace named by `excluding` skipped, so
    /// a rename accepts a workspace's own current name in a different case
    /// (`FR.15.8`).
    fn name_taken(&self, name: &str, excluding: Option<&WorkspaceId>) -> bool {
        self.entries.iter().any(|entry| {
            Some(&entry.id) != excluding && entry.name.to_lowercase() == name.to_lowercase()
        })
    }

    /// Whether `workspace` has room for `additional` more accounts. Ungrouped
    /// has no limit; a named workspace is capped at
    /// [`NAMED_WORKSPACE_CAPACITY`].
    ///
    /// # Errors
    ///
    /// [`WorkspaceRefusal::UnknownWorkspace`] if no workspace has this id,
    /// [`WorkspaceRefusal::NoRoom`] if it does not have room.
    fn ensure_room(
        &self,
        workspace: &WorkspaceId,
        additional: usize,
    ) -> Result<(), WorkspaceRefusal> {
        if workspace.is_ungrouped() {
            return Ok(());
        }
        let book = self
            .book(workspace)
            .ok_or(WorkspaceRefusal::UnknownWorkspace)?;
        if book.sessions().len() + additional > NAMED_WORKSPACE_CAPACITY {
            Err(WorkspaceRefusal::NoRoom)
        } else {
            Ok(())
        }
    }

    /// Adds a new account created from a typed address into `workspace`,
    /// mints its identifier from the one counter shared by every workspace,
    /// and returns it. Never switches which workspace is shown (`FR.17.9`).
    ///
    /// # Errors
    ///
    /// [`WorkspaceRefusal::UnknownWorkspace`] if no workspace has this id,
    /// [`WorkspaceRefusal::NoRoom`] if `workspace` is named and already full
    /// (`FR.15.3`).
    pub fn add(
        &mut self,
        workspace: &WorkspaceId,
        display_name: &str,
        start_address: &str,
    ) -> Result<SessionId, WorkspaceRefusal> {
        self.ensure_room(workspace, 1)?;
        let id = self.mint_account_id();
        let book = self
            .book_mut(workspace)
            .ok_or(WorkspaceRefusal::UnknownWorkspace)?;
        book.add(&id, display_name, start_address);
        Ok(id)
    }

    /// Adds a new account for `preset`'s game under `account_name` into
    /// `workspace`, otherwise exactly [`WorkspaceBook::add`].
    ///
    /// # Errors
    ///
    /// Same as [`WorkspaceBook::add`].
    pub fn add_from_preset(
        &mut self,
        workspace: &WorkspaceId,
        account_name: &str,
        preset: &Preset,
    ) -> Result<SessionId, WorkspaceRefusal> {
        self.ensure_room(workspace, 1)?;
        let id = self.mint_account_id();
        let book = self
            .book_mut(workspace)
            .ok_or(WorkspaceRefusal::UnknownWorkspace)?;
        book.add_from_preset(&id, account_name, preset);
        Ok(id)
    }

    /// Park `id`'s account, wherever its workspace is. A no-op, returning
    /// [`Liveness::Live`], for an id no workspace holds.
    pub fn park(&mut self, id: &SessionId) -> Liveness {
        self.book_holding_mut(id)
            .map_or(Liveness::Live, |book| book.park(id))
    }

    /// Unpark `id`'s account, wherever its workspace is. A no-op, returning
    /// [`Liveness::Live`], for an id no workspace holds.
    pub fn unpark(&mut self, id: &SessionId) -> Liveness {
        self.book_holding_mut(id)
            .map_or(Liveness::Live, |book| book.unpark(id))
    }

    /// End `id`'s starting interval, wherever its workspace is. A no-op,
    /// returning [`Liveness::Live`], for an id no workspace holds.
    pub fn mark_started(&mut self, id: &SessionId) -> Liveness {
        self.book_holding_mut(id)
            .map_or(Liveness::Live, |book| book.mark_started(id))
    }

    /// Sets `id`'s keep-awake flag, wherever its workspace is. `false` for an
    /// id no workspace holds.
    pub fn set_keep_awake(&mut self, id: &SessionId, value: bool) -> bool {
        self.book_holding_mut(id)
            .is_some_and(|book| book.set_keep_awake(id, value))
    }

    /// Renames `id`'s account, wherever its workspace is. `false` for an id no
    /// workspace holds or a name that trims to empty.
    pub fn rename(&mut self, id: &SessionId, name: &str) -> bool {
        self.book_holding_mut(id)
            .is_some_and(|book| book.rename(id, name))
    }

    /// `id`'s own visibility in the shown workspace, or
    /// [`Visibility::OffGrid`] for an account that belongs to another
    /// workspace — that one method is how an inactive workspace keeps its own
    /// seating untouched while its views sit out of sight (`FR.18.1`). `None`
    /// for an id no workspace holds at all.
    #[must_use]
    pub fn placement(&self, id: &SessionId) -> Option<Visibility> {
        if let Some(visibility) = self.active().placement(id) {
            return Some(visibility);
        }
        self.workspaces()
            .any(|workspace| workspace.book().session(id).is_some())
            .then_some(Visibility::OffGrid)
    }

    /// Brings `id` into focus. When its workspace is not the one shown, that
    /// workspace becomes shown first and the switch is reported as
    /// `Some(Switch { from, to })`; either way the account's own workspace
    /// then focuses it exactly as [`SessionBook::focus_session`] would
    /// (`FR.16.3`). `None`, with nothing changed, for an id no workspace
    /// holds.
    pub fn focus_account(&mut self, id: &SessionId) -> Option<Switch> {
        let target = self
            .entries
            .iter()
            .find(|entry| entry.book.session(id).is_some())
            .map(|entry| entry.id.clone())?;

        let switch = (target != self.active).then(|| Switch {
            from: self.active.clone(),
            to: target.clone(),
        });
        self.active = target;
        self.apply_mobile_to_active();

        self.active_mut().focus_session(id);
        switch
    }

    /// Walks forward from the shown workspace, wrapping, to the next
    /// workspace holding at least one session, skipping the active one, and
    /// shows it. `None`, changing nothing, when no other workspace qualifies
    /// (`FR.23.2`).
    ///
    /// The landed workspace's own focused position is untouched, so it shows
    /// the page and place it was left on (`FR.18.3`), and if mobile mode is
    /// on, landing applies it exactly as [`WorkspaceBook::focus_account`]
    /// does.
    ///
    /// # Panics
    ///
    /// Never: `active` always names a workspace this book holds, an
    /// invariant every constructor and mutator maintains.
    pub fn focus_next_workspace(&mut self) -> Option<Switch> {
        let start = self
            .entries
            .iter()
            .position(|entry| entry.id == self.active)
            .expect("active always names a workspace this book holds");
        let len = self.entries.len();
        let target = (1..len)
            .map(|offset| (start + offset) % len)
            .find(|&index| !self.entries[index].book.sessions().is_empty())?;

        let from = self.active.clone();
        let to = self.entries[target].id.clone();
        self.active = to.clone();
        self.apply_mobile_to_active();

        Some(Switch { from, to })
    }

    /// Turns the shown workspace to its next page, wrapping, otherwise
    /// exactly [`SessionBook::next_page`], so the shell never reaches for
    /// [`WorkspaceBook::active_mut`] for a navigation (architecture rule 8).
    pub fn next_page(&mut self) -> bool {
        self.active_mut().next_page()
    }

    /// Turns the shown workspace to its previous page, otherwise exactly
    /// [`WorkspaceBook::next_page`].
    pub fn previous_page(&mut self) -> bool {
        self.active_mut().previous_page()
    }

    /// Steps the shown workspace's focus to its next account, wrapping,
    /// otherwise exactly [`WorkspaceBook::next_page`].
    pub fn focus_next_account(&mut self) -> bool {
        self.active_mut().focus_next()
    }

    /// Sets the shown workspace's layout (`FR.16.4`) — a layout switch acts
    /// only on what is on screen. Not the way into mobile mode: that is
    /// [`WorkspaceBook::enter_mobile_mode`], which also takes the snapshot
    /// leaving restores.
    pub fn set_layout(&mut self, layout: Layout) {
        self.active_mut().set_layout(layout);
    }

    /// Switches mobile mode on with the one slot shaped as `viewport`: the
    /// shown workspace's arrangement is snapshotted, it is switched to
    /// [`Layout::Mobile`] through the ordinary placement pass, and the
    /// account that held its focused slot is brought into the one slot; every
    /// other account goes off-grid, still running (Remote Access `FR.3.1`,
    /// `FR.3.2`). Already on, only the viewport changes — the snapshot from
    /// the first entry is kept, since it is the arrangement leaving must put
    /// back.
    pub fn enter_mobile_mode(&mut self, viewport: Viewport) {
        if let Some(mode) = self.mobile.as_mut() {
            mode.viewport = viewport;
            return;
        }
        self.mobile = Some(MobileMode {
            viewport,
            snapshots: HashMap::new(),
        });
        self.apply_mobile_to_active();
    }

    /// Switches mobile mode off: every workspace the mode reached gets the
    /// layout and focused position it had back, the focused position clamped
    /// the same way every mutation clamps it in case an account removed
    /// meanwhile made it invalid (Remote Access `FR.3.5`). The order was
    /// never touched by the mode, so nothing needs to be reseated. A no-op
    /// while the mode is off.
    pub fn leave_mobile_mode(&mut self) {
        let Some(mode) = self.mobile.take() else {
            return;
        };
        for (workspace, (layout, focused)) in &mode.snapshots {
            if let Some(book) = self.book_mut(workspace) {
                book.set_layout(*layout);
                book.restore_focus(*focused);
            }
        }
    }

    /// Whether mobile mode is on.
    #[must_use]
    pub fn is_mobile_mode(&self) -> bool {
        self.mobile.is_some()
    }

    /// The viewport the one mobile slot is shaped as, or `None` while the
    /// mode is off.
    #[must_use]
    pub fn mobile_viewport(&self) -> Option<Viewport> {
        self.mobile.as_ref().map(|mode| mode.viewport)
    }

    /// Reshapes the mobile slot to `viewport` — an attached phone reporting
    /// its own screen (Remote Access `FR.3.2`). A no-op while the mode is
    /// off: a viewport with no mode to apply it to is not a state this book
    /// holds.
    pub fn set_mobile_viewport(&mut self, viewport: Viewport) {
        if let Some(mode) = self.mobile.as_mut() {
            mode.viewport = viewport;
        }
    }

    /// Puts the shown workspace into [`Layout::Mobile`] if the mode is on and
    /// it has not been reached yet: its layout and focused position are
    /// snapshotted first, so leaving can put them back, then the layout
    /// switches. The page a layout switch shows is derived from the
    /// unchanged focused position, so the account that was focused is what
    /// the one mobile slot now shows — no separate focus step is needed. The
    /// one place a workspace crosses into the mode — entering and switching
    /// workspace both land here.
    fn apply_mobile_to_active(&mut self) {
        let active = self.active.clone();
        let Some(mode) = self.mobile.as_mut() else {
            return;
        };
        if mode.snapshots.contains_key(&active) {
            return;
        }
        let Some(book) = self
            .entries
            .iter_mut()
            .find(|entry| entry.id == active)
            .map(|entry| &mut entry.book)
        else {
            return;
        };

        mode.snapshots
            .insert(active, (book.layout(), book.focused_position()));
        book.set_layout(Layout::Mobile);
    }

    /// Sets whether `workspace`'s sidebar heading is expanded, carried into
    /// [`WorkspaceBook::saved`] so it survives a relaunch (`FR.16.2`). A
    /// no-op for an id no workspace holds.
    pub fn set_expanded(&mut self, workspace: &WorkspaceId, expanded: bool) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| &entry.id == workspace) {
            entry.is_expanded = expanded;
        }
    }

    /// Whether `name`, trimmed, matches another workspace's name ignoring
    /// case — `Ungrouped` included, and the workspace named by `excluding`
    /// skipped so a rename accepts its own current name in a different case
    /// (`FR.15.8`). The rule `create_workspace` and `rename_workspace` apply
    /// themselves, exposed so a name field can validate as a person types
    /// without attempting either.
    #[must_use]
    pub fn name_conflict(&self, name: &str, excluding: Option<&WorkspaceId>) -> bool {
        self.name_taken(name, excluding)
    }

    /// The named workspaces with room for `count` more accounts, in sidebar
    /// order, Ungrouped always last since it always has room, and whether a
    /// brand new named workspace could hold `count` (`FR.17.2`, `FR.17.5`).
    /// The single source the move menu and the add-game field both ask, so
    /// neither can offer a destination this book would refuse.
    #[must_use]
    pub fn destinations(&self, count: usize) -> Destinations {
        let has_room = |entry: &&Entry| {
            entry.id.is_ungrouped()
                || entry.book.sessions().len() + count <= NAMED_WORKSPACE_CAPACITY
        };
        let workspaces = self
            .entries
            .iter()
            .filter(has_room)
            .map(|entry| DestinationWorkspace {
                id: entry.id.clone(),
                name: entry.name.clone(),
            })
            .collect();

        Destinations {
            workspaces,
            can_create: count <= NAMED_WORKSPACE_CAPACITY,
        }
    }

    /// Moves every account in `ids` to `to`, all or nothing on room for
    /// whichever of them are not already there — accounts already in `to`
    /// are skipped and cost no room (`FR.17.8`). Each moved account joins the
    /// bottom of `to`'s list, seated in a free slot if one is free and
    /// off-grid otherwise, never displacing anyone (`FR.17.3`). Never changes
    /// which workspace is shown.
    ///
    /// # Errors
    ///
    /// [`WorkspaceRefusal::UnknownWorkspace`] if no workspace has id `to`;
    /// [`WorkspaceRefusal::UnknownAccount`] if any id in `ids` is not held by
    /// any workspace; [`WorkspaceRefusal::NoRoom`] if `to` cannot hold every
    /// arriving account — in either failure case nothing changes.
    ///
    /// # Panics
    ///
    /// Never: every lookup of `to` below runs only after the existence check
    /// above has already returned `Ok`.
    pub fn move_accounts(
        &mut self,
        ids: &[SessionId],
        to: &WorkspaceId,
    ) -> Result<(), WorkspaceRefusal> {
        if !self.entries.iter().any(|entry| &entry.id == to) {
            return Err(WorkspaceRefusal::UnknownWorkspace);
        }
        for id in ids {
            if !self.holds_account(id) {
                return Err(WorkspaceRefusal::UnknownAccount);
            }
        }

        let already_there = self
            .book(to)
            .expect("checked above")
            .sessions()
            .iter()
            .filter(|session| ids.contains(session.id()))
            .count();
        self.ensure_room(to, ids.len() - already_there)?;

        for id in ids {
            if self.book(to).expect("checked above").session(id).is_some() {
                continue;
            }
            let Some(session) = self.book_holding_mut(id).and_then(|book| book.take(id)) else {
                continue;
            };
            self.book_mut(to).expect("checked above").adopt(session);
        }
        Ok(())
    }

    /// Creates a named workspace holding exactly `ids`, inserted before
    /// Ungrouped, expanded, and never changes which workspace is shown.
    ///
    /// # Errors
    ///
    /// [`WorkspaceRefusal::EmptyName`] if `name` trims to nothing;
    /// [`WorkspaceRefusal::NameTaken`] if it matches another workspace's,
    /// ignoring case, Ungrouped included (`FR.15.8`);
    /// [`WorkspaceRefusal::NoRoom`] if `ids` holds more than the named
    /// capacity; [`WorkspaceRefusal::UnknownAccount`] if any id in `ids` is
    /// not held by any workspace. Nothing changes on any refusal.
    ///
    /// # Panics
    ///
    /// Never: `id` is looked up only after this call has just inserted the
    /// entry that carries it.
    pub fn create_workspace(
        &mut self,
        name: &str,
        ids: &[SessionId],
    ) -> Result<WorkspaceId, WorkspaceRefusal> {
        let trimmed = workspace_name(name).ok_or(WorkspaceRefusal::EmptyName)?;
        if self.name_taken(&trimmed, None) {
            return Err(WorkspaceRefusal::NameTaken);
        }
        if ids.len() > NAMED_WORKSPACE_CAPACITY {
            return Err(WorkspaceRefusal::NoRoom);
        }
        for id in ids {
            if !self.holds_account(id) {
                return Err(WorkspaceRefusal::UnknownAccount);
            }
        }

        let id = self.mint_workspace_id();
        // Ungrouped is always last, so inserting one position before the end
        // keeps it there.
        let insert_at = self.entries.len() - 1;
        self.entries.insert(
            insert_at,
            Entry {
                id: id.clone(),
                name: trimmed,
                is_expanded: true,
                book: SessionBook::new(),
            },
        );

        for account in ids {
            if let Some(session) = self
                .book_holding_mut(account)
                .and_then(|book| book.take(account))
            {
                self.book_mut(&id)
                    .expect("just inserted above")
                    .adopt(session);
            }
        }

        Ok(id)
    }

    /// Renames workspace `id` to `name`.
    ///
    /// # Errors
    ///
    /// [`WorkspaceRefusal::Ungrouped`] for the built-in workspace, which is
    /// never renamed; [`WorkspaceRefusal::EmptyName`] if `name` trims to
    /// nothing; [`WorkspaceRefusal::NameTaken`] if it matches *another*
    /// workspace's, ignoring case — the workspace's own current name, in any
    /// case, is accepted; [`WorkspaceRefusal::UnknownWorkspace`] if no
    /// workspace has this id.
    pub fn rename_workspace(
        &mut self,
        id: &WorkspaceId,
        name: &str,
    ) -> Result<(), WorkspaceRefusal> {
        if id.is_ungrouped() {
            return Err(WorkspaceRefusal::Ungrouped);
        }
        let trimmed = workspace_name(name).ok_or(WorkspaceRefusal::EmptyName)?;
        if self.name_taken(&trimmed, Some(id)) {
            return Err(WorkspaceRefusal::NameTaken);
        }
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| &entry.id == id)
            .ok_or(WorkspaceRefusal::UnknownWorkspace)?;
        entry.name = trimmed;
        Ok(())
    }

    /// Removes workspace `id`, adopting every account it held into the
    /// bottom of Ungrouped's list, in order. Shows Ungrouped if `id` was
    /// shown (`FR.15.4`, `FR.15.11`). Touches nothing on disk.
    ///
    /// # Errors
    ///
    /// [`WorkspaceRefusal::Ungrouped`] for the built-in workspace, which is
    /// never removed; [`WorkspaceRefusal::UnknownWorkspace`] if no workspace
    /// has this id.
    ///
    /// # Panics
    ///
    /// Never: the id just refused above is Ungrouped's own, which
    /// [`WorkspaceBook::restore`] guarantees this book always holds.
    pub fn remove_workspace(&mut self, id: &WorkspaceId) -> Result<(), WorkspaceRefusal> {
        if id.is_ungrouped() {
            return Err(WorkspaceRefusal::Ungrouped);
        }
        let index = self
            .entries
            .iter()
            .position(|entry| &entry.id == id)
            .ok_or(WorkspaceRefusal::UnknownWorkspace)?;
        let mut removed = self.entries.remove(index);

        let account_ids: Vec<SessionId> = removed
            .book
            .sessions()
            .iter()
            .map(|session| session.id().clone())
            .collect();
        let ungrouped = self
            .book_mut(&WorkspaceId::ungrouped())
            .expect("ungrouped always exists");
        for account in account_ids {
            if let Some(session) = removed.book.take(&account) {
                ungrouped.adopt(session);
            }
        }

        if let Some(mode) = self.mobile.as_mut() {
            mode.snapshots.remove(id);
        }
        if &self.active == id {
            self.active = WorkspaceId::ungrouped();
            self.apply_mobile_to_active();
        }

        Ok(())
    }

    /// Forgets `id` entirely: takes it from its own book and drops it. The
    /// one call that forgets an account, made only after its folder is gone
    /// (`FR.21.5`); `next_account_number` is never touched, so a deleted
    /// account's number is never handed to a new one (`FR.21.7`). `false` for
    /// an id no workspace holds.
    pub fn remove_account(&mut self, id: &SessionId) -> bool {
        self.book_holding_mut(id)
            .and_then(|book| book.take(id))
            .is_some()
    }

    /// Steps `id`'s size one step larger, for the **shown** workspace's
    /// current layout only (`FR.11.7`): an account elsewhere is never
    /// visible, so a gesture cannot mean resizing it. `None` if `id` is not
    /// in the shown workspace.
    pub fn zoom_in(&mut self, id: &SessionId) -> Option<crate::preset::ZoomLevel> {
        self.active_mut().zoom_in(id)
    }

    /// Steps `id`'s size one step smaller, otherwise exactly
    /// [`WorkspaceBook::zoom_in`].
    pub fn zoom_out(&mut self, id: &SessionId) -> Option<crate::preset::ZoomLevel> {
        self.active_mut().zoom_out(id)
    }

    /// Drops `id`'s remembered size for the shown workspace's current layout,
    /// otherwise exactly [`WorkspaceBook::zoom_in`].
    pub fn reset_zoom(&mut self, id: &SessionId) -> Option<crate::preset::ZoomLevel> {
        self.active_mut().reset_zoom(id)
    }

    /// The shown workspace's queued accounts first, then every other
    /// workspace's, in sidebar order (`FR.18.4`) — the order restored
    /// accounts are brought back in.
    #[must_use]
    pub fn start_order(&self) -> Vec<SessionId> {
        let mut order = self.active().start_order();
        for entry in &self.entries {
            if entry.id != self.active {
                order.extend(entry.book.start_order());
            }
        }
        order
    }

    /// How many accounts are running right now, summed over every workspace
    /// (`FR.18.2`) — every running account costs memory whichever workspace is
    /// shown.
    #[must_use]
    pub fn live_session_count(&self) -> usize {
        self.entries
            .iter()
            .map(|entry| entry.book.live_session_count())
            .sum()
    }
}

impl Default for WorkspaceBook {
    /// A book holding nothing but an empty, active, expanded Ungrouped
    /// workspace — a fresh install's starting point, and the placeholder a
    /// `#[derive(Default)]` template needs before
    /// [`WorkspaceBook::restore`] replaces it with whatever was actually
    /// saved.
    fn default() -> Self {
        Self::restore(WorkspaceList {
            workspaces: vec![Workspace::default()],
            active: WorkspaceId::ungrouped(),
            next_account_number: 1,
            next_workspace_number: 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::SlotId;
    use crate::preset::ZoomLevel;
    use crate::remote::DEFAULT_MOBILE_VIEWPORT;
    use crate::session::Visibility;
    use crate::workspace::{Account, SavedLiveness, Workspace};

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

    fn named_workspace(id: &str, name: &str, accounts: Vec<Account>) -> Workspace {
        Workspace {
            id: WorkspaceId::new(id),
            name: name.to_owned(),
            accounts,
            ..Workspace::default()
        }
    }

    fn a_list(workspaces: Vec<Workspace>, active: &str) -> WorkspaceList {
        WorkspaceList {
            workspaces,
            active: WorkspaceId::new(active),
            next_account_number: 1,
            next_workspace_number: 1,
        }
    }

    #[test]
    fn a_restored_and_saved_workspace_list_reproduces_every_field() {
        let party = Workspace {
            focused: 0,
            is_expanded: false,
            layout: Layout::SideBySide,
            ..named_workspace(
                "workspace-0001",
                "Party",
                vec![saved_account("session-0001", "A")],
            )
        };
        let ungrouped = Workspace {
            accounts: vec![saved_account("session-0002", "B")],
            ..Workspace::default()
        };
        let list = WorkspaceList {
            workspaces: vec![party.clone(), ungrouped.clone()],
            active: WorkspaceId::new("workspace-0001"),
            next_account_number: 3,
            next_workspace_number: 2,
        };

        let book = WorkspaceBook::restore(list.clone());

        assert_eq!(book.saved(), list);
    }

    #[test]
    fn an_active_naming_no_workspace_falls_back_to_ungrouped() {
        let list = a_list(vec![Workspace::default()], "workspace-9999");

        let book = WorkspaceBook::restore(list);

        assert_eq!(book.active_id(), &WorkspaceId::ungrouped());
    }

    #[test]
    fn a_list_with_no_ungrouped_workspace_gets_one_appended_last() {
        let list = a_list(
            vec![named_workspace("workspace-0001", "Party", Vec::new())],
            "workspace-0001",
        );

        let book = WorkspaceBook::restore(list);

        let ids: Vec<&WorkspaceId> = book.workspaces().map(|w| w.id()).collect();
        assert_eq!(ids.last(), Some(&&WorkspaceId::ungrouped()));
    }

    #[test]
    fn minting_reads_the_counter_when_it_sits_above_every_account() {
        let list = a_list(
            vec![Workspace {
                accounts: vec![saved_account("session-0001", "A")],
                ..Workspace::default()
            }],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(WorkspaceList {
            next_account_number: 9,
            ..list
        });

        let id = book
            .add(&WorkspaceId::ungrouped(), "New", "https://example.test/new")
            .expect("ungrouped always has room");

        assert_eq!(id.as_str(), "session-0009");
    }

    #[test]
    fn a_counter_left_below_a_restored_accounts_number_is_raised_above_it() {
        let list = a_list(
            vec![Workspace {
                accounts: vec![saved_account("session-0007", "A")],
                ..Workspace::default()
            }],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(WorkspaceList {
            next_account_number: 1,
            ..list
        });

        let id = book
            .add(&WorkspaceId::ungrouped(), "New", "https://example.test/new")
            .expect("ungrouped always has room");

        assert_eq!(id.as_str(), "session-0008");
    }

    #[test]
    fn adding_into_a_full_named_workspace_is_refused_and_changes_nothing() {
        let accounts = (1..=NAMED_WORKSPACE_CAPACITY)
            .map(|n| saved_account(&format!("session-{n:04}"), "A"))
            .collect();
        let list = a_list(
            vec![named_workspace("workspace-0001", "Party", accounts)],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(list);
        let before = book.saved();

        let result = book.add(
            &WorkspaceId::new("workspace-0001"),
            "New",
            "https://example.test/new",
        );

        assert_eq!(
            (result, book.saved()),
            (Err(WorkspaceRefusal::NoRoom), before)
        );
    }

    #[test]
    fn adding_into_a_workspace_that_is_not_shown_leaves_active_unchanged() {
        let list = a_list(
            vec![named_workspace("workspace-0001", "Party", Vec::new())],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(list);

        book.add(
            &WorkspaceId::new("workspace-0001"),
            "New",
            "https://example.test/new",
        )
        .expect("party has room");

        assert_eq!(book.active_id(), &WorkspaceId::ungrouped());
    }

    #[test]
    fn start_order_lists_the_shown_workspaces_queued_accounts_before_every_others() {
        let party = named_workspace(
            "workspace-0001",
            "Party",
            vec![saved_account("session-0001", "A")],
        );
        let ungrouped = Workspace {
            accounts: vec![saved_account("session-0002", "B")],
            ..Workspace::default()
        };
        let list = a_list(vec![party, ungrouped], "ungrouped");
        let book = WorkspaceBook::restore(list);

        let order = book.start_order();

        let ids: Vec<&str> = order.iter().map(SessionId::as_str).collect();
        assert_eq!(ids, vec!["session-0002", "session-0001"]);
    }

    #[test]
    fn live_session_count_sums_every_workspace() {
        let list = a_list(
            vec![named_workspace("workspace-0001", "Party", Vec::new())],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(list);

        book.add(
            &WorkspaceId::new("workspace-0001"),
            "A",
            "https://example.test/a",
        )
        .expect("party has room");
        book.add(&WorkspaceId::ungrouped(), "B", "https://example.test/b")
            .expect("ungrouped has room");

        assert_eq!(book.live_session_count(), 2);
    }

    #[test]
    fn remembered_zoom_survives_a_restore_via_the_book() {
        let mut book = WorkspaceBook::restore(WorkspaceList {
            workspaces: vec![Workspace::default()],
            active: WorkspaceId::ungrouped(),
            next_account_number: 1,
            next_workspace_number: 1,
        });
        let id = book
            .add(&WorkspaceId::ungrouped(), "One", "https://example.test/one")
            .expect("ungrouped has room");

        let stepped = book
            .zoom_in(&id)
            .expect("the account is in the shown workspace");

        assert!(stepped.multiplier() > 1.0);
    }

    fn a_book_with_party_and_ungrouped() -> (WorkspaceBook, SessionId, SessionId) {
        let list = a_list(
            vec![named_workspace("workspace-0001", "Party", Vec::new())],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(list);
        let party_member = book
            .add(
                &WorkspaceId::new("workspace-0001"),
                "A",
                "https://example.test/a",
            )
            .expect("party has room");
        let ungrouped_member = book
            .add(&WorkspaceId::ungrouped(), "B", "https://example.test/b")
            .expect("ungrouped has room");
        (book, party_member, ungrouped_member)
    }

    #[test]
    fn placement_reports_off_grid_for_an_account_in_another_workspace() {
        let (book, party_member, ungrouped_member) = a_book_with_party_and_ungrouped();

        assert_eq!(
            (
                book.placement(&party_member),
                book.placement(&ungrouped_member)
            ),
            (
                Some(Visibility::OffGrid),
                Some(Visibility::InSlot(SlotId::FIRST)),
            ),
        );
    }

    #[test]
    fn placement_is_none_for_an_id_no_workspace_holds() {
        let (book, ..) = a_book_with_party_and_ungrouped();

        assert_eq!(book.placement(&SessionId::new("session-9999")), None);
    }

    #[test]
    fn focus_account_on_another_workspace_switches_and_names_both_workspaces() {
        let (mut book, party_member, _) = a_book_with_party_and_ungrouped();

        let switch = book
            .focus_account(&party_member)
            .expect("the account is in a workspace that is not shown");

        assert_eq!(
            (switch.from(), switch.to(), book.active_id()),
            (
                &WorkspaceId::ungrouped(),
                &WorkspaceId::new("workspace-0001"),
                &WorkspaceId::new("workspace-0001"),
            ),
        );
    }

    #[test]
    fn focus_account_puts_the_account_in_the_new_workspaces_focused_place() {
        let (mut book, party_member, _) = a_book_with_party_and_ungrouped();

        book.focus_account(&party_member);

        assert_eq!(
            book.placement(&party_member),
            Some(Visibility::InSlot(SlotId::FIRST)),
        );
    }

    #[test]
    fn focus_account_leaves_the_outgoing_workspaces_seating_and_focus_untouched() {
        let (mut book, party_member, ungrouped_member) = a_book_with_party_and_ungrouped();
        let ungrouped_before = book
            .book(&WorkspaceId::ungrouped())
            .expect("ungrouped always exists")
            .workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true);

        book.focus_account(&party_member);

        // Ungrouped is no longer shown, so `placement` now reports the member
        // off-grid — the switch changing which workspace's seating is on
        // screen, never the seating itself, which the `Workspace` comparison
        // below proves directly.
        let ungrouped_after = book
            .book(&WorkspaceId::ungrouped())
            .expect("ungrouped always exists")
            .workspace(WorkspaceId::ungrouped(), "Ungrouped".to_owned(), true);
        assert_eq!(
            (ungrouped_after, book.placement(&ungrouped_member)),
            (ungrouped_before, Some(Visibility::OffGrid)),
        );
    }

    #[test]
    fn focus_account_already_in_the_shown_workspace_reports_no_switch() {
        let (mut book, _, ungrouped_member) = a_book_with_party_and_ungrouped();

        let switch = book.focus_account(&ungrouped_member);

        assert_eq!(switch, None);
    }

    #[test]
    fn focus_account_on_an_unknown_id_changes_nothing() {
        let (mut book, ..) = a_book_with_party_and_ungrouped();
        let before = book.saved();

        let switch = book.focus_account(&SessionId::new("session-9999"));

        assert_eq!((switch, book.saved()), (None, before));
    }

    /// Acceptance: `focus_next_workspace` from `Ungrouped` over `[Ungrouped:
    /// A]`, `[W1: —]`, `[W2: B]` lands on `W2`, and from `W2` wraps to
    /// `Ungrouped`.
    #[test]
    fn focus_next_workspace_walks_forward_skipping_an_empty_workspace_and_wraps() {
        let list = a_list(
            vec![
                named_workspace("workspace-0001", "W1", Vec::new()),
                named_workspace(
                    "workspace-0002",
                    "W2",
                    vec![saved_account("session-0002", "B")],
                ),
                Workspace {
                    accounts: vec![saved_account("session-0001", "A")],
                    ..Workspace::default()
                },
            ],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(list);

        let to_w2 = book.focus_next_workspace();
        let active_after_first = book.active_id().clone();
        let to_ungrouped = book.focus_next_workspace();
        let active_after_second = book.active_id().clone();

        assert_eq!(
            (
                to_w2.map(|switch| switch.to().clone()),
                active_after_first,
                to_ungrouped.map(|switch| switch.to().clone()),
                active_after_second,
            ),
            (
                Some(WorkspaceId::new("workspace-0002")),
                WorkspaceId::new("workspace-0002"),
                Some(WorkspaceId::ungrouped()),
                WorkspaceId::ungrouped(),
            ),
        );
    }

    /// Acceptance: `focus_next_workspace` with a single non-empty workspace
    /// returns `None` and leaves `active_id()` unchanged.
    #[test]
    fn focus_next_workspace_with_no_other_workspace_to_land_on_returns_none_and_leaves_active_unchanged()
     {
        let list = a_list(
            vec![Workspace {
                accounts: vec![saved_account("session-0001", "A")],
                ..Workspace::default()
            }],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(list);

        let switch = book.focus_next_workspace();

        assert_eq!(
            (switch, book.active_id()),
            (None, &WorkspaceId::ungrouped()),
        );
    }

    /// Acceptance: after `move_accounts` puts one account into a workspace
    /// the walk skipped, `focus_next_workspace` lands on it.
    #[test]
    fn focus_next_workspace_lands_on_a_workspace_once_an_account_moves_into_it() {
        let list = a_list(
            vec![named_workspace("workspace-0001", "W1", Vec::new())],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(list);
        let member = book
            .add(&WorkspaceId::ungrouped(), "A", "https://example.test/a")
            .expect("ungrouped has room");

        let skipped = book.focus_next_workspace();

        book.move_accounts(&[member], &WorkspaceId::new("workspace-0001"))
            .expect("w1 has room");
        let landed = book.focus_next_workspace();

        assert_eq!(
            (skipped, landed.map(|switch| switch.to().clone())),
            (None, Some(WorkspaceId::new("workspace-0001"))),
        );
    }

    /// Acceptance: landing on a workspace whose focused position is `3`
    /// shows that position — its `focused_session()` is the same account as
    /// before the switch away.
    #[test]
    fn focus_next_workspace_lands_showing_the_same_focused_account_as_before_the_switch_away() {
        let ids: Vec<SessionId> = (1..=4)
            .map(|n| SessionId::new(format!("session-000{n}")))
            .collect();
        let party = Workspace {
            layout: Layout::Grid,
            focused: 3,
            ..named_workspace(
                "workspace-0001",
                "Party",
                ids.iter()
                    .map(|id| saved_account(id.as_str(), id.as_str()))
                    .collect(),
            )
        };
        let ungrouped = Workspace {
            accounts: vec![saved_account("session-0005", "Farm")],
            ..Workspace::default()
        };
        let mut book = WorkspaceBook::restore(WorkspaceList {
            workspaces: vec![party, ungrouped],
            active: WorkspaceId::ungrouped(),
            next_account_number: 6,
            next_workspace_number: 2,
        });
        let focus_before_switch_away = book
            .book(&WorkspaceId::new("workspace-0001"))
            .expect("party exists")
            .focused_session()
            .map(|session| session.id().clone());

        book.focus_next_workspace();

        assert_eq!(
            book.active()
                .focused_session()
                .map(|session| session.id().clone()),
            focus_before_switch_away,
        );
    }

    /// Acceptance: in mobile mode, landing applies `Layout::Mobile` to the
    /// landed workspace and `leave_mobile_mode` afterwards restores its
    /// layout.
    #[test]
    fn focus_next_workspace_applies_mobile_mode_to_the_landed_workspace_and_leaving_restores_it() {
        let (mut book, _) = a_seated_grid_book();
        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);

        let switch = book.focus_next_workspace();
        let layout_while_mobile = book.active().layout();

        book.leave_mobile_mode();
        let layout_after_leaving = book
            .book(&WorkspaceId::ungrouped())
            .expect("ungrouped always exists")
            .layout();

        assert_eq!(
            (
                switch.map(|switch| switch.to().clone()),
                layout_while_mobile,
                layout_after_leaving,
            ),
            (
                Some(WorkspaceId::ungrouped()),
                Layout::Mobile,
                Layout::default()
            ),
        );
    }

    #[test]
    fn set_layout_changes_only_the_shown_workspaces_layout() {
        let (mut book, ..) = a_book_with_party_and_ungrouped();

        book.set_layout(Layout::Grid);

        assert_eq!(
            (
                book.active().layout(),
                book.book(&WorkspaceId::new("workspace-0001"))
                    .expect("party exists")
                    .layout(),
            ),
            (Layout::Grid, Layout::default()),
        );
    }

    #[test]
    fn set_expanded_is_carried_into_saved() {
        let (mut book, ..) = a_book_with_party_and_ungrouped();

        book.set_expanded(&WorkspaceId::new("workspace-0001"), false);

        let party = book
            .saved()
            .workspaces
            .into_iter()
            .find(|workspace| workspace.id == WorkspaceId::new("workspace-0001"))
            .expect("party is in the saved list");
        assert!(!party.is_expanded);
    }

    fn full_named_workspace(id: &str, name: &str) -> Workspace {
        let accounts = (1..=NAMED_WORKSPACE_CAPACITY)
            .map(|n| saved_account(&format!("session-{n:04}"), &format!("Member{n}")))
            .collect();
        named_workspace(id, name, accounts)
    }

    #[test]
    fn destinations_lists_named_workspaces_with_room_then_ungrouped_last() {
        let list = a_list(
            vec![
                named_workspace(
                    "workspace-0001",
                    "Party",
                    vec![saved_account("session-0001", "A")],
                ),
                full_named_workspace("workspace-0002", "Duo"),
            ],
            "ungrouped",
        );
        let book = WorkspaceBook::restore(list);

        let destinations = book.destinations(2);

        let ids: Vec<&WorkspaceId> = destinations
            .workspaces()
            .iter()
            .map(DestinationWorkspace::id)
            .collect();
        assert_eq!(
            ids,
            vec![
                &WorkspaceId::new("workspace-0001"),
                &WorkspaceId::ungrouped()
            ],
        );
    }

    #[test]
    fn destinations_for_a_set_larger_than_capacity_offers_no_named_workspace_and_cannot_create() {
        let list = a_list(
            vec![named_workspace("workspace-0001", "Party", Vec::new())],
            "ungrouped",
        );
        let book = WorkspaceBook::restore(list);

        let destinations = book.destinations(5);

        assert_eq!(
            (
                destinations.can_create(),
                destinations
                    .workspaces()
                    .iter()
                    .map(DestinationWorkspace::id)
                    .collect::<Vec<_>>(),
            ),
            (false, vec![&WorkspaceId::ungrouped()]),
        );
    }

    #[test]
    fn move_accounts_to_a_workspace_without_room_for_the_whole_set_is_refused_and_changes_nothing()
    {
        let list = a_list(
            vec![full_named_workspace("workspace-0001", "Full")],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(list);
        let extra = book
            .add(
                &WorkspaceId::ungrouped(),
                "Extra",
                "https://example.test/extra",
            )
            .expect("ungrouped has room");
        let before = book.saved();

        let result = book.move_accounts(&[extra], &WorkspaceId::new("workspace-0001"));

        assert_eq!(
            (result, book.saved()),
            (Err(WorkspaceRefusal::NoRoom), before)
        );
    }

    #[test]
    fn moving_an_account_out_of_the_shown_workspace_leaves_active_unchanged_and_skips_ids_already_there()
     {
        let (mut book, party_member, ungrouped_member) = a_book_with_party_and_ungrouped();
        let before_party = book
            .book(&WorkspaceId::new("workspace-0001"))
            .expect("party exists")
            .workspace(WorkspaceId::new("workspace-0001"), "Party".to_owned(), true);

        book.move_accounts(
            &[ungrouped_member.clone(), party_member.clone()],
            &WorkspaceId::new("workspace-0001"),
        )
        .expect("ungrouped and party both have room");

        let after_party = book
            .book(&WorkspaceId::new("workspace-0001"))
            .expect("party exists")
            .workspace(WorkspaceId::new("workspace-0001"), "Party".to_owned(), true);
        assert_eq!(
            (
                book.active_id(),
                book.placement(&party_member),
                after_party.accounts.len(),
                before_party.accounts.len(),
            ),
            (&WorkspaceId::ungrouped(), Some(Visibility::OffGrid), 2, 1),
        );
    }

    #[test]
    fn create_workspace_refuses_an_empty_name_a_taken_name_and_an_oversized_set() {
        let list = a_list(
            vec![named_workspace("workspace-0001", "Party", Vec::new())],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(list);
        let five: Vec<SessionId> = (1..=5)
            .map(|n| {
                book.add(
                    &WorkspaceId::ungrouped(),
                    &format!("M{n}"),
                    "https://example.test/m",
                )
                .expect("ungrouped has room")
            })
            .collect();
        let before = book.saved();

        let empty = book.create_workspace("   ", &[]);
        let taken_lower = book.create_workspace("party", &[]);
        let taken_ungrouped = book.create_workspace("ungrouped", &[]);
        let oversized = book.create_workspace("Five", &five);

        assert_eq!(
            (empty, taken_lower, taken_ungrouped, oversized, book.saved()),
            (
                Err(WorkspaceRefusal::EmptyName),
                Err(WorkspaceRefusal::NameTaken),
                Err(WorkspaceRefusal::NameTaken),
                Err(WorkspaceRefusal::NoRoom),
                before,
            ),
        );
    }

    #[test]
    fn create_workspace_inserts_before_ungrouped_expanded_and_never_reuses_a_minted_id() {
        let list = a_list(Vec::new(), "ungrouped");
        let mut book = WorkspaceBook::restore(list);
        let member = book
            .add(&WorkspaceId::ungrouped(), "A", "https://example.test/a")
            .expect("ungrouped has room");

        let id = book
            .create_workspace("Party", std::slice::from_ref(&member))
            .expect("a unique name with one account");

        let ids: Vec<&WorkspaceId> = book.workspaces().map(|w| w.id()).collect();
        let party =
            book.book(&id)
                .expect("just created")
                .workspace(id.clone(), "Party".to_owned(), true);

        let restored = WorkspaceBook::restore(book.saved());
        let mut restored = restored;
        let second = restored
            .create_workspace("Another", &[])
            .expect("a second unique name");

        assert_eq!(
            (
                ids,
                party.is_expanded,
                party.accounts.into_iter().map(|a| a.id).collect::<Vec<_>>(),
                id == second,
            ),
            (
                vec![&id, &WorkspaceId::ungrouped()],
                true,
                vec![member],
                false,
            ),
        );
    }

    #[test]
    fn rename_workspace_refuses_ungrouped_and_another_names_case_insensitively() {
        let list = a_list(
            vec![
                named_workspace("workspace-0001", "Party", Vec::new()),
                named_workspace("workspace-0002", "Duo", Vec::new()),
            ],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(list);

        let ungrouped = book.rename_workspace(&WorkspaceId::ungrouped(), "New name");
        let taken = book.rename_workspace(&WorkspaceId::new("workspace-0002"), "party");
        let own_name_recased = book.rename_workspace(&WorkspaceId::new("workspace-0001"), "PARTY");
        let renamed = book.rename_workspace(&WorkspaceId::new("workspace-0002"), "Trio");

        let names: Vec<String> = book.workspaces().map(|w| w.name().to_owned()).collect();
        assert_eq!(
            (ungrouped, taken, own_name_recased, renamed, names),
            (
                Err(WorkspaceRefusal::Ungrouped),
                Err(WorkspaceRefusal::NameTaken),
                Ok(()),
                Ok(()),
                vec![
                    "PARTY".to_owned(),
                    "Trio".to_owned(),
                    "Ungrouped".to_owned()
                ],
            ),
        );
    }

    #[test]
    fn remove_workspace_refuses_ungrouped_otherwise_adopts_accounts_in_order() {
        let list = a_list(
            vec![named_workspace(
                "workspace-0001",
                "Party",
                vec![
                    saved_account("session-0001", "A"),
                    saved_account("session-0002", "B"),
                ],
            )],
            "ungrouped",
        );
        let mut book = WorkspaceBook::restore(list);

        let refused = book.remove_workspace(&WorkspaceId::ungrouped());
        book.remove_workspace(&WorkspaceId::new("workspace-0001"))
            .expect("party can be removed");

        let ungrouped_ids: Vec<String> = book
            .book(&WorkspaceId::ungrouped())
            .expect("ungrouped exists")
            .sessions()
            .iter()
            .map(|session| session.id().as_str().to_owned())
            .collect();
        assert_eq!(
            (refused, ungrouped_ids),
            (
                Err(WorkspaceRefusal::Ungrouped),
                vec!["session-0001".to_owned(), "session-0002".to_owned()],
            ),
        );
    }

    #[test]
    fn removing_the_shown_workspace_shows_ungrouped() {
        let (mut book, party_member, _) = a_book_with_party_and_ungrouped();
        book.focus_account(&party_member);
        assert_eq!(book.active_id(), &WorkspaceId::new("workspace-0001"));

        book.remove_workspace(&WorkspaceId::new("workspace-0001"))
            .expect("party can be removed");

        assert_eq!(book.active_id(), &WorkspaceId::ungrouped());
    }

    #[test]
    fn remove_account_leaves_the_place_empty_and_never_reuses_its_number_after_a_restore() {
        let mut book = WorkspaceBook::restore(a_list(Vec::new(), "ungrouped"));
        let id = book
            .add(&WorkspaceId::ungrouped(), "A", "https://example.test/a")
            .expect("ungrouped has room");

        let removed = book.remove_account(&id);
        let restored = WorkspaceBook::restore(book.saved());
        let mut restored = restored;
        let next = restored
            .add(&WorkspaceId::ungrouped(), "B", "https://example.test/b")
            .expect("ungrouped has room");

        assert_eq!((removed, next == id), (true, false));
    }

    /// A `Grid` workspace `Party` with four accounts in order, focused on the
    /// third (position 2, slot 2), shown; Ungrouped holds one account in
    /// `Single`.
    fn a_seated_grid_book() -> (WorkspaceBook, Vec<SessionId>) {
        let ids: Vec<SessionId> = (1..=4)
            .map(|n| SessionId::new(format!("session-000{n}")))
            .collect();
        let party = Workspace {
            layout: Layout::Grid,
            focused: 2,
            ..named_workspace(
                "workspace-0001",
                "Party",
                ids.iter()
                    .map(|id| saved_account(id.as_str(), id.as_str()))
                    .collect(),
            )
        };
        let ungrouped = Workspace {
            accounts: vec![saved_account("session-0005", "Farm")],
            ..Workspace::default()
        };
        let book = WorkspaceBook::restore(WorkspaceList {
            workspaces: vec![party, ungrouped],
            active: WorkspaceId::new("workspace-0001"),
            next_account_number: 6,
            next_workspace_number: 2,
        });
        (book, ids)
    }

    fn visibilities(book: &WorkspaceBook, ids: &[SessionId]) -> Vec<Option<Visibility>> {
        ids.iter().map(|id| book.placement(id)).collect()
    }

    #[test]
    fn entering_mobile_mode_leaves_the_focused_account_in_slot_zero_and_the_rest_off_grid() {
        let (mut book, ids) = a_seated_grid_book();

        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);

        assert_eq!(
            visibilities(&book, &ids),
            vec![
                Some(Visibility::OffGrid),
                Some(Visibility::OffGrid),
                Some(Visibility::InSlot(SlotId::FIRST)),
                Some(Visibility::OffGrid),
            ],
        );
    }

    #[test]
    fn entering_mobile_mode_switches_the_shown_workspace_to_the_mobile_layout_only() {
        let (mut book, _) = a_seated_grid_book();

        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);

        let layouts: Vec<Layout> = book.workspaces().map(|w| w.book().layout()).collect();
        assert_eq!(layouts, vec![Layout::Mobile, Layout::Single]);
    }

    #[test]
    fn leaving_mobile_mode_returns_every_account_to_its_previous_slot_with_the_previous_focus() {
        let (mut book, _) = a_seated_grid_book();
        let before = book.saved();

        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);
        book.leave_mobile_mode();

        assert_eq!(book.saved(), before);
    }

    #[test]
    fn focusing_another_account_while_the_mode_is_on_does_not_change_what_leaving_restores() {
        let (mut book, ids) = a_seated_grid_book();
        let before = book.saved();

        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);
        book.focus_account(&ids[3]);
        book.leave_mobile_mode();

        assert_eq!(book.saved(), before);
    }

    #[test]
    fn focusing_another_account_while_the_mode_is_on_swaps_it_into_the_one_slot() {
        let (mut book, ids) = a_seated_grid_book();
        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);

        book.focus_account(&ids[3]);

        assert_eq!(
            (book.placement(&ids[3]), book.placement(&ids[2])),
            (
                Some(Visibility::InSlot(SlotId::FIRST)),
                Some(Visibility::OffGrid)
            ),
        );
    }

    /// An account removed while the mode is on shifts every account after it
    /// forward by one position — there is no remembered slot to return an
    /// unaffected sibling to, since a slot is derived from the order alone
    /// (code standards rule 1). `leave_mobile_mode` restores the pre-mobile
    /// layout and focused position over whatever order remains.
    #[test]
    fn an_account_removed_while_the_mode_is_on_is_simply_gone_on_leave() {
        let (mut book, ids) = a_seated_grid_book();
        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);

        book.remove_account(&ids[1]);
        book.leave_mobile_mode();

        assert_eq!(
            visibilities(&book, &ids),
            vec![
                Some(Visibility::InSlot(SlotId::new(0))),
                None,
                Some(Visibility::InSlot(SlotId::new(1))),
                Some(Visibility::InSlot(SlotId::new(2))),
            ],
        );
    }

    #[test]
    fn an_account_added_while_the_mode_is_on_is_seated_by_the_placement_pass_on_leave() {
        let (mut book, ids) = a_seated_grid_book();
        book.remove_account(&ids[3]);
        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);

        let added = book
            .add(
                &WorkspaceId::new("workspace-0001"),
                "New",
                "https://example.test/new",
            )
            .expect("a slot was freed above");
        book.leave_mobile_mode();

        assert_eq!(
            visibilities(
                &book,
                &[ids[0].clone(), ids[1].clone(), ids[2].clone(), added]
            ),
            vec![
                Some(Visibility::InSlot(SlotId::new(0))),
                Some(Visibility::InSlot(SlotId::new(1))),
                Some(Visibility::InSlot(SlotId::new(2))),
                Some(Visibility::InSlot(SlotId::new(3))),
            ],
        );
    }

    #[test]
    fn an_account_added_into_a_full_grid_while_the_mode_is_on_goes_off_grid_on_leave() {
        let ids: Vec<SessionId> = (1..=4)
            .map(|n| SessionId::new(format!("session-000{n}")))
            .collect();
        let ungrouped = Workspace {
            layout: Layout::Grid,
            accounts: ids
                .iter()
                .map(|id| saved_account(id.as_str(), id.as_str()))
                .collect(),
            ..Workspace::default()
        };
        let mut book = WorkspaceBook::restore(a_list(vec![ungrouped], "ungrouped"));
        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);

        let added = book
            .add(&WorkspaceId::ungrouped(), "New", "https://example.test/new")
            .expect("ungrouped always has room");
        book.leave_mobile_mode();

        assert_eq!(
            visibilities(&book, &[ids[0].clone(), ids[3].clone(), added]),
            vec![
                Some(Visibility::InSlot(SlotId::new(0))),
                Some(Visibility::InSlot(SlotId::new(3))),
                Some(Visibility::OffGrid),
            ],
        );
    }

    #[test]
    fn saved_while_the_mode_is_on_reports_the_pre_mobile_layout_focus_and_slots() {
        let (mut book, ids) = a_seated_grid_book();
        let before = book.saved();

        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);
        book.focus_account(&ids[0]);

        assert_eq!(book.saved(), before);
    }

    #[test]
    fn a_park_while_the_mode_is_on_is_written_into_the_saved_liveness() {
        let (mut book, ids) = a_seated_grid_book();
        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);

        book.park(&ids[1]);

        let saved = book.saved();
        assert_eq!(
            saved.workspaces[0].accounts[1].liveness,
            SavedLiveness::Parked
        );
    }

    #[test]
    fn a_workspace_shown_for_the_first_time_while_the_mode_is_on_is_switched_to_mobile_too() {
        let (mut book, _) = a_seated_grid_book();
        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);

        book.focus_account(&SessionId::new("session-0005"));

        assert_eq!(book.active().layout(), Layout::Mobile);
    }

    #[test]
    fn leaving_restores_every_workspace_the_mode_reached_not_only_the_shown_one() {
        let (mut book, _) = a_seated_grid_book();
        let before = book.saved();
        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);
        book.focus_account(&SessionId::new("session-0005"));

        book.leave_mobile_mode();

        assert_eq!(
            book.saved(),
            WorkspaceList {
                active: WorkspaceId::ungrouped(),
                ..before
            }
        );
    }

    #[test]
    fn entering_again_while_the_mode_is_on_changes_the_viewport_and_keeps_the_first_snapshot() {
        let (mut book, ids) = a_seated_grid_book();
        let before = book.saved();
        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);
        book.focus_account(&ids[0]);
        let phone = Viewport {
            width: 390,
            height: 844,
        };

        book.enter_mobile_mode(phone);
        let viewport = book.mobile_viewport();
        book.leave_mobile_mode();

        assert_eq!((viewport, book.saved()), (Some(phone), before));
    }

    #[test]
    fn the_mode_and_its_viewport_are_reported_only_while_on() {
        let (mut book, _) = a_seated_grid_book();
        let off = (book.is_mobile_mode(), book.mobile_viewport());

        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);
        let on = (book.is_mobile_mode(), book.mobile_viewport());
        book.leave_mobile_mode();
        let off_again = (book.is_mobile_mode(), book.mobile_viewport());

        assert_eq!(
            (off, on, off_again),
            (
                (false, None),
                (true, Some(DEFAULT_MOBILE_VIEWPORT)),
                (false, None)
            ),
        );
    }

    #[test]
    fn set_mobile_viewport_reshapes_the_slot_while_on_and_does_nothing_while_off() {
        let (mut book, _) = a_seated_grid_book();
        let phone = Viewport {
            width: 390,
            height: 844,
        };

        book.set_mobile_viewport(phone);
        let while_off = book.mobile_viewport();
        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);
        book.set_mobile_viewport(phone);

        assert_eq!((while_off, book.mobile_viewport()), (None, Some(phone)));
    }

    #[test]
    fn removing_the_shown_workspace_while_the_mode_is_on_shows_ungrouped_in_mobile() {
        let (mut book, _) = a_seated_grid_book();
        book.enter_mobile_mode(DEFAULT_MOBILE_VIEWPORT);

        book.remove_workspace(&WorkspaceId::new("workspace-0001"))
            .expect("a named workspace can be removed");

        assert_eq!(book.active().layout(), Layout::Mobile);
    }
}
