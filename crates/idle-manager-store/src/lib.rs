//! Persistence: XDG locations, the on-disk preset catalogue, and the session
//! file that survives a restart.
//!
//! The record types here are the file format, deliberately separate from the
//! domain types they map to. That separation is what lets the domain be
//! refactored without rewriting files a user already has on disk.

mod paths;
mod preset;
mod session_file;

pub use paths::{LocatorSetup, XdgProfileLocator, presets_dir, workspace_file};
pub use preset::{PresetFileError, TomlPresetCatalogue};
pub use session_file::{SessionFileError, SessionWriteError, TomlWorkspaceStore};
