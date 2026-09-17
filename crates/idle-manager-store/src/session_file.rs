//! `TomlWorkspaceStore`: the [`WorkspaceStore`] port backed by one hand-editable
//! TOML file under the XDG configuration directory.
//!
//! The on-disk shape is [`WorkspaceListFile`] (and, for an old file, the
//! version 1 [`SessionFileV1`]), its own serde types mapped to and from the
//! domain [`WorkspaceList`] (architecture rule 7). The file is a contract with
//! people who already have one on disk: a domain rename must never turn into a
//! broken file, and the format carries a `version` from its first write so a
//! later change can tell a file that predates it from one it understands.
//!
//! [`WorkspaceStore`]: idle_manager_core::WorkspaceStore

use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use idle_manager_core::{
    Account, Layout, SavedLiveness, SessionId, SlotId, Visibility, Workspace, WorkspaceId,
    WorkspaceList, WorkspaceReadError, WorkspaceStore, WorkspaceWriteError, ZoomLevel,
};
use serde::{Deserialize, Serialize};

use crate::paths::{self, LocatorSetup};

/// The format version this build writes. It still reads version 1, migrating
/// it to this shape on load (`FR.15.2`).
const FORMAT_VERSION: u64 = 2;

/// How many accounts a named workspace may hold — the number of places the
/// largest arrangement has. Derived from [`Layout::Grid`] rather than a bare
/// `4` (code standards rule 5); a hand-edited file that ignores this is
/// repaired on read, not rejected.
const NAMED_WORKSPACE_CAPACITY: usize = Layout::Grid.slot_count();

/// The workspace file's name under the configuration directory.
const WORKSPACE_FILE: &str = "sessions.toml";

/// The name a version 1 file is copied to, once, the first time it is written
/// over as version 2 — so a downgraded build can still read its old
/// arrangement.
const V1_BACKUP_FILE: &str = "sessions.v1.toml";

/// The saved workspace list could not be read.
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
    /// The file is not valid TOML, is missing a required key, holds a value
    /// the domain cannot represent, or names one account id more than once.
    /// The bytes are kept at `kept`.
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
        "the workspace file {} is version {found}; this build understands versions 1 and {}",
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
    /// The workspace list could not be turned into TOML.
    #[error("could not serialise the workspace: {0}")]
    Serialise(#[from] toml::ser::Error),
}

/// The on-disk shape of one account, exactly as version 1 wrote it and
/// version 2 still does, one per `[[workspace.account]]` table.
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

/// The version 1 shape: one arrangement for the whole window, before
/// workspaces existed. Kept only to read an old file; version 2 is always
/// written.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionFileV1 {
    /// The format version. Checked before anything else on read.
    version: u64,
    /// The layout the window was arranged for.
    layout: LayoutRecord,
    /// One table per account, in the order they were added.
    #[serde(rename = "account", default)]
    accounts: Vec<SessionEntry>,
}

/// The version 2 shape: every workspace, which one is shown, and both
/// minting counters.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceListFile {
    /// The format version. Checked before anything else on read.
    version: u64,
    /// The workspace shown on screen.
    active: String,
    /// The number the next minted account is numbered with.
    next_account: u64,
    /// The number the next minted named workspace is numbered with.
    next_workspace: u64,
    /// One table per workspace, in sidebar order.
    #[serde(rename = "workspace", default)]
    workspaces: Vec<WorkspaceRecord>,
}

/// The on-disk shape of one workspace.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkspaceRecord {
    /// The identifier this workspace was minted with, or `"ungrouped"`.
    id: String,
    /// The name shown on the sidebar heading.
    name: String,
    /// The layout this workspace was arranged for.
    layout: LayoutRecord,
    /// The slot a full-grid addition to this workspace displaces.
    focused: usize,
    /// Whether this workspace's sidebar heading is expanded.
    expanded: bool,
    /// One table per account, in the order they were added.
    #[serde(rename = "account", default)]
    accounts: Vec<SessionEntry>,
}

/// Just enough of the file to read its `version` before trusting the rest.
#[derive(Deserialize)]
struct VersionProbe {
    version: u64,
}

