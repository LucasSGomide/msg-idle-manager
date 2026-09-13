//! The domain: what a session is, how it moves between states, and the traits
//! the rest of the application must satisfy to serve it.
//!
//! This crate knows nothing about GTK, `WebKit`, the filesystem or `/proc`. It
//! depends on no other crate in the workspace, which is what lets its tests run
//! in milliseconds without a display server. `make arch-check` fails the build
//! if a UI, serialisation or I/O dependency ever reaches it.

mod layout;
mod memory;
mod ports;
mod preset;
mod session;
mod workspace;

pub use layout::{Layout, MoveOutcome, Outcome, Placement, SlotId, arrange};
pub use memory::{BudgetVerdict, MemoryReading};
pub use ports::{
    MemoryProbe, MemoryProbeError, PresetCatalogue, PresetCatalogueReading, PresetFailure,
    ProfileDirectories, ProfileError, ProfileLocator, WorkspaceReadError, WorkspaceStore,
    WorkspaceWriteError, ZoomMemory, ZoomMemoryError,
};
pub use preset::{InvalidZoom, Preset, PresetId, ZoomLevel};
pub use session::{
    Liveness, RememberedZoom, Session, SessionBook, SessionId, Visibility, account_name,
};
pub use workspace::{Account, SavedLiveness, Workspace};
