//! What a *game* is, as opposed to an account on one: the small record the shell
//! reads to fill in everything an account needs beyond the name the user typed.
//!
//! Nothing here is serialised or read from disk — that is `store`'s job
//! (architecture rules 1, 7). The core only knows the shape a game takes once it
//! has been read.

/// The identifier for a preset: the stem of the file it was read from.
///
/// A newtype so it can never be passed where a [`crate::SessionId`] belongs —
/// both are strings to the compiler until made otherwise (code standards
/// rule 2).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PresetId(String);

impl PresetId {
    /// Wraps an existing identifier string. The adapter that reads the
    /// catalogue is the only place these are minted, from each file's name.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        PresetId(value.into())
    }

    /// The identifier as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PresetId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A page-zoom multiplier: `1.0` is the engine's normal size, `0.8` is smaller.
///
/// Constructed only through [`ZoomLevel::new`], which rejects anything that is
/// not a finite multiplier inside [`ZoomLevel::MIN`]`..=`[`ZoomLevel::MAX`], so
/// a view can never be asked to draw a page at a nonsensical size (code
/// standards rule 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ZoomLevel(f64);

impl ZoomLevel {
    /// The smallest multiplier a preset may ask for. Below this a page is
    /// unreadable, which is a different failure from a too-large one but just
    /// as useless.
    pub const MIN: f64 = 0.25;
    /// The largest multiplier a preset may ask for. Above this a page cannot
    /// show enough to play, and a mistyped `80` instead of `0.8` lands here.
    pub const MAX: f64 = 5.0;

    /// The multiplier an account gets when nothing supplies one — the "Something
    /// else" path, and any preset field left to its default. The engine's own
    /// normal size.
    pub const DEFAULT: ZoomLevel = ZoomLevel(1.0);

    /// Wraps `multiplier` if it is a finite value in [`ZoomLevel::MIN`]`..=`
    /// [`ZoomLevel::MAX`].
    ///
    /// # Errors
    ///
    /// [`InvalidZoom`] carrying `multiplier` if it is not finite, is zero or
    /// negative, or falls outside the accepted range.
    pub fn new(multiplier: f64) -> Result<Self, InvalidZoom> {
        if multiplier.is_finite() && (Self::MIN..=Self::MAX).contains(&multiplier) {
            Ok(ZoomLevel(multiplier))
        } else {
            Err(InvalidZoom(multiplier))
        }
    }

    /// The multiplier, exactly as it was constructed — the value the engine's
    /// `set_zoom_level` takes.
    #[must_use]
    pub fn multiplier(self) -> f64 {
        self.0
    }
}

impl Default for ZoomLevel {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The multiplier handed to [`ZoomLevel::new`] was not a usable zoom.
#[derive(Debug, Clone, Copy, PartialEq, thiserror::Error)]
#[error(
    "zoom multiplier {0} is outside the accepted range {min}..={max}",
    min = ZoomLevel::MIN,
    max = ZoomLevel::MAX
)]
pub struct InvalidZoom(pub f64);

/// One game the application knows how to add an account for: the name to show,
/// the address it starts at, an optional identity to introduce the engine with,
/// how large to draw the page, and whether that game needs to keep running at
/// full speed while hidden.
///
/// Every value here is copied onto a [`crate::Session`] at creation and never
/// read again, which is what makes an account independent of the file it came
/// from (roadmap item 06).
#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    /// Where the file was read from, for a caller that needs a stable key for
    /// the entry.
    pub id: PresetId,
    /// The name shown in the chooser.
    pub display_name: String,
    /// The address a new account loads first.
    pub start_address: String,
    /// The identity to present to this game, or `None` for the engine's own.
    ///
    /// Absent means *the engine's own*, never a string this crate supplies: a
    /// measured Chrome claim breaks a real sign-in, so there is deliberately no
    /// fallback constant (`FR.10.5`).
    pub browser_identity: Option<String>,
    /// How large to draw the page.
    pub zoom: ZoomLevel,
    /// The value a new account's keep-awake flag starts at. Only a starting
    /// value — the account owns the flag afterwards.
    pub keep_awake_default: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zoom_level_cannot_be_built_from_a_zero_multiplier() {
        let result = ZoomLevel::new(0.0);

        assert_eq!(result, Err(InvalidZoom(0.0)));
    }

    #[test]
    fn a_zoom_level_cannot_be_built_from_a_negative_multiplier() {
        let result = ZoomLevel::new(-1.0);

        assert_eq!(result, Err(InvalidZoom(-1.0)));
    }

    #[test]
    fn a_zoom_level_reports_back_the_multiplier_it_was_built_from() {
        let zoom = ZoomLevel::new(0.5).expect("0.5 is an accepted multiplier");

        assert!((zoom.multiplier() - 0.5).abs() < f64::EPSILON);
    }
}
