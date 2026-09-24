//! [`VelopackChannel`]: the [`UpdateChannel`] port, fulfilled over Velopack's
//! own `UpdateManager` (roadmap item 16 task 06).
//!
//! A GitHub release feed answers [`UpdateChannel::check`]. [`UpdateChannel::download`]
//! asks Velopack's manager for the package itself — which already checks its
//! size and its checksum against the feed — then independently recomputes the
//! SHA-256 with `sha2` before fetching the release's `.minisig` with `ureq`
//! (`sources::UpdateSource` resolves a package's own download URL internally
//! and never hands it back, so the signature has to be fetched by a URL this
//! module builds itself) and checking it with [`signature::verify_package`].
//! A package that fails either check is deleted before this returns, never
//! handed to [`UpdateChannel::apply_on_exit`] half-verified.

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::mpsc;

use idle_manager_core::{UpdateChannel, UpdateError, UpdateInfo, VerifiedPackage, Version};
use sha2::{Digest, Sha256};
use velopack::locator::{
    LocationContext, VelopackLocator, VelopackLocatorConfig, auto_locate_app_manifest,
};
use velopack::sources::{GithubSource, UpdateSource};

use crate::signature;

/// Why a [`VelopackChannel`] could not be built.
#[derive(Debug, thiserror::Error)]
pub enum ChannelSetup {
    /// `repository_url` was not shaped like `https://github.com/<owner>/<repo>`.
    #[error("not a GitHub repository URL: {url}")]
    NotGithub {
        /// The URL that was rejected.
        url: String,
    },
    /// Velopack could not locate this application's own install, or could
    /// not make sense of the locator it was handed.
    #[error("could not prepare the update channel: {reason}")]
    Unavailable {
        /// One line describing what went wrong, ready to log.
        reason: String,
    },
}

/// Where to fetch a release asset's companion `.minisig` from.
///
/// `sources::UpdateSource` downloads the package itself but never hands back
/// the URL it used, so the signature's URL has to be built independently —
/// and built differently depending on where the release actually lives.
#[derive(Debug, Clone)]
enum AssetLocation {
    /// A GitHub repository: every release's assets sit under its own
    /// `releases/download/v<version>/` path.
    GithubRelease {
        /// The repository URL, with no trailing slash.
        repo_url: String,
    },
    /// A flat HTTP directory carrying every asset directly under one base
    /// URL. Exists only for this crate's own integration tests, which serve
    /// a fixture release from a local server rather than reaching GitHub.
    Flat {
        /// The base URL, with no trailing slash.
        base_url: String,
    },
}

impl AssetLocation {
    fn minisig_url(&self, version: Version, file_name: &str) -> String {
        match self {
            Self::GithubRelease { repo_url } => {
                format!("{repo_url}/releases/download/v{version}/{file_name}.minisig")
            }
            Self::Flat { base_url } => format!("{base_url}/{file_name}.minisig"),
        }
    }

    fn notes_url(&self, version: Version) -> String {
        match self {
            Self::GithubRelease { repo_url } | Self::Flat { base_url: repo_url } => {
                format!("{repo_url}/releases/tag/v{version}")
            }
        }
    }
}

/// The [`UpdateChannel`] port, fulfilled over Velopack's own `UpdateManager`
/// (architecture rules 2, 3, 4, 5, 6).
pub struct VelopackChannel {
    manager: velopack::UpdateManager,
    packages_dir: PathBuf,
    asset_location: AssetLocation,
}

impl fmt::Debug for VelopackChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VelopackChannel")
            .field(
                "current_version",
                &self.manager.get_current_version_as_string(),
            )
            .field("packages_dir", &self.packages_dir)
            .field("asset_location", &self.asset_location)
            .finish()
    }
}

impl VelopackChannel {
    /// Builds a channel over the GitHub repository at `repository_url`
    /// (`https://github.com/<owner>/<repo>`), auto-locating this
    /// application's own Velopack install (the composition root's binary,
    /// never a plain `cargo run`).
    ///
    /// # Errors
    ///
    /// [`ChannelSetup::NotGithub`] if `repository_url` is not shaped like a
    /// GitHub repository; [`ChannelSetup::Unavailable`] if this application
    /// is not installed the way Velopack expects.
    pub fn new(repository_url: &str) -> Result<Self, ChannelSetup> {
        let repo_url = validate_github_url(repository_url)?;
        let source = GithubSource::new(repository_url, None, false);
        Self::build(source, None, AssetLocation::GithubRelease { repo_url })
    }

