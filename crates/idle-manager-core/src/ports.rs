//! The traits adapters must satisfy to serve the domain. Each is named for the
//! capability the domain needs, not the technology that provides it.

use std::path::PathBuf;

use crate::session::SessionId;

/// The data and cache directories that belong to one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileDirectories {
    /// Where the engine writes cookies, storage, databases and workers.
    pub data: PathBuf,
    /// Where the engine writes its disposable cache.
    pub cache: PathBuf,
}

/// A directory could not be made ready for a session's profile.
#[derive(Debug, thiserror::Error)]
pub enum ProfileError {
    /// The directory did not exist and could not be created.
    #[error("could not create profile directory {}", .path.display())]
    NotCreated {
        /// The directory that could not be created.
        path: PathBuf,
        /// The underlying failure.
        #[source]
        source: std::io::Error,
    },
    /// The directory exists but the process cannot write into it.
    #[error("profile directory {} exists but is not writable", .path.display())]
    NotWritable {
        /// The directory that cannot be written.
        path: PathBuf,
    },
}

/// Gives a session its private directories on disk, creating them if absent.
///
/// The domain uses this to hand the shell somewhere isolated for each account's
/// cookies and storage without knowing where on disk that is. Both returned
/// directories are guaranteed to exist. Fails if a directory cannot be created,
/// or exists but is not writable — two cases a caller must tell apart.
pub trait ProfileLocator: std::fmt::Debug {
    /// The data and cache directories for `session`, created if they do not
    /// exist. Calling twice for the same identifier returns the same two paths.
    ///
    /// # Errors
    ///
    /// [`ProfileError::NotCreated`] if a directory is absent and cannot be
    /// made, [`ProfileError::NotWritable`] if it exists but the process cannot
    /// write into it.
    fn locate(&self, session: &SessionId) -> Result<ProfileDirectories, ProfileError>;
}
