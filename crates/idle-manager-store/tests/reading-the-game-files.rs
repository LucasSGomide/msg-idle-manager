//! `TomlPresetCatalogue` reads a folder of hand-editable TOML files, yields the
//! ones it understood sorted by display name, turns each unusable file into one
//! named failure without taking the folder down, and reports where it looked.

mod common;

use common::TempDir;
use idle_manager_core::{PresetCatalogue, ZoomLevel};
use idle_manager_store::TomlPresetCatalogue;

const HUNTERA: &str = "\
name = \"Huntera\"
url = \"https://huntera.test/\"
zoom = 0.8
keep_awake = true
";

const BAIAKI: &str = "\
name = \"Baiaki Idle\"
url = \"https://baiaki.test/\"
zoom = 1.0
keep_awake = false
";

const LORVATH: &str = "\
name = \"Lorvath\"
url = \"https://lorvath.test/\"
zoom = 1.0
keep_awake = false
";

#[test]
fn three_well_formed_files_yield_three_presets_sorted_by_display_name() {
    let dir = TempDir::new("presets-three");
    dir.write("huntera.toml", HUNTERA);
    dir.write("baiaki-idle.toml", BAIAKI);
    dir.write("lorvath.toml", LORVATH);

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    let names: Vec<&str> = reading
        .presets
        .iter()
        .map(|preset| preset.display_name.as_str())
        .collect();
    assert_eq!(names, ["Baiaki Idle", "Huntera", "Lorvath"]);
}

#[test]
fn every_field_reaches_the_domain_preset_unchanged() {
    let dir = TempDir::new("presets-fields");
    dir.write("huntera.toml", HUNTERA);

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    let preset = &reading.presets[0];
    assert_eq!(
        (
            preset.display_name.as_str(),
            preset.start_address.as_str(),
            preset.zoom,
            preset.keep_awake_default,
        ),
        (
            "Huntera",
            "https://huntera.test/",
            ZoomLevel::new(0.8).expect("0.8 is accepted"),
            true,
        ),
    );
}

#[test]
fn a_file_with_no_user_agent_key_yields_a_preset_with_no_identity() {
    let dir = TempDir::new("presets-no-ua");
    dir.write("huntera.toml", HUNTERA);

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    assert_eq!(reading.presets[0].browser_identity, None);
}

#[test]
fn a_file_with_an_empty_user_agent_is_a_failure_not_a_preset() {
    let dir = TempDir::new("presets-empty-ua");
    dir.write(
        "huntera.toml",
        "name = \"Huntera\"\nurl = \"https://huntera.test/\"\nzoom = 1.0\nkeep_awake = false\nuser_agent = \"\"\n",
    );

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    assert_eq!(
        (reading.presets.len(), reading.failures.len()),
        (0, 1),
        "an empty user_agent is a failure, not a silently empty identity"
    );
}

#[test]
fn a_file_that_will_not_parse_is_one_failure_and_the_rest_still_read() {
    let dir = TempDir::new("presets-unparseable");
    dir.write("huntera.toml", HUNTERA);
    dir.write("broken.toml", "name = \"Broken\"\nurl = \n");

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    let failure = &reading.failures[0];
    assert_eq!(
        (
            reading.presets.len(),
            reading.failures.len(),
            failure.entry.as_str(),
            failure.reason.is_empty(),
        ),
        (1, 1, "broken.toml", false),
    );
}

#[test]
fn a_file_missing_a_required_key_is_a_failure() {
    let dir = TempDir::new("presets-missing-key");
    dir.write("huntera.toml", HUNTERA);
    dir.write(
        "no-url.toml",
        "name = \"No URL\"\nzoom = 1.0\nkeep_awake = false\n",
    );

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    assert_eq!((reading.presets.len(), reading.failures.len()), (1, 1),);
}

#[test]
fn a_file_whose_zoom_is_zero_or_negative_is_a_failure_not_a_preset() {
    let dir = TempDir::new("presets-bad-zoom");
    dir.write(
        "flat.toml",
        "name = \"Flat\"\nurl = \"https://flat.test/\"\nzoom = 0.0\nkeep_awake = false\n",
    );
    dir.write(
        "inverted.toml",
        "name = \"Inverted\"\nurl = \"https://inverted.test/\"\nzoom = -0.8\nkeep_awake = false\n",
    );

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    assert_eq!((reading.presets.len(), reading.failures.len()), (0, 2));
}

#[test]
fn a_directory_that_exists_and_is_empty_yields_nothing() {
    let dir = TempDir::new("presets-empty");

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    assert_eq!((reading.presets.len(), reading.failures.len()), (0, 0),);
}

#[test]
fn a_file_whose_extension_is_not_toml_is_neither_a_preset_nor_a_failure() {
    let dir = TempDir::new("presets-extension");
    dir.write("huntera.toml", HUNTERA);
    dir.write("notes.txt", "not a preset");
    dir.write("huntera.toml.bak", HUNTERA);

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    assert_eq!((reading.presets.len(), reading.failures.len()), (1, 0),);
}

#[test]
fn the_catalogue_reports_the_directory_it_read() {
    let dir = TempDir::new("presets-source");

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    assert_eq!(reading.source, dir.path().display().to_string());
}

#[test]
fn the_real_presets_directory_sits_under_xdg_config_not_data() {
    let dirs = directories::ProjectDirs::from("", "", "idle-manager")
        .expect("a home directory to resolve XDG paths from");
    let presets = idle_manager_store::presets_dir().expect("resolve the presets directory");

    assert!(
        presets.starts_with(dirs.config_dir()) && !presets.starts_with(dirs.data_dir()),
        "presets at {} must be under config {} and not data {}",
        presets.display(),
        dirs.config_dir().display(),
        dirs.data_dir().display(),
    );
}

#[test]
fn a_webgl_key_set_false_reads_back_disabled_and_a_file_without_the_key_reads_back_enabled() {
    let dir = TempDir::new("presets-webgl");
    dir.write(
        "off.toml",
        "name = \"Off\"\nurl = \"https://off.test/\"\nzoom = 1.0\nkeep_awake = false\nwebgl = false\n",
    );
    dir.write("huntera.toml", HUNTERA);

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    let webgl: Vec<(&str, bool)> = reading
        .presets
        .iter()
        .map(|preset| (preset.display_name.as_str(), preset.webgl_enabled))
        .collect();
    assert_eq!(webgl, [("Huntera", true), ("Off", false)]);
}

#[test]
fn a_non_boolean_webgl_key_falls_back_to_enabled_and_the_other_fields_still_load() {
    let dir = TempDir::new("presets-webgl-bad");
    dir.write(
        "weird.toml",
        "name = \"Weird\"\nurl = \"https://weird.test/\"\nzoom = 0.8\nkeep_awake = true\nwebgl = \"yes\"\n",
    );

    let reading = TomlPresetCatalogue::under(dir.path()).read();

    let preset = &reading.presets[0];
    assert_eq!(
        (
            reading.failures.len(),
            preset.webgl_enabled,
            preset.keep_awake_default,
            preset.zoom,
        ),
        (0, true, true, ZoomLevel::new(0.8).expect("0.8 is accepted")),
    );
}
