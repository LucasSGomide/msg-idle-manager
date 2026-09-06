//! `XdgProfileLocator`: the [`ProfileLocator`] port backed by the XDG data
//! directory, one profile folder per session identifier.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use idle_manager_core::{ProfileDirectories, ProfileError, ProfileLocator, SessionId};

/// The application name used to derive the XDG data location.
const APP_NAME: &str = "idle-manager";

/// The file dropped and removed to prove a directory is writable.
const WRITE_PROBE: &str = ".idle-manager-write-probe";

/// The XDG data directory could not be resolved.
#[derive(Debug, thiserror::Error)]
pub enum LocatorSetup {
    /// No home directory is known, so there is no XDG data directory to root
    /// profiles under.
    #[error("no home directory: cannot resolve the XDG data directory")]
    NoHome,
}

/// Locates session profiles under `<XDG data>/idle-manager/profiles/<id>/`.
///
/// The data root is read from the environment, never hardcoded, so moving a
/// home directory or running under an unusual `XDG_DATA_HOME` keeps working.
#[derive(Debug, Clone)]
pub struct XdgProfileLocator {
    profiles_root: PathBuf,
}

impl XdgProfileLocator {
    /// Reads the XDG data directory from the environment and roots profiles
    /// under it.
    ///
    /// # Errors
    ///
    /// [`LocatorSetup::NoHome`] if no home directory can be determined.
    pub fn new() -> Result<Self, LocatorSetup> {
        let dirs = ProjectDirs::from("", "", APP_NAME).ok_or(LocatorSetup::NoHome)?;
        Ok(Self::under(dirs.data_dir()))
    }

    /// Roots profiles under an explicit data directory, for tests that must not
    /// touch the real XDG location.
    #[must_use]
    pub fn under(data_root: impl AsRef<Path>) -> Self {
        Self {
            profiles_root: data_root.as_ref().join("profiles"),
        }
    }
}

impl ProfileLocator for XdgProfileLocator {
    fn locate(&self, session: &SessionId) -> Result<ProfileDirectories, ProfileError> {
        let base = self.profiles_root.join(session.as_str());
        let data = base.join("data");
        let cache = base.join("cache");

        ensure_writable_dir(&data)?;
        ensure_writable_dir(&cache)?;

        Ok(ProfileDirectories { data, cache })
    }
}

/// Creates `dir` and every missing parent, then proves the process can write
/// into it. Maps a permission failure to [`ProfileError::NotWritable`] and
/// anything else to [`ProfileError::NotCreated`], so a caller can tell "the
/// path is unusable" from "the disk is locked down".
fn ensure_writable_dir(dir: &Path) -> Result<(), ProfileError> {
    if let Err(source) = fs::create_dir_all(dir) {
        return Err(classify(dir, source));
    }

    let probe = dir.join(WRITE_PROBE);
    match fs::File::create(&probe) {
        Ok(_) => {
            if let Err(error) = fs::remove_file(&probe) {
                tracing::warn!(path = %probe.display(), %error, "could not remove write probe");
            }
            Ok(())
        }
        Err(source) => Err(classify(dir, source)),
    }
}

fn classify(dir: &Path, source: io::Error) -> ProfileError {
    if source.kind() == io::ErrorKind::PermissionDenied {
        ProfileError::NotWritable {
            path: dir.to_owned(),
        }
    } else {
        ProfileError::NotCreated {
            path: dir.to_owned(),
            source,
        }
    }
}
