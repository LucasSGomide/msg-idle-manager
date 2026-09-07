//! `TomlPresetCatalogue` seeds the three shipped game files into the presets
//! directory the first time it is missing, and never touches the folder again.

mod common;

use std::fs;

use common::TempDir;
use idle_manager_core::PresetCatalogue;
use idle_manager_store::TomlPresetCatalogue;

const SHIPPED_FILE_NAMES: [&str; 3] = ["baiaki-idle.toml", "huntera.toml", "lorvath.toml"];

fn file_names(directory: &std::path::Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(directory)
        .expect("read the seeded directory")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn constructing_over_a_missing_path_creates_and_seeds_the_directory() {
    let temp = TempDir::new("seed-missing");
    let presets = temp.join("config/presets");

    let catalogue = TomlPresetCatalogue::under(&presets);

    assert_eq!(
        (presets.is_dir(), file_names(catalogue.directory())),
        (true, SHIPPED_FILE_NAMES.map(str::to_owned).to_vec()),
    );
}

#[test]
fn the_seeded_directory_yields_the_three_shipped_games_sorted_by_name() {
    let temp = TempDir::new("seed-read");
    let presets = temp.join("config/presets");

    let reading = TomlPresetCatalogue::under(&presets).read();

    let names: Vec<&str> = reading
        .presets
        .iter()
        .map(|preset| preset.display_name.as_str())
        .collect();
    assert_eq!(names, ["Baiaki Idle", "Huntera", "Lorvath"]);
}

#[test]
fn each_shipped_file_carries_its_address_and_no_browser_identity() {
    let temp = TempDir::new("seed-fields");
    let presets = temp.join("config/presets");

    let reading = TomlPresetCatalogue::under(&presets).read();

    let seen: Vec<(&str, &str, Option<&str>)> = reading
        .presets
        .iter()
        .map(|preset| {
            (
                preset.display_name.as_str(),
                preset.start_address.as_str(),
                preset.browser_identity.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            ("Baiaki Idle", "https://baiakidle.com/", None),
            ("Huntera", "https://huntera.com.br/", None),
            ("Lorvath", "https://lorvath.com/", None),
        ],
    );
}

#[test]
fn a_second_construction_writes_nothing_over_a_hand_edited_folder() {
    let temp = TempDir::new("seed-second");
    let presets = temp.join("config/presets");
    let _first = TomlPresetCatalogue::under(&presets);

    fs::write(
        presets.join("huntera.toml"),
        "name = \"Huntera (mine)\"\nurl = \"https://huntera.com.br/\"\nzoom = 0.8\nkeep_awake = true\n",
    )
    .expect("hand-edit a seeded file");
    fs::remove_file(presets.join("lorvath.toml")).expect("hand-delete a seeded file");

    let reading = TomlPresetCatalogue::under(&presets).read();

    let names: Vec<&str> = reading
        .presets
        .iter()
        .map(|preset| preset.display_name.as_str())
        .collect();
    assert_eq!(
        names,
        ["Baiaki Idle", "Huntera (mine)"],
        "the edit is kept and the deletion stays; nothing is put back"
    );
}

#[test]
fn a_directory_that_already_exists_but_is_empty_is_left_empty() {
    let temp = TempDir::new("seed-empty");

    let reading = TomlPresetCatalogue::under(temp.path()).read();

    assert_eq!(
        (file_names(temp.path()).len(), reading.presets.len()),
        (0, 0),
    );
}

#[test]
fn one_shipped_file_matches_its_committed_snapshot_exactly() {
    let temp = TempDir::new("seed-snapshot");
    let presets = temp.join("config/presets");
    let _seeded = TomlPresetCatalogue::under(&presets);

    let on_disk = fs::read_to_string(presets.join("huntera.toml")).expect("read the seeded file");

    insta::assert_snapshot!("shipped-huntera-toml", on_disk);
}

#[test]
#[cfg(unix)]
fn a_presets_directory_that_cannot_be_created_leaves_an_empty_catalogue() {
    let temp = TempDir::new("seed-readonly");
    temp.make_readonly();

    let reading = TomlPresetCatalogue::under(temp.join("presets")).read();

    assert_eq!(
        (reading.presets.len(), reading.failures.len()),
        (0, 0),
        "construction succeeds and the catalogue is simply empty"
    );
}
