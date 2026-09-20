//! `TomlZoomMemory`: the [`ZoomMemory`] port backed by one small TOML file at
//! the root of each account's profile folder.
//!
//! The on-disk shape is [`StateFile`], its own serde record mapped to and from
//! the domain [`RememberedZoom`] (architecture rule 7). The arrangement names
//! are written in full and in a spelling chosen for the file — `single`,
//! `side-by-side`, `grid` — so renaming something in the program later cannot
//! break a file already sitting on somebody's disk.
//!
//! [`ZoomMemory`]: idle_manager_core::ZoomMemory

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use idle_manager_core::{
    Layout, RememberedZoom, SessionId, ZoomLevel, ZoomMemory, ZoomMemoryError,
};
use serde::{Deserialize, Serialize};

use crate::paths::{self, LocatorSetup};

/// The state file's name at the root of an account's profile folder, a sibling
/// of the `data` and `cache` directories (`FR.12.3`).
const STATE_FILE: &str = "state.toml";

/// The on-disk spelling of [`Layout::Single`] under the `zoom` table.
const LAYOUT_KEY_SINGLE: &str = "single";
/// The on-disk spelling of [`Layout::SideBySide`].
const LAYOUT_KEY_SIDE_BY_SIDE: &str = "side-by-side";
/// The on-disk spelling of [`Layout::Grid`].
const LAYOUT_KEY_GRID: &str = "grid";

/// The on-disk shape of one account's state file.
///
/// Its own serde type, deliberately separate from the domain (architecture
/// rule 7). Unknown top-level keys are *not* rejected: `FR.12.3` shapes this
/// file so a later per-account setting is one more table, and an older build
/// must read a newer file as far as it understands rather than discarding it.
#[derive(Debug, Default, Serialize, Deserialize)]
struct StateFile {
    /// The chosen zoom multiplier per arrangement. Only the arrangements whose
    /// size has been changed appear (`FR.12.4`).
    #[serde(default)]
    zoom: BTreeMap<String, f64>,
}

/// Reads and writes each account's `state.toml`.
///
/// Built with [`TomlZoomMemory::new`] against the real XDG data location, or
/// [`TomlZoomMemory::under`] against an explicit directory for tests — the same
/// pair of constructors [`crate::XdgProfileLocator`] has.
#[derive(Debug, Clone)]
pub struct TomlZoomMemory {
    profiles_root: PathBuf,
}

impl TomlZoomMemory {
    /// Resolves the profiles root from the environment —
    /// `<XDG data>/idle-manager/profiles/`.
    ///
    /// # Errors
    ///
    /// [`LocatorSetup::NoHome`] if no home directory can be determined.
    pub fn new() -> Result<Self, LocatorSetup> {
        Ok(Self {
            profiles_root: paths::xdg_profiles_root()?,
        })
    }

    /// Roots the store at an explicit data directory, for tests that must not
    /// touch the real XDG location. Each account's file is
    /// `profiles/<id>/state.toml` under it.
    #[must_use]
    pub fn under(data_root: impl AsRef<Path>) -> Self {
        Self {
            profiles_root: paths::profiles_root(data_root.as_ref()),
        }
    }

    /// The `state.toml` path for `account`.
    #[must_use]
    pub fn state_file(&self, account: &SessionId) -> PathBuf {
        paths::account_profile_dir(&self.profiles_root, account).join(STATE_FILE)
    }
}

impl ZoomMemory for TomlZoomMemory {
    fn read(&self, account: &SessionId) -> RememberedZoom {
        let path = self.state_file(account);

        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return RememberedZoom::new(),
            Err(error) => {
                tracing::warn!(
                    account = %account, path = %path.display(), %error,
                    "could not read the account state file; nothing remembered"
                );
                return RememberedZoom::new();
            }
        };

        let file: StateFile = match toml::from_str(&text) {
            Ok(file) => file,
            Err(error) => {
                tracing::warn!(
                    account = %account, %error,
                    "the account state file will not parse; nothing remembered"
                );
                return RememberedZoom::new();
            }
        };

        file.zoom
            .into_iter()
            .filter_map(|(key, multiplier)| map_entry(account, &key, multiplier))
            .collect()
    }

    fn write(
        &self,
        account: &SessionId,
        remembered: &RememberedZoom,
    ) -> Result<(), ZoomMemoryError> {
        let path = self.state_file(account);
        let directory = path
            .parent()
            .expect("a state path is a file name joined onto a profile folder");
        fs::create_dir_all(directory).map_err(|source| not_stored(directory, &source))?;

        let file = StateFile {
            zoom: remembered
                .entries()
                .filter_map(|(layout, zoom)| {
                    key_for_layout(layout).map(|key| (key.to_owned(), zoom.multiplier()))
                })
                .collect(),
        };
        let body = toml::to_string(&file).map_err(|error| ZoomMemoryError::NotStored {
            reason: error.to_string(),
        })?;

        let temporary = directory.join(format!("{STATE_FILE}.{}.tmp", std::process::id()));
        fs::write(&temporary, body).map_err(|source| not_stored(&temporary, &source))?;
        fs::rename(&temporary, &path).map_err(|source| {
            if let Err(error) = fs::remove_file(&temporary) {
                tracing::warn!(path = %temporary.display(), %error, "could not remove the state temporary file");
            }
            not_stored(&path, &source)
        })
    }
}

/// Maps one on-disk `(key, multiplier)` pair to a domain entry, or `None` with
/// a log line when the key is unrecognised or the multiplier is out of range —
/// the same per-key tolerance a bad zoom in a game file already gets
/// (`FR.12.6`).
fn map_entry(account: &SessionId, key: &str, multiplier: f64) -> Option<(Layout, ZoomLevel)> {
    let Some(layout) = layout_for_key(key) else {
        tracing::warn!(account = %account, key, "unrecognised key under the zoom table; ignored");
        return None;
    };
    match ZoomLevel::new(multiplier) {
        Ok(zoom) => Some((layout, zoom)),
        Err(error) => {
            tracing::warn!(
                account = %account, key, %error,
                "the remembered zoom for this arrangement is out of range; dropped"
            );
            None
        }
    }
}

fn layout_for_key(key: &str) -> Option<Layout> {
    match key {
        LAYOUT_KEY_SINGLE => Some(Layout::Single),
        LAYOUT_KEY_SIDE_BY_SIDE => Some(Layout::SideBySide),
        LAYOUT_KEY_GRID => Some(Layout::Grid),
        _ => None,
    }
}

/// The on-disk key for `layout`, or `None` for [`Layout::Mobile`], which has
/// no key: the size is locked there, `SessionBook` never records one for it,
/// and a `mobile` key would be a word the file never needs (Remote Access
/// `FR.3.2`).
fn key_for_layout(layout: Layout) -> Option<&'static str> {
    match layout {
        Layout::Single => Some(LAYOUT_KEY_SINGLE),
        Layout::SideBySide => Some(LAYOUT_KEY_SIDE_BY_SIDE),
        Layout::Grid => Some(LAYOUT_KEY_GRID),
        Layout::Mobile => None,
    }
}

fn not_stored(path: &Path, source: &io::Error) -> ZoomMemoryError {
    ZoomMemoryError::NotStored {
        reason: format!("{}: {source}", path.display()),
    }
}
