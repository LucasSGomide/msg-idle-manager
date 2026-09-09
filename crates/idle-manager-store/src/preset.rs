//! `TomlPresetCatalogue`: the [`PresetCatalogue`] port backed by a folder of
//! hand-editable TOML files, one per game.
//!
//! The on-disk shape is [`PresetFile`], its own serde type mapped to and from
//! the domain [`Preset`] (architecture rule 7). The folder is a contract with
//! users invited to edit it by hand: a domain rename must never turn into a
//! broken file on somebody's disk, and the format will grow fields the domain
//! has no interest in.

use std::fs;
use std::path::{Path, PathBuf};

use idle_manager_core::{
    InvalidZoom, Preset, PresetCatalogue, PresetCatalogueReading, PresetFailure, PresetId,
    ZoomLevel,
};
use serde::Deserialize;

use crate::paths::{self, LocatorSetup};

/// The file extension a preset file must carry to be read at all.
const PRESET_EXTENSION: &str = "toml";

/// What a preset's WebGL setting is when its file leaves the key out, or sets
/// it to something that is not a boolean (`FR.19.7`): enabled, so no existing
/// game file changes behaviour when the key lands.
const WEBGL_DEFAULT_ENABLED: bool = true;

/// The game files carried in the binary and written into the presets directory
/// the first time it is found missing.
///
/// `include_str!` for the same reasons `web_view.rs` embeds its scripts: this is
/// their only reader, the read stays infallible, and nothing has to be installed
/// beside the binary (code standards rule 18). After the first run the folder is
/// the user's — these are never written again.
const SHIPPED_PRESETS: [(&str, &str); 3] = [
    (
        "huntera.toml",
        include_str!("../../../presets/huntera.toml"),
    ),
    (
        "baiaki-idle.toml",
        include_str!("../../../presets/baiaki-idle.toml"),
    ),
    (
        "lorvath.toml",
        include_str!("../../../presets/lorvath.toml"),
    ),
];