/// Reads and writes every workspace as one TOML file under the configuration
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

    /// Where a version 1 file is copied to, beside [`TomlWorkspaceStore::path`].
    fn v1_backup_path(&self) -> PathBuf {
        self.path.with_file_name(V1_BACKUP_FILE)
    }

    /// Reads the workspace list now, with every failure told apart.
    ///
    /// # Errors
    ///
    /// [`SessionFileError::Missing`] on a first run with no file;
    /// [`SessionFileError::Unreachable`] if the file cannot be read;
    /// [`SessionFileError::Malformed`] or [`SessionFileError::UnknownVersion`]
    /// if it will not parse — in both cases the bytes are kept aside at the
    /// path the error carries.
    pub fn load(&self) -> Result<WorkspaceList, SessionFileError> {
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

    /// Writes `workspaces`, replacing any previously saved list.
    ///
    /// If the file currently on disk is still version 1, it is copied once to
    /// [`V1_BACKUP_FILE`] beside it before being overwritten — a downgraded
    /// build can then still read its old arrangement. That copy is best
    /// effort: a failure is logged and does not block the write it precedes,
    /// since losing the backup is a smaller loss than losing the save. The
    /// write itself goes to a temporary file in the target's own directory and
    /// is then renamed over the target, so a crash mid-write leaves the
    /// previous file intact and never a half-written one.
    ///
    /// # Errors
    ///
    /// [`SessionWriteError`] if the directory cannot be prepared, the list
    /// cannot be serialised, or the file cannot be written or renamed.
    ///
    /// # Panics
    ///
    /// If the store's path has no parent directory — impossible for a path
    /// built by [`TomlWorkspaceStore::new`] or [`TomlWorkspaceStore::under`],
    /// both of which join a file name onto a directory.
    pub fn save(&self, workspaces: &WorkspaceList) -> Result<(), SessionWriteError> {
        let directory = self
            .path
            .parent()
            .expect("a store path is a file name joined onto a directory");
        fs::create_dir_all(directory).map_err(|source| SessionWriteError::Directory {
            path: directory.to_owned(),
            source,
        })?;

        self.snapshot_v1_backup();

        let file = WorkspaceListFile {
            version: FORMAT_VERSION,
            active: workspaces.active.as_str().to_owned(),
            next_account: workspaces.next_account_number,
            next_workspace: workspaces.next_workspace_number,
            workspaces: workspaces
                .workspaces
                .iter()
                .map(workspace_to_record)
                .collect(),
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

    /// Copies the file currently on disk to [`TomlWorkspaceStore::v1_backup_path`]
    /// if, and only if, it is still version 1 and no backup exists yet.
    /// Anything short of that — no file yet, an unreadable or malformed one, a
    /// backup already there — is silently not this function's problem; the
    /// save that follows handles those cases (or does not need to).
    fn snapshot_v1_backup(&self) {
        if self.v1_backup_path().exists() {
            return;
        }
        let Ok(text) = fs::read_to_string(&self.path) else {
            return;
        };
        let Ok(probe) = toml::from_str::<VersionProbe>(&text) else {
            return;
        };
        if probe.version != 1 {
            return;
        }

        if let Err(error) = fs::copy(&self.path, self.v1_backup_path()) {
            tracing::warn!(
                from = %self.path.display(), to = %self.v1_backup_path().display(), %error,
                "could not keep a copy of the version 1 workspace file"
            );
        }
    }

    fn parse(&self, text: &str) -> Result<WorkspaceList, SessionFileError> {
        let probe: VersionProbe = toml::from_str(text).map_err(|error| self.malformed(&error))?;
        match probe.version {
            1 => self.parse_v1(text),
            2 => self.parse_v2(text),
            found => Err(SessionFileError::UnknownVersion {
                path: self.path.clone(),
                kept: self.quarantine(),
                found,
            }),
        }
    }

    /// Reads a version 1 file as one active, expanded Ungrouped workspace: the
    /// accounts in file order, the file's layout, focus on the first place,
    /// and the next account number one above the highest restored id
    /// (`FR.15.2`).
    fn parse_v1(&self, text: &str) -> Result<WorkspaceList, SessionFileError> {
        let file: SessionFileV1 = toml::from_str(text).map_err(|error| self.malformed(&error))?;

        let accounts = file
            .accounts
            .into_iter()
            .map(entry_to_account)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|reason| self.malformed_with(reason))?;

        let next_account_number = highest_account_number(&accounts) + 1;

        Ok(WorkspaceList {
            workspaces: vec![Workspace {
                id: WorkspaceId::ungrouped(),
                name: "Ungrouped".to_owned(),
                focused: SlotId::FIRST,
                is_expanded: true,
                accounts,
                layout: file.layout.into(),
            }],
            active: WorkspaceId::ungrouped(),
            next_account_number,
            next_workspace_number: 1,
        })
    }

    /// Reads a version 2 file. A named workspace holding more accounts than
    /// [`NAMED_WORKSPACE_CAPACITY`] keeps the first four and the rest are
    /// appended to Ungrouped (creating an empty one if the file had none),
    /// with a `tracing::warn` — a hand edit costs grouping, not the whole
    /// file. An account id appearing more than once anywhere in the file is
    /// [`SessionFileError::Malformed`].
    fn parse_v2(&self, text: &str) -> Result<WorkspaceList, SessionFileError> {
        let file: WorkspaceListFile =
            toml::from_str(text).map_err(|error| self.malformed(&error))?;

        let mut workspaces = Vec::with_capacity(file.workspaces.len());
        let mut overflow: Vec<Account> = Vec::new();

        for record in file.workspaces {
            let id = WorkspaceId::new(record.id);
            let mut accounts = record
                .accounts
                .into_iter()
                .map(entry_to_account)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|reason| self.malformed_with(reason))?;

            if !id.is_ungrouped() && accounts.len() > NAMED_WORKSPACE_CAPACITY {
                tracing::warn!(
                    workspace = %id,
                    count = accounts.len(),
                    capacity = NAMED_WORKSPACE_CAPACITY,
                    "named workspace exceeds capacity; extra accounts moved to Ungrouped"
                );
                overflow.extend(accounts.split_off(NAMED_WORKSPACE_CAPACITY));
            }

            workspaces.push(Workspace {
                id,
                name: record.name,
                focused: SlotId::new(record.focused),
                is_expanded: record.expanded,
                accounts,
                layout: record.layout.into(),
            });
        }

        if !overflow.is_empty() {
            match workspaces
                .iter_mut()
                .find(|workspace| workspace.id.is_ungrouped())
            {
                Some(ungrouped) => ungrouped.accounts.extend(overflow),
                None => workspaces.push(Workspace {
                    accounts: overflow,
                    ..Workspace::default()
                }),
            }
        }

        self.reject_duplicate_ids(&workspaces)?;

        Ok(WorkspaceList {
            workspaces,
            active: WorkspaceId::new(file.active),
            next_account_number: file.next_account,
            next_workspace_number: file.next_workspace,
        })
    }

    /// Fails with [`SessionFileError::Malformed`] the moment the same account
    /// id appears in more than one place in the file — ids name a folder on
    /// disk and must be unique app-wide.
    fn reject_duplicate_ids(&self, workspaces: &[Workspace]) -> Result<(), SessionFileError> {
        let mut seen: HashSet<&SessionId> = HashSet::new();
        for account in workspaces.iter().flat_map(|workspace| &workspace.accounts) {
            if !seen.insert(&account.id) {
                return Err(self
                    .malformed_with(format!("account id {} appears more than once", account.id)));
            }
        }
        Ok(())
    }

    fn malformed(&self, error: &toml::de::Error) -> SessionFileError {
        self.malformed_with(error.to_string())
    }

    fn malformed_with(&self, reason: String) -> SessionFileError {
        SessionFileError::Malformed {
            path: self.path.clone(),
            kept: self.quarantine(),
            reason,
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
    fn read(&self) -> Result<Option<WorkspaceList>, WorkspaceReadError> {
        match self.load() {
            Ok(workspaces) => Ok(Some(workspaces)),
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

    fn write(&self, workspaces: &WorkspaceList) -> Result<(), WorkspaceWriteError> {
        self.save(workspaces).map_err(|error| WorkspaceWriteError {
            reason: error.to_string(),
        })
    }
}

/// The highest account number found among `accounts`' ids, or `0` if none
/// parse as `session-NNNN`.
fn highest_account_number(accounts: &[Account]) -> u64 {
    accounts
        .iter()
        .filter_map(|account| account.id.as_str().strip_prefix("session-"))
        .filter_map(|suffix| suffix.parse::<u64>().ok())
        .max()
        .unwrap_or(0)
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

/// Maps one domain [`Workspace`] to its on-disk record.
fn workspace_to_record(workspace: &Workspace) -> WorkspaceRecord {
    WorkspaceRecord {
        id: workspace.id.as_str().to_owned(),
        name: workspace.name.clone(),
        layout: workspace.layout.into(),
        focused: workspace.focused.index(),
        expanded: workspace.is_expanded,
        accounts: workspace.accounts.iter().map(account_to_entry).collect(),
    }
}
