//! The traits adapters must satisfy to serve the domain. Each is named for the
//! capability the domain needs, not the technology that provides it.

use std::path::PathBuf;

use crate::memory::MemoryReading;
use crate::preset::Preset;
use crate::remote::{EnrolledPhone, EnrolmentOffer, Frame, PhoneStatus, RemoteState};
use crate::session::{RememberedZoom, SessionId};
use crate::update::Version;
use crate::workspace::WorkspaceList;

/// A memory sample could not be taken.
///
/// Two cases a caller must tell apart, because the shell shows the same
/// unavailable footer for both but logs a different reason (code standards
/// rule 15): the system refused to let us look, or it showed us something we
/// could not read. A crate that flattened them into one error would make the
/// log useless at the one moment it is needed.
#[derive(Debug, Clone, thiserror::Error)]
pub enum MemoryProbeError {
    /// The measurement was refused — a file that is not there, or a permission
    /// denied. `reason` is one line ready to log.
    #[error("the memory measurement was refused: {reason}")]
    Refused {
        /// One line describing what was refused.
        reason: String,
    },
    /// The kernel produced output the probe could not parse. `reason` is one
    /// line ready to log.
    #[error("the memory measurement could not be read: {reason}")]
    Unreadable {
        /// One line describing what would not parse.
        reason: String,
    },
}

/// Someone who can be asked what the application currently costs in memory.
///
/// Named for the capability, not the technology (naming rule 10): the core asks
/// for a `MemoryProbe` and `idle-manager-metrics` supplies a `ProcPssProbe`
/// that reads `/proc`. Rule 1 is why the port exists at all —
/// `scripts/arch-check.sh` forbids the core from reading `/proc` — and rule 6
/// is met twice over, because the footer's formatting and its verdict both need
/// a test that does not depend on a machine having games running.
///
/// `Send + Sync` so the shell can hand a sample to a worker thread and keep the
/// GTK main context free while `/proc` is read for every process on the machine
/// (architecture rule 10).
pub trait MemoryProbe: std::fmt::Debug + Send + Sync {
    /// Takes a reading now: the application's own figure, its descendants', and
    /// the count of processes that contributed.
    ///
    /// # Errors
    ///
    /// [`MemoryProbeError::Refused`] if the system would not let the probe
    /// look, [`MemoryProbeError::Unreadable`] if it looked and could not parse
    /// what it saw.
    fn sample(&self) -> Result<MemoryReading, MemoryProbeError>;
}

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

/// The saved workspace list could not be read.
///
/// Three-way at the call site: [`WorkspaceStore::read`] returns `Ok(None)` for
/// a first run with no file, `Ok(Some(_))` for a workspace list, and this for
/// a failure — two cases a caller must tell apart, because one keeps the bad
/// bytes aside to name to the user and the other has nothing to show.
#[derive(Debug, thiserror::Error)]
pub enum WorkspaceReadError {
    /// A file was there and would not parse — bad TOML, a value the domain
    /// cannot represent, or a version this build does not understand. The
    /// original bytes have been moved to `kept` and never overwritten, so a
    /// caller can name that path and open as a first run.
    #[error("the saved workspace was unreadable and kept at {}: {reason}", .kept.display())]
    Unreadable {
        /// Where the unreadable file was moved, for naming to the user.
        kept: PathBuf,
        /// One line describing what was wrong, ready to show as-is.
        reason: String,
    },
    /// The workspace location could not be reached at all — a permission or I/O
    /// failure with no single file to set aside.
    #[error("the workspace location could not be read: {reason}")]
    Inaccessible {
        /// One line describing what was wrong, ready to show as-is.
        reason: String,
    },
}

/// The workspace list could not be written.
#[derive(Debug, thiserror::Error)]
#[error("the workspace could not be saved: {reason}")]
pub struct WorkspaceWriteError {
    /// One line describing what was wrong, ready to show as-is.
    pub reason: String,
}