    /// Builds a channel over an arbitrary Velopack source, resolving a
    /// package's signature under `asset_base_url` instead of a GitHub
    /// release, and locating the current install from `locator_config`
    /// instead of auto-locating it.
    ///
    /// Used only by this crate's own integration tests
    /// (`tests/downloading-a-release.rs`), which serve a release fixture
    /// from a local HTTP server and must never reach the real GitHub. The
    /// composition root always calls [`VelopackChannel::new`] instead.
    ///
    /// # Errors
    ///
    /// [`ChannelSetup::Unavailable`] if `locator_config` does not describe a
    /// real install — the `Update.exe` placeholder and the manifest file it
    /// names must both exist on disk.
    pub fn over_source_for_tests<S: UpdateSource + 'static>(
        source: S,
        locator_config: VelopackLocatorConfig,
        asset_base_url: &str,
    ) -> Result<Self, ChannelSetup> {
        Self::build(
            source,
            Some(locator_config),
            AssetLocation::Flat {
                base_url: asset_base_url.trim_end_matches('/').to_string(),
            },
        )
    }

    fn build<S: UpdateSource + 'static>(
        source: S,
        locator_config: Option<VelopackLocatorConfig>,
        asset_location: AssetLocation,
    ) -> Result<Self, ChannelSetup> {
        let locate = |config: &VelopackLocatorConfig| VelopackLocator::new(config);
        let locator = match &locator_config {
            Some(config) => locate(config),
            None => auto_locate_app_manifest(LocationContext::FromCurrentExe),
        }
        .map_err(|error| ChannelSetup::Unavailable {
            reason: error.to_string(),
        })?;
        let packages_dir = locator.get_packages_dir();

        let manager =
            velopack::UpdateManager::new(source, None, locator_config).map_err(|error| {
                ChannelSetup::Unavailable {
                    reason: error.to_string(),
                }
            })?;

        Ok(Self {
            manager,
            packages_dir,
            asset_location,
        })
    }

    /// The release asset matching `version`, read fresh from the feed —
    /// [`UpdateChannel::check`]'s own answer never carries enough for
    /// Velopack's own manager to download from (its file name, its size, its
    /// checksums), so [`UpdateChannel::download`] looks it up again by the
    /// version the caller asked for.
    fn find_asset(&self, version: Version) -> Result<velopack::VelopackAsset, UpdateError> {
        let feed = self
            .manager
            .get_release_feed()
            .map_err(map_velopack_error)?;
        feed.Assets
            .into_iter()
            .find(|asset| Version::from_str(&asset.Version).is_ok_and(|found| found == version))
            .ok_or_else(|| UpdateError::Malformed {
                reason: format!("no release asset published for version {version}"),
            })
    }
}

impl UpdateChannel for VelopackChannel {
    fn check(&self) -> Result<idle_manager_core::UpdateCheck, UpdateError> {
        match self
            .manager
            .check_for_updates()
            .map_err(map_velopack_error)?
        {
            velopack::UpdateCheck::UpdateAvailable(info) => {
                let raw_version = info.TargetFullRelease.Version.clone();
                let version =
                    Version::from_str(&raw_version).map_err(|_| UpdateError::Malformed {
                        reason: format!("unreadable release version {raw_version:?}"),
                    })?;
                Ok(idle_manager_core::UpdateCheck::Available(UpdateInfo {
                    version,
                    notes_url: self.asset_location.notes_url(version),
                }))
            }
            velopack::UpdateCheck::NoUpdateAvailable | velopack::UpdateCheck::RemoteIsEmpty => {
                Ok(idle_manager_core::UpdateCheck::UpToDate)
            }
        }
    }

