//! One account's engine objects. The network session and the account's settings
//! outlive the view; the view itself is disposable and rebuilt on every start.

use idle_manager_core::{ProfileDirectories, SessionId, ZoomLevel};

use crate::web_engine::{EngineProfile, EngineView};

/// The view settings an account carries from its preset: the page zoom, the
/// browser identity (or `None` for the engine's own), and whether its pages get
/// a WebGL context.
///
/// One value rather than three positional arguments, so a fourth preset-derived
/// setting is a field rather than another parameter on every constructor
/// (`FR.19.7`).
#[derive(Debug, Clone)]
pub struct AccountSettings {
    /// How large to draw the account's pages.
    pub zoom: ZoomLevel,
    /// The identity to present, or `None` for the engine's own (`FR.10.5`).
    pub identity: Option<String>,
    /// Whether the account's pages get a WebGL context (`FR.19.7`).
    pub webgl_enabled: bool,
}

/// One account's engine objects, held so the view can be destroyed and rebuilt
/// without losing the account's storage.
///
/// The [`EngineProfile`], the start address, the keep-awake flag, the zoom and
/// the browser identity are permanent; the [`EngineView`] is present only while
/// the account is running. Build one with [`SessionView::new`], read the
/// current view with [`SessionView::view`], move between running and parked
/// with [`SessionView::stop`] and [`SessionView::start`], and change
/// keep-awake with [`SessionView::set_keep_awake`].
#[derive(Debug)]
pub struct SessionView {
    id: SessionId,
    profile: EngineProfile,
    start_address: String,
    /// Whether the account must keep running at full speed while hidden
    /// (`FR.6.1`). Remembered here, not just on the live view, so a view
    /// rebuilt by [`SessionView::start`] after a park carries it forward.
    keep_awake: bool,
    /// The zoom, identity and WebGL setting copied from the account's preset.
    /// Remembered here, not just on the live view, so a view rebuilt by
    /// [`SessionView::start`] after a park comes back with the same size,
    /// identity and context rather than the engine's defaults (code standards
    /// rule 18).
    settings: AccountSettings,
    view: Option<EngineView>,
}

impl SessionView {
    /// Builds the holder and its own persistent, isolated engine profile
    /// rooted at `directories`, without building a view.
    ///
    /// A restored account gets this and nothing more (item 07 task 04):
    /// building a view only to stop it would spend the process spike the
    /// one-at-a-time restore exists to spread out. The start queue calls
    /// [`SessionView::start`] when the account's turn comes.
    #[must_use]
    pub fn dormant(
        id: &SessionId,
        directories: &ProfileDirectories,
        start_address: &str,
        settings: AccountSettings,
    ) -> Self {
        Self {
            id: id.clone(),
            profile: EngineProfile::new(id, directories),
            start_address: start_address.to_owned(),
            keep_awake: false,
            settings,
            view: None,
        }
    }

    /// [`SessionView::dormant`] plus a first [`SessionView::start`]: the holder
    /// and its first running view, loading `start_address` at `zoom` and
    /// presenting `identity` when it is `Some`. The one path that builds a
    /// view for a brand-new account. `minimised` is the window's current
    /// state, so a view built while it is already minimised starts marked to
    /// match (roadmap item 12 task 04, `FR.1.9`).
    #[must_use]
    pub fn new(
        id: &SessionId,
        directories: &ProfileDirectories,
        start_address: &str,
        settings: AccountSettings,
        minimised: bool,
    ) -> Self {
        let mut holder = Self::dormant(id, directories, start_address, settings);
        holder.start(minimised);
        holder
    }

    /// The account's live view, or `None` while it is parked.
    #[must_use]
    pub fn view(&self) -> Option<&EngineView> {
        self.view.as_ref()
    }

    /// The account's persistent engine profile — the network session and
    /// whatever storage it is rooted at, outliving every view. Read by
    /// [`crate::account_deletion`] so it can clear the account's website data
    /// before the holder (and this profile with it) is dropped.
    #[must_use]
    pub(crate) fn profile(&self) -> &EngineProfile {
        &self.profile
    }