/// Saves the whole arrangement — every workspace — and gives back the one it
/// saved, without the domain knowing a file is involved.
///
/// Named for the capability, not the technology behind it (naming rule 10) and
/// implemented outside the core (architecture rules 5, 6). The read is
/// three-way — no file, a workspace list, or a failure — and the write is
/// atomic from the caller's point of view: it either replaces the saved list
/// entirely or leaves the previous one intact.
///
/// `Send + Sync` so the shell can hand a write to a worker thread and keep the
/// GTK main context free while the disk blocks (architecture rule 10).
pub trait WorkspaceStore: std::fmt::Debug + Send + Sync {
    /// Reads the saved workspace list now.
    ///
    /// # Errors
    ///
    /// [`WorkspaceReadError::Unreadable`] if a file is present but will not
    /// parse — the bytes are kept aside at the path the error carries;
    /// [`WorkspaceReadError::Inaccessible`] if the location cannot be read.
    /// A first run with no file is `Ok(None)`, not an error.
    fn read(&self) -> Result<Option<WorkspaceList>, WorkspaceReadError>;

    /// Writes `workspaces`, replacing any previously saved list.
    ///
    /// # Errors
    ///
    /// [`WorkspaceWriteError`] if the write could not be completed; the
    /// previously saved list is left intact in that case.
    fn write(&self, workspaces: &WorkspaceList) -> Result<(), WorkspaceWriteError>;
}

/// One preset entry the catalogue could not turn into a [`Preset`].
///
/// Carried separately from the readable presets so the dialog can show a
/// partial catalogue plus a line about the bad entry, rather than nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetFailure {
    /// The entry that failed, named the way the user would recognise it — the
    /// file's own name, without its directory.
    pub entry: String,
    /// One line saying what was wrong with it, ready to show as-is.
    pub reason: String,
}

/// Everything one read of the catalogue produced: the games it understood, the
/// entries it could not read, and a plain description of where it looked.
///
/// Three parts rather than one because a screen showing a partial list plus a
/// line about the bad entry beats a screen showing nothing, and because the
/// screen has to be able to name the folder when the list is empty without
/// knowing the entries were ever files (architecture rule 3).
#[derive(Debug, Clone, PartialEq)]
pub struct PresetCatalogueReading {
    /// The presets that parsed, sorted by display name.
    pub presets: Vec<Preset>,
    /// The entries that did not, each with a one-line reason.
    pub failures: Vec<PresetFailure>,
    /// A human-readable description of where the entries came from, shown when
    /// `presets` is empty.
    pub source: String,
}

/// Storing an account's remembered zoom failed.
///
/// An enum rather than a struct so a later distinction a caller might act on —
/// a permission failure separate from a disk-full one, say — is one more
/// variant, not a new type (architecture rule 11, code standards rule 12). The
/// caller only logs it: losing a remembered size is a small annoyance and
/// refusing to run is not (`FR.12.5`).
#[derive(Debug, thiserror::Error)]
pub enum ZoomMemoryError {
    /// The write did not complete. `reason` is one line ready to log.
    #[error("the remembered zoom could not be stored: {reason}")]
    NotStored {
        /// One line describing what was wrong.
        reason: String,
    },
}

/// Reads and stores an account's chosen zoom sizes, without the domain knowing
/// a file is involved.
///
/// Named for the capability, not the technology (naming rule 10), and
/// implemented outside the core (architecture rules 5, 6) — `scripts/arch-check.sh`
/// forbids `serde` and `toml` in the core, which is why this port exists at all
/// (`FR.12.8`).
pub trait ZoomMemory: std::fmt::Debug {
    /// The sizes `account`'s owner has chosen, per arrangement.
    ///
    /// Never fails: a missing store is the normal state of a fresh account and
    /// an unreadable one is no worse — both are an empty map (`FR.12.6`). A
    /// single nonsensical entry is dropped with a log line while its siblings
    /// still apply.
    fn read(&self, account: &SessionId) -> RememberedZoom;

    /// Stores `remembered` as `account`'s chosen sizes, replacing what was
    /// there. An empty map leaves nothing remembered.
    ///
    /// # Errors
    ///
    /// [`ZoomMemoryError::NotStored`] if the write could not be completed.
    fn write(
        &self,
        account: &SessionId,
        remembered: &RememberedZoom,
    ) -> Result<(), ZoomMemoryError>;
}