    fn download(
        &self,
        info: &UpdateInfo,
        progress: &(dyn Fn(u8) + Send),
    ) -> Result<VerifiedPackage, UpdateError> {
        let asset = self.find_asset(info.version)?;
        let package_path = self.packages_dir.join(&asset.FileName);

        let velopack_info = velopack::UpdateInfo {
            TargetFullRelease: asset.clone(),
            ..Default::default()
        };
        let (sender, receiver) = mpsc::channel::<i16>();
        // The download itself runs on a second, scoped thread so this one is
        // free to drain `receiver` and call `progress` as percentages
        // arrive — `progress` is a borrowed, non-`'static` callback (the
        // shell's own closure back to the GTK main context), so it can only
        // ever be called from the thread that already holds it.
        let download_result = std::thread::scope(|scope| {
            let handle =
                scope.spawn(|| self.manager.download_updates(&velopack_info, Some(sender)));
            while let Ok(percent) = receiver.recv() {
                progress(u8::try_from(percent.clamp(0, 100)).unwrap_or(100));
            }
            handle.join().unwrap_or_else(|_| {
                Err(velopack::Error::Other(
                    "the download thread panicked".to_string(),
                ))
            })
        });
        download_result.map_err(map_velopack_error)?;

        if let Err(error) = verify_checksum(&package_path, &asset.SHA256) {
            delete_unverified_package(&package_path, &error);
            return Err(error);
        }

        let minisig_path = minisig_path_for(&package_path);
        let minisig_url = self
            .asset_location
            .minisig_url(info.version, &asset.FileName);
        if let Err(error) = fetch_signature(&minisig_url, &minisig_path) {
            delete_unverified_package(&package_path, &error);
            return Err(error);
        }

        match signature::verify_package(&package_path, &minisig_path) {
            Ok(()) => Ok(VerifiedPackage {
                version: info.version,
                path: package_path,
            }),
            Err(error) => Err(UpdateError::Rejected {
                reason: error.to_string(),
            }),
        }
    }

    fn apply_on_exit(&self, package: &VerifiedPackage, relaunch: bool) -> Result<(), UpdateError> {
        let file_name = package
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| UpdateError::Io {
                reason: format!("not a package file name: {}", package.path.display()),
            })?;
        let asset = velopack::VelopackAsset {
            FileName: file_name.to_string(),
            ..Default::default()
        };

        self.manager
            .wait_exit_then_apply_updates(asset, false, relaunch, Vec::<String>::new())
            .map_err(map_velopack_error)
    }

    fn current_version(&self) -> Version {
        let raw = self.manager.get_current_version_as_string();
        Version::from_str(&raw).unwrap_or_else(|_| {
            tracing::warn!(version = %raw, "the running version does not parse; reporting 0.0.0");
            Version::new(0, 0, 0)
        })
    }

    fn last_apply_failure(&self) -> Option<String> {
        // A package still sitting in the packages directory, newer than the
        // manifest version this same locate just read, means the swap
        // `apply_on_exit` staged did not apply — a locked file or a failed
        // write left the previous version running (roadmap item 16's own
        // message-strip line names exactly this case).
        self.manager.get_update_pending_restart().map(|asset| {
            format!(
                "the update to {} did not apply; the previous version is running",
                asset.Version
            )
        })
    }
}

/// `url` must be `https://github.com/<owner>/<repo>`, with no further path,
/// query or fragment. Returns the URL with any trailing slash trimmed.
fn validate_github_url(url: &str) -> Result<String, ChannelSetup> {
    let not_github = || ChannelSetup::NotGithub {
        url: url.to_string(),
    };

    let trimmed = url.trim_end_matches('/');
    let path = trimmed
        .strip_prefix("https://github.com/")
        .ok_or_else(not_github)?;

    let mut segments = path.split('/');
    let (Some(owner), Some(repo), None) = (segments.next(), segments.next(), segments.next())
    else {
        return Err(not_github());
    };
    if owner.is_empty() || repo.is_empty() {
        return Err(not_github());
    }

    Ok(trimmed.to_string())
}

/// Recomputes `package_path`'s SHA-256 and compares it against
/// `expected_sha256`, independently of the checksum check Velopack's own
/// `download_updates` already performed — the roadmap's own two checks,
/// intact then signed, kept as two separate steps rather than trusting a
/// dependency's internals to keep doing the first one. An empty
/// `expected_sha256` (an older feed with only a SHA1) is treated as nothing
/// to check here.
fn verify_checksum(package_path: &Path, expected_sha256: &str) -> Result<(), UpdateError> {
    if expected_sha256.is_empty() {
        return Ok(());
    }

    let bytes = fs::read(package_path).map_err(|error| UpdateError::Io {
        reason: error.to_string(),
    })?;
    let digest = Sha256::digest(&bytes);
    let actual = digest
        .iter()
        .fold(String::with_capacity(digest.len() * 2), |mut hex, byte| {
            use std::fmt::Write as _;
            write!(hex, "{byte:02x}").expect("writing to a String never fails");
            hex
        });

    if actual.eq_ignore_ascii_case(expected_sha256) {
        Ok(())
    } else {
        Err(UpdateError::Malformed {
            reason: format!("checksum mismatch for {}", package_path.display()),
        })
    }
}

