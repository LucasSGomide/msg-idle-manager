//! `TomlPhoneRecord`: the [`PhoneRecordStore`] port backed by one small TOML
//! file, `<XDG config>/idle-manager/phone.toml`, readable by the user alone.
//!
//! The on-disk shape is [`PhoneFile`], its own serde record mapped to and from
//! the domain [`EnrolledPhone`] (architecture rule 7), with the secret written
//! as hex so the file stays a text file a user can open. The same file carries
//! the hand-editable `[listen]` table, which is configuration rather than a
//! record and is therefore read by an inherent method, never through the port.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use idle_manager_core::{EnrolledPhone, PhoneRecordError, PhoneRecordStore};
use serde::{Deserialize, Serialize};

use crate::paths::{self, LocatorSetup};

/// The file's name under the configuration directory.
const PHONE_FILE: &str = "phone.toml";

/// The name a malformed file is moved aside under, beside the original.
const UNREADABLE_FILE: &str = "phone.toml.unreadable";

/// The `[listen] address` value that means "discover the address", the same
/// as leaving the key out.
const LISTEN_AUTO: &str = "auto";

/// The mode the file is created with on Unix: read and write for the owner,
/// nothing for anyone else, because it holds the phone's secret.
#[cfg(unix)]
const OWNER_ONLY_MODE: u32 = 0o600;

/// The on-disk shape of the whole file.
///
/// Its own serde type, deliberately separate from the domain (architecture
/// rule 7). Both tables are optional: a file holding only `[listen]` is a
/// user's override with no phone enrolled yet, and a file holding only
/// `[phone]` is the common case.
#[derive(Debug, Default, Serialize, Deserialize)]
struct PhoneFile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    phone: Option<PhoneRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    listen: Option<ListenRecord>,
}

/// The enrolled phone as the file spells it.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PhoneRecord {
    /// The identifier the phone presents in its cookie.
    device_id: String,
    /// The shared secret, lowercase hex.
    secret: String,
    /// When the phone was enrolled, written as the server produced it.
    enrolled_on: String,
}

/// The hand-editable `[listen]` table.
#[derive(Debug, Serialize, Deserialize)]
struct ListenRecord {
    /// `"auto"` or `"<ip>:<port>"`; absent means `"auto"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    address: Option<String>,
}

/// Reads and writes `phone.toml`.
///
/// Built with [`TomlPhoneRecord::new`] against the real config location, or
/// [`TomlPhoneRecord::under`] against an explicit directory for tests — the
/// same pair of constructors [`crate::TomlWorkspaceStore`] has. A write goes
/// to a temporary file created with the final mode and is renamed over the
/// target, so a crash mid-write leaves the previous record intact and the
/// secret is never, even briefly, world-readable.
#[derive(Debug, Clone)]
pub struct TomlPhoneRecord {
    path: PathBuf,
}

impl TomlPhoneRecord {
    /// Resolves the phone file from the environment —
    /// `<XDG config>/idle-manager/phone.toml`.
    ///
    /// # Errors
    ///
    /// [`LocatorSetup::NoHome`] if no home directory can be determined.
    pub fn new() -> Result<Self, LocatorSetup> {
        Ok(Self {
            path: paths::phone_file()?,
        })
    }

    /// Roots the store at an explicit directory, for tests that must not touch
    /// the real config location. The file is `phone.toml` inside it.
    #[must_use]
    pub fn under(directory: impl AsRef<Path>) -> Self {
        Self {
            path: directory.as_ref().join(PHONE_FILE),
        }
    }

    /// The file this store reads and writes.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The `[listen] address` override, if the user wrote one: `Some("<ip>:<port>")`
    /// for an explicit address, `None` when the key is absent or `"auto"`.
    ///
    /// Never fails and never moves the file aside: a file that will not parse
    /// is logged and read as no override, because this is configuration the
    /// server can do without, while [`PhoneRecordStore::read`] on the same
    /// file still reports the damage where it matters.
    #[must_use]
    pub fn listen_override(&self) -> Option<String> {
        let file = match self.load() {
            Ok(file) => file?,
            Err(error) => {
                tracing::warn!(path = %self.path.display(), %error, "could not read the listen override");
                return None;
            }
        };
        file.listen?
            .address
            .filter(|address| address != LISTEN_AUTO)
    }

