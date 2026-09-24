//! Verifies a downloaded package against the compiled-in minisign public key
//! before the update channel applies anything (roadmap item 16 task 04).
//!
//! A checksum proves a download arrived intact; it says nothing about who
//! produced it. [`verify_package`] is the second, independent check: the
//! package must carry a signature made with the private half of
//! [`PUBLIC_KEY`], which lives only in the project's GitHub secrets
//! (`release/README.md`). A package that fails either read or the signature
//! check itself is deleted, never handed to the installer half-verified.

use std::fs;
use std::path::{Path, PathBuf};

use minisign_verify::{PublicKey, Signature};
use thiserror::Error;

/// The project's release public key, committed at `release/minisign.pub` and
/// compiled into the binary at build time. Its matching private half signs
/// every release asset (`make release-sign`) and never itself enters this
/// repository — see `release/README.md` for how the pair was made and how it
/// rotates.
pub(crate) const PUBLIC_KEY: &str = include_str!("../../../release/minisign.pub");

/// Why [`verify_package`] refused a package.
///
/// A refusal never distinguishes "signed by an unknown key" from "signed by
/// the right key over different bytes" — both collapse into [`Rejected`],
/// because the caller's only correct response to either is the same: throw
/// the package away.
///
/// [`Rejected`]: SignatureError::Rejected
#[derive(Debug, Error)]
pub enum SignatureError {
    /// `package` or its signature file could not be read at all — missing,
    /// unreadable permissions, or similar. Carries the path that failed so
    /// the caller can say which one.
    #[error("could not read {}", path.display())]
    Unreadable {
        /// The file that could not be read.
        path: PathBuf,
    },
    /// The signature file existed but was not a well-formed minisign
    /// signature (the wrong shape, truncated, or corrupted).
    #[error("malformed signature")]
    MalformedSignature,
    /// The signature did not verify against [`PUBLIC_KEY`] for `package`'s
    /// current bytes — signed by a different key, or the bytes changed since
    /// signing.
    #[error("signature rejected")]
    Rejected,
}

/// Verifies `package`'s current bytes against `signature` and the
/// compiled-in [`PUBLIC_KEY`], deleting `package` on any failure.
///
/// `Ok(())` proves only that whoever holds the release signing key produced
/// `signature` over exactly `package`'s bytes as they are right now — it
/// says nothing about whether the package is otherwise safe to run. On
/// `Err`, `package` no longer exists on disk by the time this returns (a
/// failed deletion is logged, never silently ignored, but still reports the
/// original verification error): a caller can rely on the file being gone
/// rather than repeat the check before removing it itself.
///
/// # Errors
///
/// [`SignatureError::Unreadable`] if `package` or `signature` cannot be
/// read, [`SignatureError::MalformedSignature`] if `signature` is not a
/// well-formed minisign signature, [`SignatureError::Rejected`] if it is
/// well-formed but does not verify against [`PUBLIC_KEY`] for `package`'s
/// bytes.
pub fn verify_package(package: &Path, signature: &Path) -> Result<(), SignatureError> {
    let outcome = verify(package, signature);

    if let Err(error) = &outcome {
        match fs::remove_file(package) {
            Ok(()) => {
                tracing::warn!(
                    package = %package.display(),
                    error = %error,
                    "deleted a package that failed signature verification",
                );
            }
            Err(delete_error) => {
                tracing::warn!(
                    package = %package.display(),
                    error = %error,
                    delete_error = %delete_error,
                    "package failed signature verification and could not be deleted",
                );
            }
        }
    }

    outcome
}

fn verify(package: &Path, signature: &Path) -> Result<(), SignatureError> {
    // The committed key is checked into the repository and compiled in by
    // every build, so a decode failure here means the commit itself is
    // broken, not anything this call's arguments could cause.
    let public_key = PublicKey::decode(PUBLIC_KEY)
        .expect("release/minisign.pub is a well-formed committed minisign public key");

    let signature_text = fs::read_to_string(signature).map_err(|_| SignatureError::Unreadable {
        path: signature.to_path_buf(),
    })?;
    let signature =
        Signature::decode(&signature_text).map_err(|_| SignatureError::MalformedSignature)?;

    let bytes = fs::read(package).map_err(|_| SignatureError::Unreadable {
        path: package.to_path_buf(),
    })?;

    public_key
        .verify(&bytes, &signature, false)
        .map_err(|_| SignatureError::Rejected)
}
