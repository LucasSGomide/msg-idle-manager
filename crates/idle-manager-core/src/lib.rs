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
mod remote;
mod session;
mod update;
mod workspace;
mod workspace_book;

pub use layout::{Layout, MoveOutcome, SlotId};
pub use memory::{BudgetVerdict, MemoryReading};
pub use ports::{
    MemoryProbe, MemoryProbeError, PhoneLink, PhoneRecordError, PhoneRecordStore, PresetCatalogue,
    PresetCatalogueReading, PresetFailure, ProfileDirectories, ProfileError, ProfileLocator,
    ProfileRemoval, ProfileRemovalError, UpdateChannel, UpdateCheck, UpdateError, UpdateInfo,
    VerifiedPackage, WorkspaceReadError, WorkspaceStore, WorkspaceWriteError, ZoomMemory,
    ZoomMemoryError,
};
pub use preset::{InvalidZoom, Preset, PresetId, ZoomLevel};
pub use remote::{
    DEFAULT_MOBILE_VIEWPORT, EnrolledPhone, EnrolmentOffer, Frame, HEARTBEAT_INTERVAL_SECS,
    PhoneStatus, Presence, PresenceChange, RemoteAccount, RemoteIntent, RemoteState,
    RemoteWorkspace, SILENCE_LIMIT_SECS, Viewport, scroll_script, tap_script,
};
pub use session::{
    Liveness, RememberedZoom, Session, SessionBook, SessionId, Visibility, WorkspaceId,
    account_name, workspace_name,
};
pub use update::{
    Effect, ParseVersionError, UpdateEvent, UpdatePolicy, UpdateSchedule, UpdateState, Version,
};
pub use workspace::{Account, SavedLiveness, Workspace, WorkspaceList};
pub use workspace_book::{
    DestinationWorkspace, Destinations, Switch, WorkspaceBook, WorkspaceRefusal, WorkspaceView,
};
