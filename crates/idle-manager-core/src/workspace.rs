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
use crate::session::{SessionId, Visibility};

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

/// Every account in the order it was added, plus the layout the window was
/// arranged for.
///
/// The value [`SessionBook::workspace`] produces and [`SessionBook::restore`]
/// consumes. Reading one back off a book restored from it reproduces it field
/// for field, with a starting or queued account reported as
/// [`SavedLiveness::Running`].
///
/// [`SessionBook::workspace`]: crate::SessionBook::workspace
/// [`SessionBook::restore`]: crate::SessionBook::restore
#[derive(Debug, Clone, PartialEq)]
pub struct Workspace {
    /// The accounts, in the order they were added.
    pub accounts: Vec<Account>,
    /// The layout the window was arranged for.
    pub layout: Layout,
}