/// An account's profile folder could not be removed.
#[derive(Debug, thiserror::Error)]
#[error("could not remove the account's data: {reason}")]
pub struct ProfileRemovalError {
    /// One line describing what was wrong, ready to show as-is.
    pub reason: String,
}

/// Removes exactly one account's profile folder, without the domain knowing a
/// filesystem is involved.
///
/// Named for the capability, not the technology (naming rule 10) and
/// implemented outside the core (architecture rules 5, 6) — the domain's
/// deletion flow must not know about the filesystem, and a test needs a fake.
/// There is no open-file check: the engine never releases a deleted session's
/// cookie and HSTS files while the application runs, so there is nothing to
/// wait for (roadmap item 11, Technical References).
///
/// `Send + Sync` so the shell can call it through `gio::spawn_blocking`
/// (architecture rule 10).
pub trait ProfileRemoval: std::fmt::Debug + Send + Sync {
    /// Removes `id`'s whole profile folder, together with everything in it.
    /// A folder that does not exist counts as success, so a retry after a
    /// partial removal can still finish (`FR.21.11`). Removes exactly that one
    /// folder and never a parent or sibling (`FR.21.8`).
    ///
    /// # Errors
    ///
    /// [`ProfileRemovalError`] if the folder exists but could not be removed.
    fn remove(&self, id: &SessionId) -> Result<(), ProfileRemovalError>;

    /// Where `id`'s profile folder lives, for naming to the user when
    /// [`ProfileRemoval::remove`] fails.
    fn folder(&self, id: &SessionId) -> std::path::PathBuf;
}

/// Tells the domain which games it knows about, without the domain knowing the
/// list comes from files on disk.
///
/// Named for the capability, not the technology behind it (naming rule 10) and
/// implemented outside the core (architecture rules 5, 6). [`PresetCatalogue::read`]
/// never fails as a whole: a folder that cannot be read is an empty reading and
/// a single bad entry is one [`PresetFailure`], so the caller always has
/// something to show.
pub trait PresetCatalogue: std::fmt::Debug {
    /// Reads the catalogue now.
    ///
    /// Called afresh each time the add-game dialog opens, so an entry added by
    /// hand appears on the next open with no restart. The returned presets are
    /// sorted by display name.
    fn read(&self) -> PresetCatalogueReading;
}

/// The enrolled phone's record could not be read or written.
///
/// The [`WorkspaceReadError`] shape: [`PhoneRecordStore::read`] is three-way —
/// `Ok(None)` when no phone is enrolled, `Ok(Some(_))` when one is, and this
/// for a failure — with two cases a caller must tell apart, because one has a
/// file set aside to name and the other has nothing to show.
#[derive(Debug, thiserror::Error)]
pub enum PhoneRecordError {
    /// A record was there and would not parse. The original bytes have been
    /// moved to `kept` and never overwritten, so a caller can name that path
    /// and carry on as if no phone were enrolled.
    #[error("the phone record was unreadable and kept at {}: {reason}", .kept.display())]
    Unreadable {
        /// Where the unreadable file was moved, for naming to the user.
        kept: PathBuf,
        /// One line describing what was wrong, ready to show as-is.
        reason: String,
    },
    /// The record's location could not be reached at all — a permission or
    /// I/O failure on a read, a write or a removal.
    #[error("the phone record could not be reached: {reason}")]
    Inaccessible {
        /// One line describing what was wrong, ready to show as-is.
        reason: String,
    },
}

/// The desktop's side of the link to its phone: what the shell tells the
/// server and what it asks of it.
///
/// Named for the capability, not the technology (naming rule 10): the shell
/// holds a `PhoneLink` and `idle-manager-remote` supplies the handle of a
/// server it runs on its own threads. Rule 1 is why the port exists — the
/// core knows no socket — and rule 6 is met because the window's tests need a
/// link that records what was published without listening on anything.
/// Intents travel the other way, as values on a channel the server hands out
/// when it starts, so nothing here takes a callback.
///
/// `Send + Sync` because the shell shares it with the thread that owns the
/// socket, and every method is `&self` for the same reason (architecture rule
/// 10).
pub trait PhoneLink: std::fmt::Debug + Send + Sync {
    /// Tells the phone the desktop's current state. Cheap to call on every
    /// change; nothing is sent while no phone is attached.
    fn publish_state(&self, state: &RemoteState);

