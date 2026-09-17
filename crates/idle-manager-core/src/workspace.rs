//! The whole arrangement as one value: every account with the settings that
//! outlive a session, plus the layout in use. [`SessionBook::workspace`] hands
//! one out to be saved and [`SessionBook::restore`] rebuilds a book from one.
//!
//! Nothing here reads or writes anything — persistence is `store`'s job
//! (architecture rules 1, 7). This module only says what the saved shape is.
//!
//! [`SessionBook::workspace`]: crate::SessionBook::workspace
//! [`SessionBook::restore`]: crate::SessionBook::restore

use crate::layout::{Layout, SlotId};
use crate::preset::ZoomLevel;
use crate::session::{SessionId, Visibility, WorkspaceId};

/// Whether a saved account was running or had been parked.
///
/// The only two liveness states that describe a wish rather than a moment.
/// [`Liveness::Starting`] and [`Liveness::Queued`] are moments a session passes
/// through, so [`SessionBook::workspace`] never emits them — a starting or
/// queued account is saved as [`SavedLiveness::Running`].
///
/// [`Liveness::Starting`]: crate::Liveness::Starting
/// [`Liveness::Queued`]: crate::Liveness::Queued
/// [`SessionBook::workspace`]: crate::SessionBook::workspace
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedLiveness {
    /// The account's game was running when the workspace was taken.
    Running,
    /// The account had been parked to save memory.
    Parked,
}

/// One saved account: the identity the book minted, the two things the user
/// typed, the view settings copied from its preset at creation, and where it
/// sat and whether it was running when the workspace was taken.
///
/// Carries no reference to the preset it was created from — every field the
/// shell needs is here directly, so a preset file deleted by hand cannot break
/// a saved workspace (roadmap item 06).
#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    /// The identifier the book minted; names the account's folder on disk.
    pub id: SessionId,
    /// The name the user typed for this account.
    pub display_name: String,
    /// The address the game starts at.
    pub start_address: String,
    /// Whether the account was running or parked.
    pub liveness: SavedLiveness,
    /// Where the account sat: a visible slot, or out of sight.
    pub visibility: Visibility,
    /// The slot the account returns to when a layout that has it is chosen,
    /// even while it is out of sight. `None` for an account that has never held
    /// a slot.
    pub remembered_slot: Option<SlotId>,
    /// Whether the account keeps running at full speed while hidden.
    pub is_kept_awake: bool,
    /// The identity to present to the game, or `None` for the engine's own.
    pub browser_identity: Option<String>,
    /// How large to draw the account's page.
    pub zoom: ZoomLevel,
}

/// One workspace's whole arrangement as one value: its identity, its
/// accounts in the order they sit in, and the layout and focused place the
/// window was arranged for.
///
/// The value [`SessionBook::workspace`] produces and [`SessionBook::restore`]
/// consumes for one workspace at a time — [`crate::WorkspaceBook`] is what
/// carries a whole [`WorkspaceList`] of these. Reading one back off a book
/// restored from it reproduces it field for field, with a starting or queued
/// account reported as [`SavedLiveness::Running`]. `id`, `name` and
/// `is_expanded` pass through [`SessionBook::restore`] and
/// [`SessionBook::workspace`] rather than being carried by the book itself —
/// a `SessionBook` is the same shape whichever workspace it belongs to.
///
/// [`SessionBook::workspace`]: crate::SessionBook::workspace
/// [`SessionBook::restore`]: crate::SessionBook::restore
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace {
    /// The identifier this workspace was minted with, or
    /// [`WorkspaceId::ungrouped`] for the built-in one.
    pub id: WorkspaceId,
    /// The name shown on the workspace's sidebar heading.
    pub name: String,
    /// The place a full-grid addition to this workspace displaces, and the
    /// place it returns to when this workspace becomes the one shown
    /// (`FR.15.1`).
    pub focused: SlotId,
    /// Whether this workspace's sidebar heading is expanded. Carried here,
    /// not invented in the shell, because it must survive a relaunch
    /// (`FR.16.2`) and this is the only value the store sees.
    pub is_expanded: bool,
    /// The accounts, in the order they sit in.
    pub accounts: Vec<Account>,
    /// The layout the window was arranged for.
    pub layout: Layout,
}

impl Default for Workspace {
    /// An empty, expanded Ungrouped workspace arranged for the single-slot
    /// layout — what a fresh install's one workspace looks like, and a handy
    /// base for a test that only cares about a few fields.
    fn default() -> Self {
        Self {
            id: WorkspaceId::ungrouped(),
            name: "Ungrouped".to_owned(),
            focused: SlotId::FIRST,
            is_expanded: true,
            accounts: Vec::new(),
            layout: Layout::default(),
        }
    }
}

/// Every workspace, in sidebar order, plus which one is shown and both
/// minting counters — the value saved and restored as a whole.
///
/// [`crate::WorkspaceBook::restore`] consumes one of these and
/// [`crate::WorkspaceBook::saved`] produces one; nothing here is serialised —
/// that is `store`'s job (architecture rules 1, 7).
#[derive(Debug, Clone, PartialEq)]
pub struct WorkspaceList {
    /// Every workspace, in the order the sidebar shows them.
    pub workspaces: Vec<Workspace>,
    /// The workspace shown on screen.
    pub active: WorkspaceId,
    /// The number the next account minted, whichever workspace it joins, is
    /// numbered with. Never the highest existing id — that would let a
    /// deleted newest account's number be handed to a new one (`FR.21.7`).
    pub next_account_number: u64,
    /// The number the next named workspace minted is numbered with.
    pub next_workspace_number: u64,
}
