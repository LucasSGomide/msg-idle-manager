//! Persistence: XDG locations, the on-disk preset catalogue, the session
//! file that survives a restart, and the enrolled phone's record.
//!
//! The record types here are the file format, deliberately separate from the
//! domain types they map to. That separation is what lets the domain be
//! refactored without rewriting files a user already has on disk.

mod account_state;
mod paths;
mod phone_record;
mod preset;
mod session_file;

pub use account_state::TomlZoomMemory;
// `cfg(any(windows, test))`, matching `paths::engine_data_root`'s own gate: on
// a Linux non-test build the function does not exist at all, so there is
// nothing to export; keeping the two `cfg`s in lock-step is what avoids an
// `unreachable_pub` warning on the Linux test build the function's own unit
// test runs under.
#[cfg(any(windows, test))]
pub use paths::engine_data_root;
pub use paths::{
    LocatorSetup, XdgProfileLocator, XdgProfileRemoval, phone_file, presets_dir, workspace_file,
};
pub use phone_record::TomlPhoneRecord;
pub use preset::{PresetFileError, TomlPresetCatalogue};
pub use session_file::{SessionFileError, SessionWriteError, TomlWorkspaceStore};
