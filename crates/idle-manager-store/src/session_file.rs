//! `TomlWorkspaceStore`: the [`WorkspaceStore`] port backed by one hand-editable
//! TOML file under the XDG configuration directory.
//!
//! The on-disk shape is [`SessionFile`], its own serde types mapped to and from
//! the domain [`Workspace`] (architecture rule 7). The file is a contract with
//! people who already have one on disk: a domain rename must never turn into a
//! broken file, and the format carries a `version` from its first write so a
//! later change can tell a file that predates it from one it understands.
//!
//! [`WorkspaceStore`]: idle_manager_core::WorkspaceStore

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use idle_manager_core::{
    Account, Layout, SavedLiveness, SessionId, SlotId, Visibility, Workspace, WorkspaceReadError,
    WorkspaceStore, WorkspaceWriteError, ZoomLevel,
};
use serde::{Deserialize, Serialize};

use crate::paths::{self, LocatorSetup};

/// The one format version this build writes and reads. A file carrying any
/// other number is set aside, not guessed at — this is the first version, so
/// there is nothing older to migrate and nothing newer this build can know.
const FORMAT_VERSION: u64 = 1;

/// The workspace file's name under the configuration directory.
const WORKSPACE_FILE: &str = "sessions.toml";

/// The saved workspace could not be read.
///
/// Four cases a caller must tell apart: there is no file yet (a first run, not
/// an error to the user), the file could not be reached, the file will not
/// parse, and the file is a version this build does not understand. The last
/// two set the original bytes aside intact and carry where they went, so the
/// caller can name it and open as a first run (code standards rule 12).
#[derive(Debug, thiserror::Error)]
pub enum SessionFileError {
    /// No workspace file exists at `path` — the first time the program runs.
    #[error("no workspace file at {}", .path.display())]
    Missing {
        /// Where a workspace file would have been.
        path: PathBuf,
    },
    /// The workspace file exists but could not be read from disk.
    #[error("could not read the workspace file {}: {source}", .path.display())]
    Unreachable {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying failure.
        #[source]
        source: io::Error,
    },
    /// The file is not valid TOML, is missing a required key, or holds a value
    /// the domain cannot represent. The bytes are kept at `kept`.
    #[error("the workspace file {} is malformed: {reason}", .path.display())]
    Malformed {
        /// The path the file was read from.
        path: PathBuf,
        /// Where the unreadable bytes were moved, intact.
        kept: PathBuf,
        /// One line describing what was wrong, ready to show as-is.
        reason: String,
    },
    /// The file carries a `version` this build does not understand. The bytes
    /// are kept at `kept` rather than read as if they were current.
    #[error(
        "the workspace file {} is version {found}; this build understands version {}",
        .path.display(), FORMAT_VERSION
    )]
    UnknownVersion {
        /// The path the file was read from.
        path: PathBuf,
        /// Where the file was moved, intact.
        kept: PathBuf,
        /// The version the file declared.
        found: u64,
    },
}

/// The workspace file could not be written.
#[derive(Debug, thiserror::Error)]
pub enum SessionWriteError {
    /// The configuration directory could not be created.
    #[error("could not prepare the configuration directory {}: {source}", .path.display())]
    Directory {
        /// The directory that could not be prepared.
        path: PathBuf,
        /// The underlying failure.
        #[source]
        source: io::Error,
    },
    /// The file could not be written or renamed into place.
    #[error("could not write the workspace file {}: {source}", .path.display())]
    Write {
        /// The path being written.
        path: PathBuf,
        /// The underlying failure.
        #[source]
        source: io::Error,
    },
    /// The workspace could not be turned into TOML.
    #[error("could not serialise the workspace: {0}")]
    Serialise(#[from] toml::ser::Error),
}

/// The on-disk shape of the whole workspace.
///
/// Its own serde types, deliberately separate from the domain [`Workspace`]
/// (architecture rule 7). `version` is the first field so it is the first line
/// written. Unknown keys are rejected, as `PresetFile` rejects them, so a
/// mistyped key is a reported failure rather than a value silently ignored.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionFile {
    /// The format version. Checked before anything else on read.
    version: u64,
    /// The layout the window was arranged for.
    layout: LayoutRecord,
    /// One table per account, in the order they were added.
    #[serde(rename = "account", default)]
    accounts: Vec<SessionEntry>,
}

/// The on-disk shape of one account.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionEntry {
    /// The identifier that names the account's profile folder on disk.
    id: String,
    /// The name the user typed.
    name: String,
    /// The address the game starts at.
    url: String,
    /// Whether the account was running or parked — the only two saved states.
    liveness: LivenessRecord,
    /// The slot the account held, or absent for an account that was off-grid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    slot: Option<usize>,
    /// Whether the account keeps running at full speed while hidden.
    keep_awake: bool,
    /// A plain zoom multiplier — `zoom = 0.8` — exactly what the engine takes.
    zoom: f64,
    /// The identity to present to the game. Absent means the engine's own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    user_agent: Option<String>,
}