    /// Offers a captured picture of the watched page. Takes the frame by
    /// value so the bytes are encoded and sent off the caller's thread; a
    /// frame offered while none is wanted is dropped.
    fn publish_frame(&self, frame: Frame);

    /// Mints a one-time enrolment code and returns the address to open on the
    /// phone together with how long the offer stands. Replaces any earlier
    /// offer still open.
    fn begin_enrolment(&self) -> EnrolmentOffer;

    /// Withdraws the open enrolment offer, if any, so its code is refused from
    /// now on.
    fn cancel_enrolment(&self);

    /// Forgets the enrolled phone: clears its record, tells it goodbye if it
    /// is connected, and closes its socket (`FR.6.2`).
    fn revoke_phone(&self);

    /// What the desktop can say about its phone right now.
    fn phone_status(&self) -> PhoneStatus;

    /// The address the enrolled phone opens the page at from then on —
    /// the desktop's listening address with the phone's device id in the
    /// query, so it works from a browser that dropped or never shared the
    /// cookie, such as a home-screen web app with a jar of its own. `None`
    /// while no phone is enrolled or the desktop is not listening. The id
    /// names the phone; it never proves it — the socket still demands the
    /// secret's proof.
    fn phone_address(&self) -> Option<String>;
}

/// Keeps the one enrolled phone between runs, without the domain or the server
/// knowing a file is involved.
///
/// Named for the capability, not the technology (naming rule 10) and
/// implemented outside the core (architecture rules 5, 6): `idle-manager-store`
/// supplies a `TomlPhoneRecord`, and the server's tests supply one that keeps
/// the record in memory. There is at most one phone: a write replaces whatever
/// was enrolled before (`FR.6.1`).
///
/// `Send + Sync` because the server calls it from its connection threads.
pub trait PhoneRecordStore: std::fmt::Debug + Send + Sync {
    /// The enrolled phone, or `Ok(None)` when none is.
    ///
    /// # Errors
    ///
    /// [`PhoneRecordError::Unreadable`] if a record is present but will not
    /// parse — the bytes are kept aside at the path the error carries;
    /// [`PhoneRecordError::Inaccessible`] if the location cannot be read.
    fn read(&self) -> Result<Option<EnrolledPhone>, PhoneRecordError>;

    /// Makes `phone` the enrolled phone, replacing any earlier one. The record
    /// is readable only by the user who owns it.
    ///
    /// # Errors
    ///
    /// [`PhoneRecordError::Inaccessible`] if the record could not be written;
    /// the earlier record, if any, is left intact in that case.
    fn write(&self, phone: &EnrolledPhone) -> Result<(), PhoneRecordError>;

    /// Forgets the enrolled phone, so a following [`PhoneRecordStore::read`]
    /// is `Ok(None)`. No phone enrolled counts as success.
    ///
    /// # Errors
    ///
    /// [`PhoneRecordError::Inaccessible`] if the record exists and could not
    /// be removed.
    fn clear(&self) -> Result<(), PhoneRecordError>;
}

/// The version on offer, and where to read what changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    /// The version on offer.
    pub version: Version,
    /// Where "What's new" opens.
    pub notes_url: String,
}

/// What [`UpdateChannel::check`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateCheck {
    /// Nothing newer than the version already running.
    UpToDate,
    /// A newer version is on offer.
    Available(UpdateInfo),
}

/// A downloaded package that has passed its checksum and signature checks,
/// staged where [`UpdateChannel::apply_on_exit`] expects to find it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPackage {
    /// The version this package installs.
    pub version: Version,
    /// Where the verified package sits on disk.
    pub path: PathBuf,
}