    /// Parks the account: ends its running view's rendering process, then
    /// drops the view.
    ///
    /// The two happen in this order and it cannot be reversed. Dropping the
    /// view first leaves the engine to decide when — and whether — the
    /// process dies, which is the difference between memory returned to the
    /// system and memory handed to the engine's process cache (`FR.5.2`,
    /// code-standards rule 18). A no-op if the account is already parked.
    pub fn stop(&mut self) {
        let Some(view) = self.view.take() else {
            return;
        };
        view.terminate();
    }

    /// Starts the account: builds a fresh view from the kept engine profile,
    /// applying the shared settings and the remembered keep-awake switch, and
    /// loads the start address. Returns the new view. Replaces any view
    /// already held.
    ///
    /// A new view, never a revived one — a profile's engine object cannot be
    /// rebound to a fresh view once one already exists (`FR.5.4`). Applying
    /// keep-awake, the zoom and the identity on every build — not just once,
    /// at construction — is what makes a parked-and-restarted account come
    /// back with the same switches, the same size and the same identity
    /// (`FR.6.1`, `FR.10.5`). `minimised` is the window's current state
    /// ([`background_for`], roadmap item 12 task 04, `FR.1.9`): a no-op on
    /// Linux, where `EngineView::set_background` stays the permanent no-op
    /// task 01 made it.
    pub fn start(&mut self, minimised: bool) -> &EngineView {
        let view = self
            .profile
            .build_view(&self.start_address, &self.settings, self.keep_awake);
        view.set_background(background_for(minimised, self.keep_awake));
        self.view.insert(view)
    }

    /// Turns keep-awake on or off for this account: applies it to the current
    /// view, reloading it, since there is no way to make the change reach a
    /// page that has already loaded (`FR.6.2`, `FR.6.3`). Returns the
    /// reloaded view, or `None` while the account is parked.
    ///
    /// Remembers `on` regardless of whether a view exists, so a later
    /// [`SessionView::start`] builds its fresh view with the same switch
    /// already applied (`FR.6.1`). Also applies [`background_for`] with the
    /// window's current `minimised` state, so toggling keep-awake while the
    /// window is already minimised takes effect at once instead of waiting
    /// for the next minimise/restore (roadmap item 12 task 04, `FR.1.9`).
    pub fn set_keep_awake(&mut self, on: bool, minimised: bool) -> Option<&EngineView> {
        self.keep_awake = on;
        let view = self.view.as_ref()?;
        view.set_keep_awake(on, &self.id);
        view.set_background(background_for(minimised, on));
        Some(view)
    }

    /// Sets this account's page zoom to `zoom`.
    ///
    /// Remembers it on the holder whether or not a view exists, so a parked
    /// account started later opens at the chosen size (`FR.11.5`, `FR.12.7`).
    /// When a view is live it is resized where it stands with **no reload**,
    /// so the account never leaves `Live`. Returns the live view, or `None`
    /// while the account is parked.
    pub fn set_zoom(&mut self, zoom: ZoomLevel) -> Option<&EngineView> {
        self.settings.zoom = zoom;
        let view = self.view.as_ref()?;
        view.set_zoom(zoom);
        Some(view)
    }
}

/// Whether an account's view should be marked invisible to the engine right
/// now: only while the window is minimised and the account's own keep-awake
/// switch is off (roadmap item 12 task 04, `FR.1.9`). A kept-awake account is
/// always left visible, whatever the window's state; a visible window never
/// puts any account in the background.
///
/// Pure and platform-independent on purpose (code standards rules 21–25):
/// `EngineView::set_background` is what actually differs between the two
/// engines, and this is the one decision behind it, unit-tested here on every
/// platform rather than only where `set_background` does something.
pub(crate) fn background_for(minimised: bool, keep_awake: bool) -> bool {
    minimised && !keep_awake
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimised_and_not_kept_awake_goes_to_the_background() {
        assert!(background_for(true, false));
    }

    #[test]
    fn minimised_but_kept_awake_stays_visible() {
        assert!(!background_for(true, true));
    }

    #[test]
    fn not_minimised_and_kept_awake_stays_visible() {
        assert!(!background_for(false, true));
    }

    #[test]
    fn not_minimised_and_not_kept_awake_stays_visible() {
        assert!(!background_for(false, false));
    }
}