/// One reason a preset file could not be turned into a [`Preset`].
///
/// A `thiserror` enum rather than a string so a caller can tell the cases apart
/// (code standards rule 12); the catalogue flattens it to a one-line reason for
/// the shell, which only displays it.
#[derive(Debug, thiserror::Error)]
pub enum PresetFileError {
    /// The file could not be read from disk.
    #[error("could not read the file: {0}")]
    Unreadable(#[source] std::io::Error),
    /// The file is not valid TOML, or is missing a key the format requires.
    #[error("{0}")]
    Malformed(#[from] toml::de::Error),
    /// The `user_agent` key is present but empty. Absent means the engine's own
    /// identity; empty is almost always a mistake, and a silently empty
    /// identity would be worse than saying so (`FR.10.5`).
    #[error("the user_agent key is present but empty; remove it for the engine's own identity")]
    EmptyUserAgent,
    /// The `zoom` value is not a usable multiplier.
    #[error(transparent)]
    Zoom(#[from] InvalidZoom),
}

/// The on-disk shape of one preset file.
///
/// Deliberately separate from the domain [`Preset`] even though the two
/// currently carry the same five things (architecture rule 7). Unknown keys are
/// rejected so a mistyped key is a failure entry the user sees, not a value
/// silently ignored.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct PresetFile {
    /// The name shown in the chooser.
    name: String,
    /// The address a new account loads first.
    url: String,
    /// A plain zoom multiplier — `zoom = 0.8` — exactly what the engine takes.
    zoom: f64,
    /// The value a new account's keep-awake flag starts at.
    keep_awake: bool,
    /// The identity to present to this game. Absent means the engine's own
    /// (`FR.10.5`).
    #[serde(default)]
    user_agent: Option<String>,
    /// Whether this game's pages get a WebGL context (`FR.19.7`). Held as a raw
    /// value rather than a `bool` so a non-boolean is tolerated — it falls back
    /// to the default with a warning naming the file, and the rest of the
    /// preset still loads, rather than taking the whole file down the way a bad
    /// `zoom` does.
    #[serde(default)]
    webgl: Option<toml::Value>,
}

/// Reads `file`'s `webgl` value into a domain boolean, defaulting a missing or
/// non-boolean value to [`WEBGL_DEFAULT_ENABLED`] and warning in the latter
/// case with `entry` — the file's own name — for the user to find.
fn webgl_enabled(value: &Option<toml::Value>, entry: &str) -> bool {
    match value {
        None => WEBGL_DEFAULT_ENABLED,
        Some(toml::Value::Boolean(enabled)) => *enabled,
        Some(other) => {
            tracing::warn!(
                entry,
                value = %other,
                "the webgl key is not true or false; defaulting it to enabled"
            );
            WEBGL_DEFAULT_ENABLED
        }
    }
}

/// Reads presets from a folder of `*.toml` files, one per game.
///
/// Built with [`TomlPresetCatalogue::new`] against the real config location, or
/// [`TomlPresetCatalogue::under`] against an explicit directory for tests.
#[derive(Debug, Clone)]
pub struct TomlPresetCatalogue {
    directory: PathBuf,
}

impl TomlPresetCatalogue {
    /// Reads the presets directory from the environment —
    /// `<XDG config>/idle-manager/presets/`.
    ///
    /// # Errors
    ///
    /// [`LocatorSetup::NoHome`] if no home directory can be determined.
    pub fn new() -> Result<Self, LocatorSetup> {
        Ok(Self::under(paths::presets_dir()?))
    }

    /// Roots the catalogue at an explicit directory, for tests that must not
    /// touch the real config location.
    ///
    /// Seeds the shipped game files into `directory` if — and only if — it does
    /// not exist yet. An existing directory, even an empty one, is left
    /// untouched.
    #[must_use]
    pub fn under(directory: impl AsRef<Path>) -> Self {
        let directory = directory.as_ref().to_owned();
        seed_if_missing(&directory);
        Self { directory }
    }

    /// The directory this catalogue reads.
    #[must_use]
    pub fn directory(&self) -> &Path {
        &self.directory
    }
}

impl PresetCatalogue for TomlPresetCatalogue {
    fn read(&self) -> PresetCatalogueReading {
        let source = self.directory.display().to_string();

        let Ok(entries) = fs::read_dir(&self.directory) else {
            // A directory that is absent or unreadable is an empty catalogue,
            // not a failure entry — there is no single file to name. Task 03
            // creates and seeds it before we ever get here on a first run.
            tracing::warn!(
                directory = %self.directory.display(),
                "the presets directory could not be read; catalogue is empty"
            );
            return PresetCatalogueReading {
                presets: Vec::new(),
                failures: Vec::new(),
                source,
            };
        };

        let mut presets = Vec::new();
        let mut failures = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some(PRESET_EXTENSION) {
                continue;
            }

            let file_name = file_name_of(&path);
            match read_preset_file(&path) {
                Ok(preset) => presets.push(preset),
                Err(error) => failures.push(PresetFailure {
                    entry: file_name,
                    reason: error.to_string(),
                }),
            }
        }

        presets.sort_by(|a, b| a.display_name.cmp(&b.display_name));
        failures.sort_by(|a, b| a.entry.cmp(&b.entry));

        PresetCatalogueReading {
            presets,
            failures,
            source,
        }
    }
}

/// Writes the shipped game files into `directory` the first time it is found
/// missing.
///
/// An existing directory — even an empty one — is the user's and is never
/// touched: a file they edited keeps its edit and a file they deleted stays
/// deleted. A directory that cannot be created, or cannot be checked, is a
/// warning naming the path, after which the catalogue is simply empty rather
/// than the application failing to start (code standards rules 14, 15).
fn seed_if_missing(directory: &Path) {
    match directory.try_exists() {
        Ok(true) => return,
        Ok(false) => {}
        Err(error) => {
            tracing::warn!(
                directory = %directory.display(), %error,
                "could not check the presets directory; not seeding it"
            );
            return;
        }
    }

    if let Err(error) = fs::create_dir_all(directory) {
        tracing::warn!(
            directory = %directory.display(), %error,
            "could not create the presets directory; the catalogue will be empty"
        );
        return;
    }

    for (name, contents) in SHIPPED_PRESETS {
        if let Err(error) = fs::write(directory.join(name), contents) {
            tracing::warn!(file = name, %error, "could not write a shipped preset file");
        }
    }
}

/// Reads and maps one preset file into a domain [`Preset`], the file's stem
/// becoming its [`PresetId`].
fn read_preset_file(path: &Path) -> Result<Preset, PresetFileError> {
    let text = fs::read_to_string(path).map_err(PresetFileError::Unreadable)?;
    let file: PresetFile = toml::from_str(&text)?;

    let browser_identity = match file.user_agent {
        Some(identity) if identity.trim().is_empty() => {
            return Err(PresetFileError::EmptyUserAgent);
        }
        Some(identity) => Some(identity),
        None => None,
    };
    let zoom = ZoomLevel::new(file.zoom)?;
    let webgl_enabled = webgl_enabled(&file.webgl, &file_name_of(path));

    Ok(Preset {
        id: PresetId::new(stem_of(path)),
        display_name: file.name,
        start_address: file.url,
        browser_identity,
        zoom,
        keep_awake_default: file.keep_awake,
        webgl_enabled,
    })
}

fn file_name_of(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_owned()
}

fn stem_of(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_preset_file_with_no_webgl_key_defaults_the_field_to_enabled() {
        let absent = None;

        assert!(webgl_enabled(&absent, "huntera.toml"));
    }

    #[test]
    fn a_webgl_key_set_false_reads_back_as_disabled() {
        let disabled = Some(toml::Value::Boolean(false));

        assert!(!webgl_enabled(&disabled, "huntera.toml"));
    }

    #[test]
    fn a_non_boolean_webgl_key_falls_back_to_enabled() {
        let nonsense = Some(toml::Value::String("yes".to_owned()));

        assert!(webgl_enabled(&nonsense, "huntera.toml"));
    }
}