    /// The parsed file, `Ok(None)` when there is none.
    fn load(&self) -> Result<Option<PhoneFile>, LoadError> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(source) if source.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => return Err(LoadError::Unreachable(source)),
        };
        toml::from_str(&text)
            .map(Some)
            .map_err(|error| LoadError::Malformed(error.to_string()))
    }

    /// Writes `file` atomically, creating the configuration directory first.
    fn save(&self, file: &PhoneFile) -> Result<(), PhoneRecordError> {
        let directory = self
            .path
            .parent()
            .expect("a store path is a file name joined onto a directory");
        fs::create_dir_all(directory).map_err(|source| inaccessible(directory, &source))?;

        let body = toml::to_string(file).map_err(|error| PhoneRecordError::Inaccessible {
            reason: error.to_string(),
        })?;

        let temporary = directory.join(format!("{PHONE_FILE}.{}.tmp", std::process::id()));
        write_owner_only(&temporary, &body).map_err(|source| inaccessible(&temporary, &source))?;
        fs::rename(&temporary, &self.path).map_err(|source| {
            if let Err(error) = fs::remove_file(&temporary) {
                tracing::warn!(path = %temporary.display(), %error, "could not remove the phone temporary file");
            }
            inaccessible(&self.path, &source)
        })
    }

    /// Moves the unreadable file aside to [`UNREADABLE_FILE`] — or, when that
    /// name is already taken, the first free numbered variant of it — so its
    /// bytes are never destroyed and never overwritten by the next write.
    /// Returns where it went even if the move itself failed.
    fn quarantine(&self) -> PathBuf {
        let base = self.path.with_file_name(UNREADABLE_FILE);
        let mut kept = base.clone();
        let mut serial = 1;
        while kept.exists() {
            kept = PathBuf::from(format!("{}.{serial}", base.display()));
            serial += 1;
        }

        if let Err(error) = fs::rename(&self.path, &kept) {
            tracing::warn!(
                from = %self.path.display(), to = %kept.display(), %error,
                "could not set the unreadable phone file aside"
            );
        }
        kept
    }
}

impl PhoneRecordStore for TomlPhoneRecord {
    fn read(&self) -> Result<Option<EnrolledPhone>, PhoneRecordError> {
        let Some(file) = self.load().map_err(|error| self.read_error(error))? else {
            return Ok(None);
        };
        let Some(record) = file.phone else {
            return Ok(None);
        };
        let secret = decode_hex(&record.secret).ok_or_else(|| {
            self.read_error(LoadError::Malformed(
                "the secret under [phone] is not hex".to_owned(),
            ))
        })?;
        Ok(Some(EnrolledPhone {
            device_id: record.device_id,
            secret,
            enrolled_on: record.enrolled_on,
        }))
    }

    fn write(&self, phone: &EnrolledPhone) -> Result<(), PhoneRecordError> {
        let mut file = self.load().ok().flatten().unwrap_or_default();
        file.phone = Some(PhoneRecord {
            device_id: phone.device_id.clone(),
            secret: encode_hex(&phone.secret),
            enrolled_on: phone.enrolled_on.clone(),
        });
        self.save(&file)
    }

    fn clear(&self) -> Result<(), PhoneRecordError> {
        let mut file = self.load().ok().flatten().unwrap_or_default();
        file.phone = None;
        if file.listen.is_some() {
            return self.save(&file);
        }
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(inaccessible(&self.path, &source)),
        }
    }
}

impl TomlPhoneRecord {
    /// Maps a load failure onto the port's error, setting a malformed file
    /// aside as it does so.
    fn read_error(&self, error: LoadError) -> PhoneRecordError {
        match error {
            LoadError::Unreachable(source) => inaccessible(&self.path, &source),
            LoadError::Malformed(reason) => PhoneRecordError::Unreadable {
                kept: self.quarantine(),
                reason,
            },
        }
    }
}

/// Why the file could not be loaded — private, because the port's own error
/// is what leaves the crate.
#[derive(Debug, thiserror::Error)]
enum LoadError {
    #[error("{0}")]
    Unreachable(io::Error),
    #[error("{0}")]
    Malformed(String),
}

fn inaccessible(path: &Path, source: &io::Error) -> PhoneRecordError {
    PhoneRecordError::Inaccessible {
        reason: format!("{}: {source}", path.display()),
    }
}

/// Creates `path` afresh — it must not exist — with the owner-only mode on
/// Unix, and writes `body` into it. On Windows the file inherits its folder's
/// permissions, which under a user's own profile already exclude other users.
fn write_owner_only(path: &Path, body: &str) -> io::Result<()> {
    use std::io::Write;

    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(OWNER_ONLY_MODE);
    }
    let mut file = options.open(path)?;
    file.write_all(body.as_bytes())?;
    file.sync_all()
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;

    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut text, byte| {
            // Writing into a `String` cannot fail; the `Result` is the trait's.
            let _ = write!(text, "{byte:02x}");
            text
        })
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) || !text.is_ascii() {
        return None;
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_secret_round_trips_through_lowercase_hex() {
        let secret = vec![0x00, 0x0f, 0xab, 0xff];

        let encoded = encode_hex(&secret);

        assert_eq!(
            (encoded.as_str(), decode_hex(&encoded)),
            ("000fabff", Some(secret))
        );
    }

    #[test]
    fn hex_with_an_odd_length_or_a_non_hex_digit_does_not_decode() {
        assert_eq!((decode_hex("abc"), decode_hex("zz")), (None, None));
    }
}
