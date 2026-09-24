//! `verify_package` checks a downloaded package against the compiled-in
//! release public key (`release/minisign.pub`), which every fixture in
//! `tests/fixtures/` here is judged against. The valid signature was made
//! once with the project's real secret key (kept outside the repository —
//! see `release/README.md`); the wrong-key signature was made with a
//! throwaway key pair committed alongside it, which exists only to prove
//! that a different key is rejected.
//!
//! Every case copies its fixtures into a scratch directory first:
//! `verify_package` deletes the package it was handed on failure, and the
//! committed fixtures must survive the test suite running more than once.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::TempDir;
use idle_manager_update::{SignatureError, verify_package};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// Copies `name` out of `tests/fixtures/` into a fresh scratch directory and
/// returns both, so a test that hands the copy to `verify_package` never
/// risks deleting the committed fixture.
fn copy_into_scratch(tag: &str, name: &str) -> (TempDir, PathBuf) {
    let dir = TempDir::new(tag);
    let dest = dir.join(name);
    fs::copy(fixture(name), &dest).expect("the fixture file exists and is readable");
    (dir, dest)
}

#[test]
fn a_package_signed_with_the_release_key_verifies_and_survives() {
    let (_dir, package) = copy_into_scratch("verifies", "package.bin");
    let signature = fixture("package.bin.minisig");

    let result = verify_package(&package, &signature);

    assert!(result.is_ok(), "expected Ok(()), got {result:?}");
    assert!(package.exists(), "a verified package must not be deleted");
}

#[test]
fn a_signature_from_a_different_key_is_rejected_and_the_package_is_deleted() {
    let (_dir, package) = copy_into_scratch("wrong-key", "package.bin");
    let signature = fixture("wrong-key.minisig");

    let result = verify_package(&package, &signature);

    assert!(matches!(result, Err(SignatureError::Rejected)));
    assert!(!package.exists(), "a rejected package must be deleted");
}

#[test]
fn a_package_changed_by_one_byte_is_rejected_and_deleted() {
    let (_dir, package) = copy_into_scratch("tampered", "package-tampered.bin");
    let signature = fixture("package.bin.minisig");

    let result = verify_package(&package, &signature);

    assert!(matches!(result, Err(SignatureError::Rejected)));
    assert!(!package.exists(), "a rejected package must be deleted");
}

#[test]
fn a_missing_signature_file_is_reported_by_its_path() {
    let (dir, package) = copy_into_scratch("missing-signature", "package.bin");
    let signature = dir.join("does-not-exist.minisig");

    let result = verify_package(&package, &signature);

    match result {
        Err(SignatureError::Unreadable { path }) => assert_eq!(path, signature),
        other => panic!("expected Unreadable naming {signature:?}, got {other:?}"),
    }
}