/// A step of learning about, fetching or applying an update did not
/// succeed.
///
/// Five cases a caller must tell apart (code standards rule 12): the two
/// that mean "try again later without saying much" ([`Offline`],
/// [`RateLimited`]), the one that means the feed answered with something
/// this build does not understand ([`Malformed`]), the one that means a
/// downloaded package must never be installed ([`Rejected`]), and a plain
/// I/O failure that fits none of the others.
///
/// [`Offline`]: UpdateError::Offline
/// [`RateLimited`]: UpdateError::RateLimited
/// [`Malformed`]: UpdateError::Malformed
/// [`Rejected`]: UpdateError::Rejected
#[derive(Debug, thiserror::Error)]
pub enum UpdateError {
    /// The update feed could not be reached at all.
    #[error("could not reach the update feed: {reason}")]
    Offline {
        /// One line describing what was wrong, ready to show as-is.
        reason: String,
    },
    /// The update feed refused the request for having asked too often.
    #[error("the update feed is rate limited; try again later")]
    RateLimited,
    /// The feed answered, but not with something this build understands.
    #[error("the update feed could not be read: {reason}")]
    Malformed {
        /// One line describing what was wrong, ready to show as-is.
        reason: String,
    },
    /// A downloaded package failed its checksum or its signature check, and
    /// has been deleted.
    #[error("the update could not be verified and was discarded: {reason}")]
    Rejected {
        /// One line describing what was wrong, ready to show as-is.
        reason: String,
    },
    /// A local file operation failed — writing, renaming or removing a
    /// package.
    #[error("the update could not be completed: {reason}")]
    Io {
        /// One line describing what was wrong, ready to show as-is.
        reason: String,
    },
}

/// Everything the domain needs from Velopack, without knowing Velopack
/// exists (naming rule 10): the core asks for an `UpdateChannel` and
/// `idle-manager-update` supplies a `VelopackChannel` built over Velopack's
/// own `UpdateManager` (architecture rules 5, 6, 9).
///
/// Every method is blocking and meant to run on a worker thread — the shell
/// is the one that keeps the GTK main context free while it does
/// (architecture rule 10). `Send + Sync` for the same reason.
pub trait UpdateChannel: std::fmt::Debug + Send + Sync {
    /// Asks whether a newer version than the one running now exists.
    ///
    /// # Errors
    ///
    /// [`UpdateError::Offline`] or [`UpdateError::RateLimited`] if the feed
    /// could not be reached or refused the request; [`UpdateError::Malformed`]
    /// if it answered with something this build cannot read.
    fn check(&self) -> Result<UpdateCheck, UpdateError>;

    /// Downloads the package `info` names, reporting `0..=100` to `progress`
    /// as it arrives, then checks its checksum and its signature before
    /// returning it. A package that fails either check is deleted before
    /// this returns.
    ///
    /// # Errors
    ///
    /// [`UpdateError::Offline`] if the download could not complete;
    /// [`UpdateError::Rejected`] if the package failed its checksum or its
    /// signature; [`UpdateError::Io`] if the file could not be written.
    fn download(
        &self,
        info: &UpdateInfo,
        progress: &(dyn Fn(u8) + Send),
    ) -> Result<VerifiedPackage, UpdateError>;

    /// Arranges for `package` to be installed once this process exits, and
    /// to relaunch the application afterwards when `relaunch` is true.
    ///
    /// # Errors
    ///
    /// [`UpdateError::Io`] if the install could not be arranged.
    fn apply_on_exit(&self, package: &VerifiedPackage, relaunch: bool) -> Result<(), UpdateError>;

    /// The version currently running.
    fn current_version(&self) -> Version;

    /// The reason a previously staged swap did not apply, read once at
    /// launch. `None` when the last swap applied cleanly or none was
    /// pending.
    fn last_apply_failure(&self) -> Option<String>;
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::*;

    #[derive(Debug)]
    struct FakeProfileRemoval;

    impl ProfileRemoval for FakeProfileRemoval {
        fn remove(&self, _id: &SessionId) -> Result<(), ProfileRemovalError> {
            Ok(())
        }

        fn folder(&self, id: &SessionId) -> PathBuf {
            PathBuf::from(id.as_str())
        }
    }

    #[test]
    fn an_arc_dyn_profile_removal_can_be_moved_into_a_thread_and_called_there() {
        let removal: Arc<dyn ProfileRemoval> = Arc::new(FakeProfileRemoval);

        let handle = std::thread::spawn(move || removal.remove(&SessionId::new("session-0001")));

        assert!(handle.join().expect("the thread did not panic").is_ok());
    }
}