/// The on-disk spelling of [`Layout`]. A string the domain cannot map is a
/// parse failure, never a default.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LayoutRecord {
    Single,
    SideBySide,
    Grid,
}

impl From<Layout> for LayoutRecord {
    fn from(layout: Layout) -> Self {
        match layout {
            Layout::Single => LayoutRecord::Single,
            Layout::SideBySide => LayoutRecord::SideBySide,
            Layout::Grid => LayoutRecord::Grid,
        }
    }
}

impl From<LayoutRecord> for Layout {
    fn from(record: LayoutRecord) -> Self {
        match record {
            LayoutRecord::Single => Layout::Single,
            LayoutRecord::SideBySide => Layout::SideBySide,
            LayoutRecord::Grid => Layout::Grid,
        }
    }
}

/// The on-disk spelling of [`SavedLiveness`]. Running or parked only; a
/// starting or queued account has already been flattened to running by
/// `SessionBook::workspace`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum LivenessRecord {
    Running,
    Parked,
}

impl From<SavedLiveness> for LivenessRecord {
    fn from(liveness: SavedLiveness) -> Self {
        match liveness {
            SavedLiveness::Running => LivenessRecord::Running,
            SavedLiveness::Parked => LivenessRecord::Parked,
        }
    }
}

impl From<LivenessRecord> for SavedLiveness {
    fn from(record: LivenessRecord) -> Self {
        match record {
            LivenessRecord::Running => SavedLiveness::Running,
            LivenessRecord::Parked => SavedLiveness::Parked,
        }
    }
}

/// Just enough of the file to read its `version` before trusting the rest.
#[derive(Deserialize)]
struct VersionProbe {
    version: u64,
}

/// Reads and writes the workspace as one TOML file under the configuration
/// directory.
///
/// Built with [`TomlWorkspaceStore::new`] against the real config location, or
/// [`TomlWorkspaceStore::under`] against an explicit directory for tests.
#[derive(Debug, Clone)]
pub struct TomlWorkspaceStore {
    path: PathBuf,
}

impl TomlWorkspaceStore {
    /// Resolves the workspace file from the environment —
    /// `<XDG config>/idle-manager/sessions.toml`.
    ///
    /// # Errors
    ///
    /// [`LocatorSetup::NoHome`] if no home directory can be determined.
    pub fn new() -> Result<Self, LocatorSetup> {
        Ok(Self {
            path: paths::workspace_file()?,
        })
    }

    /// Roots the store at an explicit directory, for tests that must not touch
    /// the real config location. The file is `sessions.toml` inside it.
    #[must_use]
    pub fn under(directory: impl AsRef<Path>) -> Self {
        Self {
            path: directory.as_ref().join(WORKSPACE_FILE),
        }
    }

    /// The file this store reads and writes.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Reads the workspace now, with every failure told apart.
    ///
    /// # Errors
    ///
    /// [`SessionFileError::Missing`] on a first run with no file;
    /// [`SessionFileError::Unreachable`] if the file cannot be read;
    /// [`SessionFileError::Malformed`] or [`SessionFileError::UnknownVersion`]
    /// if it will not parse — in both cases the bytes are kept aside at the
    /// path the error carries.
    pub fn load(&self) -> Result<Workspace, SessionFileError> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(source) if source.kind() == io::ErrorKind::NotFound => {
                return Err(SessionFileError::Missing {
                    path: self.path.clone(),
                });
            }
            Err(source) => {
                return Err(SessionFileError::Unreachable {
                    path: self.path.clone(),
                    source,
                });
            }
        };

        self.parse(&text)
    }

    /// Writes `workspace`, replacing any previously saved one.
    ///
    /// The write goes to a temporary file in the target's own directory and is
    /// then renamed over the target, so a crash mid-write leaves the previous
    /// file intact and never a half-written one.
    ///
    /// # Errors
    ///
    /// [`SessionWriteError`] if the directory cannot be prepared, the workspace
    /// cannot be serialised, or the file cannot be written or renamed.
    ///
    /// # Panics
    ///
    /// If the store's path has no parent directory — impossible for a path
    /// built by [`TomlWorkspaceStore::new`] or [`TomlWorkspaceStore::under`],
    /// both of which join a file name onto a directory.
    pub fn save(&self, workspace: &Workspace) -> Result<(), SessionWriteError> {
        let directory = self
            .path
            .parent()
            .expect("a store path is a file name joined onto a directory");
        fs::create_dir_all(directory).map_err(|source| SessionWriteError::Directory {
            path: directory.to_owned(),
            source,
        })?;

        let file = SessionFile {
            version: FORMAT_VERSION,
            layout: workspace.layout.into(),
            accounts: workspace.accounts.iter().map(account_to_entry).collect(),
        };
        let body = toml::to_string(&file)?;

        let temporary = directory.join(format!("{WORKSPACE_FILE}.{}.tmp", std::process::id()));
        fs::write(&temporary, body).map_err(|source| SessionWriteError::Write {
            path: temporary.clone(),
            source,
        })?;
        fs::rename(&temporary, &self.path).map_err(|source| {
            if let Err(error) = fs::remove_file(&temporary) {
                tracing::warn!(path = %temporary.display(), %error, "could not remove the workspace temporary file");
            }
            SessionWriteError::Write {
                path: self.path.clone(),
                source,
            }
        })
    }

    fn parse(&self, text: &str) -> Result<Workspace, SessionFileError> {
        let probe: VersionProbe = toml::from_str(text).map_err(|error| self.malformed(&error))?;
        if probe.version != FORMAT_VERSION {
            return Err(SessionFileError::UnknownVersion {
                path: self.path.clone(),
                kept: self.quarantine(),
                found: probe.version,
            });
        }

        let file: SessionFile = toml::from_str(text).map_err(|error| self.malformed(&error))?;

        let accounts = file
            .accounts
            .into_iter()
            .map(entry_to_account)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|reason| SessionFileError::Malformed {
                path: self.path.clone(),
                kept: self.quarantine(),
                reason,
            })?;

        Ok(Workspace {
            accounts,
            layout: file.layout.into(),
        })
    }

    fn malformed(&self, error: &toml::de::Error) -> SessionFileError {
        SessionFileError::Malformed {
            path: self.path.clone(),
            kept: self.quarantine(),
            reason: error.to_string(),
        }
    }

    /// Moves the unreadable file aside to a kept name that does not already
    /// exist, so its bytes are never destroyed and never overwritten by the
    /// next save. Returns where it went even if the move itself failed.
    fn quarantine(&self) -> PathBuf {
        let base = self.path.with_extension("bad");
        let mut kept = base.clone();
        let mut serial = 1;
        while kept.exists() {
            kept = PathBuf::from(format!("{}.{serial}", base.display()));
            serial += 1;
        }

        if let Err(error) = fs::rename(&self.path, &kept) {
            tracing::warn!(
                from = %self.path.display(), to = %kept.display(), %error,
                "could not set the unreadable workspace file aside"
            );
        }
        kept
    }
}

