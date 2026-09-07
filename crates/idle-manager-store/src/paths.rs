//! XDG locations: [`XdgProfileLocator`] roots one profile folder per session
//! identifier under the XDG **data** directory, and [`presets_dir`] resolves the
//! hand-editable preset folder under the XDG **config** directory. The split is
//! `FR.8.3` — configuration a user edits lives apart from data the engine
//! writes.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use directories::ProjectDirs;
use idle_manager_core::{ProfileDirectories, ProfileError, ProfileLocator, SessionId};

/// The application name used to derive the XDG data location.
const APP_NAME: &str = "idle-manager";

/// The file dropped and removed to prove a directory is writable.
const WRITE_PROBE: &str = ".idle-manager-write-probe";

/// An XDG base directory could not be resolved.
#[derive(Debug, thiserror::Error)]
pub enum LocatorSetup {
    /// No home directory is known, so there is no XDG base directory to root
    /// the profiles or the presets under.
    #[error("no home directory: cannot resolve an XDG base directory")]
    NoHome,
}

/// The directory hand-editable preset files live in:
/// `<XDG config>/idle-manager/presets/`.
///
/// Under the XDG **config** directory, never the data directory the profiles
/// use (`FR.8.3`): a preset is configuration a user is invited to edit, a
/// profile is data the engine owns. The path is read from the environment, so a
/// moved home directory or an unusual `XDG_CONFIG_HOME` keeps working.
///
/// # Errors
///
/// [`LocatorSetup::NoHome`] if no home directory can be determined.
pub fn presets_dir() -> Result<PathBuf, LocatorSetup> {
    let dirs = ProjectDirs::from("", "", APP_NAME).ok_or(LocatorSetup::NoHome)?;
    Ok(dirs.config_dir().join("presets"))
}

/// The workspace file the application saves its arrangement into:
/// `<XDG config>/idle-manager/sessions.toml`.
///
/// Under the XDG **config** directory, beside the `presets/` folder and never
/// the data directory the profiles use (`FR.8.3`): the workspace is
/// configuration a curious user may open and edit. The path is read from the
/// environment, so a moved home directory or an unusual `XDG_CONFIG_HOME` keeps
/// working.
///
/// # Errors
///
/// [`LocatorSetup::NoHome`] if no home directory can be determined.
pub fn workspace_file() -> Result<PathBuf, LocatorSetup> {
    let dirs = ProjectDirs::from("", "", APP_NAME).ok_or(LocatorSetup::NoHome)?;
    Ok(dirs.config_dir().join("sessions.toml"))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_workspace_file_resolves_beside_the_presets_folder_under_xdg_config() {
        let dirs = ProjectDirs::from("", "", APP_NAME).expect("a home directory to resolve from");
        let workspace = workspace_file().expect("resolve the workspace file");
        let presets = presets_dir().expect("resolve the presets directory");

        assert_eq!(
            (
                workspace.parent(),
                presets.parent(),
                workspace.starts_with(dirs.config_dir()),
                workspace.starts_with(dirs.data_dir()),
            ),
            (
                Some(dirs.config_dir()),
                Some(dirs.config_dir()),
                true,
                false,
            ),
        );
    }
}
