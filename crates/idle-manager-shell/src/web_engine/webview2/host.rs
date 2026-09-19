//! `EngineHost`: the GTK widget that stands where a game goes on Windows. It
//! draws nothing itself — `EngineView::widget()` returns it, and the real
//! `wry::WebView` it hosts is a separate native child window `WebView2`
//! always draws on top of (roadmap item 12, "Airspace").

mod imp;

use gtk::glib;
use gtk::subclass::prelude::*;
use gtk4 as gtk;

use idle_manager_core::SessionId;

glib::wrapper! {
    /// A widget with no visual content of its own, holding one account's
    /// `wry::WebView` as a native child window. Built empty by
    /// [`EngineHost::new`]; the view itself is built lazily, the first time
    /// the host is realized, because only a realized widget has the `HWND`
    /// `WebViewBuilder::build_as_child` needs.
    pub struct EngineHost(ObjectSubclass<imp::EngineHost>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

/// Everything a not-yet-realized [`EngineHost`] needs to build its
/// `wry::WebView` once it has an `HWND` — set once at construction, consumed
/// exactly once, the first time `realize` runs.
pub(super) struct PendingView {
    pub(super) id: SessionId,
    pub(super) start_address: String,
    pub(super) user_agent: Option<String>,
    pub(super) profile_name: String,
    pub(super) zoom: f64,
    /// Whether the prelude that arms the frame-callback shim
    /// (`script_set::KEEP_AWAKE_PRELUDE_JS`) belongs in this view's script
    /// set (roadmap item 12 task 04, `FR.6.3`; the shim itself is in every
    /// set since item 13). Read only at construction: `wry`'s
    /// `with_initialization_script` is a builder-time-only call, the same
    /// construct-only constraint `web_engine/webkit.rs`'s content manager has
    /// on Linux, so there is no way to add the prelude to a view that already
    /// exists.
    pub(super) keep_awake: bool,
}

impl EngineHost {
    /// Builds an unrealized host that will build `pending`'s view the first
    /// time it is realized.
    #[must_use]
    pub(super) fn new(pending: PendingView) -> Self {
        let host: Self = glib::Object::new();
        host.imp().set_pending(pending);
        host
    }

    /// Runs `f` once, the first time this host's view paints — or
    /// immediately, if it already has. Mirrors
    /// [`super::EngineView::connect_painted`]'s contract; queued here when
    /// the view does not exist yet, since realization can happen well after
    /// construction.
    pub(super) fn connect_painted(&self, f: impl Fn() + 'static) {
        self.imp().connect_painted(f);
    }

    /// Runs `f` on every `ProcessFailed` this host's view reports. Queued the
    /// same way [`EngineHost::connect_painted`] is, and for the same reason.
    pub(super) fn connect_terminated(&self, f: impl Fn() + 'static) {
        self.imp().connect_terminated(f);
    }

    /// Loads `uri` if the view already exists — a no-op otherwise, since
    /// `PendingView::start_address` is what the first build loads.
    pub(super) fn load_uri(&self, uri: &str) {
        self.imp().load_uri(uri);
    }

    /// Reloads the hosted view's current page. A no-op before the view
    /// exists.
    pub(super) fn reload(&self) {
        self.imp().reload();
    }

    /// Resizes the hosted view's page to `zoom`. A no-op before the view
    /// exists — `PendingView::zoom` still applies once it is built.
    pub(super) fn set_zoom(&self, zoom: f64) {
        self.imp().set_zoom(zoom);
    }

    /// Grabs keyboard focus for the hosted native view. A no-op before the
    /// view exists.
    pub(super) fn focus_view(&self) {
        self.imp().focus_view();
    }

    /// Marks the hosted view backgrounded or not (roadmap item 12 task 04).
    /// Remembered even before the view exists, so a host built already
    /// minimised opens backgrounded from its very first frame rather than a
    /// visible one this call arrived too early to reach.
    pub(super) fn set_background(&self, background: bool) {
        self.imp().set_background(background);
    }

    /// Runs `source` in the hosted view's page (roadmap item 13 task 02).
    /// `Err(reason)` says it did not run — no view built yet, or `wry`
    /// refused it — for the caller to log with its session field.
    pub(super) fn run_script(&self, source: &str) -> Result<(), String> {
        self.imp().run_script(source)
    }

    /// Asks the engine for a JPEG of the hosted view's page right now,
    /// calling `done` exactly once with the bytes or with one line saying why
    /// there are none (roadmap item 13 task 02, `FR.4.3`).
    pub(super) fn capture_frame(&self, done: impl FnOnce(Result<Vec<u8>, String>) + 'static) {
        self.imp().capture_frame(done);
    }

    /// Shrinks the hosted view to nothing, so nothing native draws over this
    /// place — `session_grid/imp.rs`'s way of keeping a GTK-drawn name cover,
    /// "Starting" placeholder, or drop highlight visible above a `WebView2`
    /// child window that would otherwise always win the airspace (roadmap
    /// item 12 task 05, "Airspace"). `pub(crate)`, unlike every other method
    /// here: the grid reaches this directly, across the module boundary but
    /// inside the one crate (architecture rule 12), because it is the one
    /// caller that needs to act on a view without going through
    /// `EngineView`'s own narrower interface.
    pub(crate) fn collapse(&self) {
        self.imp().collapse();
    }

    /// Asks the engine to delete the profile this host's view runs under,
    /// calling `done` exactly once (roadmap item 12 task 06, `FR.21.8`).
    ///
    /// `Ok` means the engine itself reported the profile gone, which is the
    /// only path that leaves every sibling account's profile in the shared
    /// user-data folder untouched while the browser process keeps running.
    /// `Err(reason)` means nothing was deleted — no view to reach a profile
    /// through, or a runtime without the profile-deletion call — and the
    /// caller falls back to removing that one profile's folder itself
    /// (`EngineProfile::delete`).
    pub(super) fn delete_profile(&self, done: impl FnOnce(Result<(), String>) + 'static) {
        self.imp().delete_profile(done);
    }

    /// Reverses [`EngineHost::collapse`], resizing the hosted view back to
    /// this host's current GTK allocation — read fresh, not remembered, so a
    /// host whose slot moved while collapsed (a drag's swap or fill) restores
    /// into its new place, not its old one.
    pub(crate) fn restore(&self) {
        self.imp().restore();
    }
}