impl WorkspaceStore for TomlWorkspaceStore {
    fn read(&self) -> Result<Option<Workspace>, WorkspaceReadError> {
        match self.load() {
            Ok(workspace) => Ok(Some(workspace)),
            Err(SessionFileError::Missing { .. }) => Ok(None),
            Err(SessionFileError::Unreachable { path, source }) => {
                Err(WorkspaceReadError::Inaccessible {
                    reason: format!("{}: {source}", path.display()),
                })
            }
            Err(SessionFileError::Malformed { kept, reason, .. }) => {
                Err(WorkspaceReadError::Unreadable { kept, reason })
            }
            Err(SessionFileError::UnknownVersion { kept, found, .. }) => {
                Err(WorkspaceReadError::Unreadable {
                    kept,
                    reason: format!(
                        "the file is version {found}, which this build does not understand"
                    ),
                })
            }
        }
    }

    fn write(&self, workspace: &Workspace) -> Result<(), WorkspaceWriteError> {
        self.save(workspace).map_err(|error| WorkspaceWriteError {
            reason: error.to_string(),
        })
    }
}

/// Maps one on-disk entry to a domain [`Account`], failing with a one-line
/// reason for any value the domain cannot represent.
fn entry_to_account(entry: SessionEntry) -> Result<Account, String> {
    let zoom = ZoomLevel::new(entry.zoom).map_err(|error| error.to_string())?;

    let browser_identity = match entry.user_agent {
        Some(identity) if identity.trim().is_empty() => {
            return Err(
                "the user_agent key is present but empty; remove it for the engine's own identity"
                    .to_owned(),
            );
        }
        other => other,
    };

    let (visibility, remembered_slot) = match entry.slot {
        Some(index) => {
            let slot = SlotId::new(index);
            (Visibility::InSlot(slot), Some(slot))
        }
        None => (Visibility::OffGrid, None),
    };

    Ok(Account {
        id: SessionId::new(entry.id),
        display_name: entry.name,
        start_address: entry.url,
        liveness: entry.liveness.into(),
        visibility,
        remembered_slot,
        is_kept_awake: entry.keep_awake,
        browser_identity,
        zoom,
    })
}

/// Maps one domain [`Account`] to its on-disk entry. An off-grid account is
/// written with no `slot` key at all — nothing for off-grid.
fn account_to_entry(account: &Account) -> SessionEntry {
    SessionEntry {
        id: account.id.as_str().to_owned(),
        name: account.display_name.clone(),
        url: account.start_address.clone(),
        liveness: account.liveness.into(),
        slot: match account.visibility {
            Visibility::InSlot(slot) => Some(slot.index()),
            Visibility::OffGrid => None,
        },
        keep_awake: account.is_kept_awake,
        zoom: account.zoom.multiplier(),
        user_agent: account.browser_identity.clone(),
    }
}