/// Fetches `url` and writes its bytes to `destination`, replacing anything
/// already there.
fn fetch_signature(url: &str, destination: &Path) -> Result<(), UpdateError> {
    let mut response = ureq::get(url).call().map_err(map_ureq_error)?;
    let bytes = response.body_mut().read_to_vec().map_err(map_ureq_error)?;
    fs::write(destination, bytes).map_err(|error| UpdateError::Io {
        reason: error.to_string(),
    })
}

/// `package_path` with `.minisig` appended to its whole name, never replacing
/// its own extension — `IdleManager-0.3.0-full.nupkg.minisig` beside
/// `IdleManager-0.3.0-full.nupkg`.
fn minisig_path_for(package_path: &Path) -> PathBuf {
    let mut with_suffix = package_path.as_os_str().to_os_string();
    with_suffix.push(".minisig");
    PathBuf::from(with_suffix)
}

/// Deletes a package that failed a checksum or a signature check, logging
/// either outcome — never silently, and never left half-verified on disk for
/// [`UpdateChannel::apply_on_exit`] to find later (code standards rule 14).
fn delete_unverified_package(package_path: &Path, reason: &UpdateError) {
    match fs::remove_file(package_path) {
        Ok(()) => {
            tracing::warn!(
                package = %package_path.display(),
                %reason,
                "deleted a package that could not be verified",
            );
        }
        Err(delete_error) => {
            tracing::warn!(
                package = %package_path.display(),
                %reason,
                %delete_error,
                "a package that could not be verified could not be deleted",
            );
        }
    }
}

/// Maps Velopack's own error into the port's, distinguishing the cases a
/// caller must tell apart (code standards rule 12): a network failure that
/// is worth a quiet retry later, the specific case of a rate limit, and a
/// feed or package that this build cannot make sense of at all.
fn map_velopack_error(error: velopack::Error) -> UpdateError {
    match error {
        velopack::Error::Network(network) => match *network {
            velopack::NetworkError::Http(ureq_error) => map_ureq_error(ureq_error),
            url_error @ velopack::NetworkError::Url(_) => UpdateError::Offline {
                reason: url_error.to_string(),
            },
        },
        velopack::Error::Io(io_error) => UpdateError::Io {
            reason: io_error.to_string(),
        },
        velopack::Error::ChecksumInvalid(path, expected, actual) => UpdateError::Malformed {
            reason: format!(
                "checksum mismatch for {}: expected {expected}, got {actual}",
                path.display()
            ),
        },
        velopack::Error::SizeInvalid(path, expected, actual) => UpdateError::Malformed {
            reason: format!(
                "size mismatch for {}: expected {expected}, got {actual}",
                path.display()
            ),
        },
        other => UpdateError::Malformed {
            reason: other.to_string(),
        },
    }
}

/// GitHub answers a rate-limited request with `403` or `429`; every other
/// failure `ureq` can raise is treated as unreachable rather than named
/// individually, since the caller's response to any of them is the same.
fn map_ureq_error(error: ureq::Error) -> UpdateError {
    match error {
        ureq::Error::StatusCode(403 | 429) => UpdateError::RateLimited,
        other => UpdateError::Offline {
            reason: other.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_github_repository_url_passes_validation() {
        let result = VelopackChannel::new("https://github.com/idle-manager/idle-manager");

        assert!(!matches!(result, Err(ChannelSetup::NotGithub { .. })));
    }

    #[test]
    fn a_github_repository_url_with_a_trailing_slash_passes_validation() {
        let result = VelopackChannel::new("https://github.com/idle-manager/idle-manager/");

        assert!(!matches!(result, Err(ChannelSetup::NotGithub { .. })));
    }

    #[test]
    fn a_non_github_url_is_rejected() {
        let result = VelopackChannel::new("https://gitlab.com/idle-manager/idle-manager");

        assert!(matches!(result, Err(ChannelSetup::NotGithub { .. })));
    }

    #[test]
    fn a_github_url_with_extra_path_segments_is_rejected() {
        let result = VelopackChannel::new("https://github.com/idle-manager/idle-manager/releases");

        assert!(matches!(result, Err(ChannelSetup::NotGithub { .. })));
    }
}
