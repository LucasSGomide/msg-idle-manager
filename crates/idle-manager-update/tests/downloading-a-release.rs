//! `VelopackChannel::download` end to end, against a local HTTP fixture
//! instead of the real GitHub (CLAUDE.md: a test that needs an update feed
//! must use a local fixture, never the real GitHub).
//!
//! The fixture's `.minisig` is `tests/fixtures/wrong-key.minisig` — the same
//! throwaway "wrong key" signature `signature-rejects-what-the-key-did-not-sign.rs`
//! uses over `package.bin` — so the download is expected to fail its
//! signature check and delete the package it fetched, exactly as
//! `signature::verify_package` does for a package already on disk.

mod common;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use common::{FixtureServer, TempDir};
use idle_manager_core::{UpdateChannel, UpdateError, UpdateInfo, Version};
use idle_manager_update::VelopackChannel;
use sha2::{Digest, Sha256};
use velopack::locator::VelopackLocatorConfig;
use velopack::sources::HttpSource;

fn fixture(name: &str) -> Vec<u8> {
    fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name),
    )
    .expect("the fixture file exists and is readable")
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;

    Sha256::digest(bytes)
        .iter()
        .fold(String::new(), |mut hex, byte| {
            write!(hex, "{byte:02x}").expect("writing to a String never fails");
            hex
        })
}

/// A `VelopackLocatorConfig` describing a fake install rooted at `dir`: an
/// empty `Update.exe` placeholder and a minimal manifest naming version
/// `0.2.0` — everything `VelopackLocator::new` checks for without this test
/// binary itself needing to be a real Velopack install.
fn fake_install(dir: &TempDir) -> VelopackLocatorConfig {
    let packages_dir = dir.join("packages");
    let current_binary_dir = dir.join("current");
    let update_exe_path = dir.join("Update.exe");
    let manifest_path = dir.join("manifest.nuspec");

    fs::create_dir_all(&packages_dir).expect("create the packages directory");
    fs::create_dir_all(&current_binary_dir).expect("create the current-binary directory");
    fs::write(&update_exe_path, []).expect("write a placeholder Update.exe");
    fs::write(
        &manifest_path,
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <manifest><id>IdleManager</id><version>0.2.0</version></manifest>",
    )
    .expect("write the fixture manifest");

    VelopackLocatorConfig {
        RootAppDir: dir.path().to_path_buf(),
        UpdateExePath: update_exe_path,
        PackagesDir: packages_dir,
        ManifestPath: manifest_path,
        CurrentBinaryDir: current_binary_dir,
        IsPortable: true,
    }
}

#[test]
fn a_download_signed_with_a_different_key_is_rejected_and_the_package_is_deleted() {
    let dir = TempDir::new("download-rejected");
    let locator_config = fake_install(&dir);

    let package_bytes = fixture("package.bin");
    let signature_bytes = fixture("wrong-key.minisig");
    let file_name = "IdleManager-0.3.0-full.nupkg";

    let feed = format!(
        r#"{{"Assets":[{{"PackageId":"IdleManager","Version":"0.3.0","Type":"Full","FileName":"{file_name}","SHA1":"","SHA256":"{checksum}","Size":{size},"NotesMarkdown":"","NotesHtml":""}}]}}"#,
        checksum = sha256_hex(&package_bytes),
        size = package_bytes.len(),
    );

    let mut routes = HashMap::new();
    routes.insert("releases.linux.json".to_string(), feed.into_bytes());
    routes.insert(file_name.to_string(), package_bytes);
    routes.insert(format!("{file_name}.minisig"), signature_bytes);

    let server = FixtureServer::start(routes);
    let source = HttpSource::new(server.base_url());
    let channel =
        VelopackChannel::over_source_for_tests(source, locator_config.clone(), &server.base_url())
            .expect("build a channel over the fixture server");

    let info = UpdateInfo {
        version: Version::new(0, 3, 0),
        notes_url: String::new(),
    };
    let result = channel.download(&info, &|_percent| {});

    assert!(
        matches!(&result, Err(UpdateError::Rejected { .. })),
        "expected Rejected, got {result:?}"
    );

    let package_path = locator_config.PackagesDir.join(file_name);
    assert!(!package_path.exists(), "a rejected package must be deleted");
}
