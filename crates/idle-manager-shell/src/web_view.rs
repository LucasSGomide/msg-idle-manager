//! One account's engine objects. The network session and the account's settings
//! outlive the view; the view itself is disposable and rebuilt on every start.

use std::cell::OnceCell;
use std::path::Path;

use gtk4 as gtk;
use webkit6::prelude::*;
use webkit6::{
    CookiePersistentStorage, Feature, FeatureList, NavigationAction, NetworkSession,
    PermissionRequest, ScriptDialog, Settings, URIRequest, UserContentInjectedFrames,
    UserContentManager, UserScript, UserScriptInjectionTime, WebProcessTerminationReason,
    WebResource, WebView,
};

use idle_manager_core::SessionId;

/// The cookie database file, kept inside the account's data directory so it
/// survives a restart alongside the rest of the profile.
const COOKIE_DB: &str = "cookies.sqlite";

/// The name the injected bridge posts console messages under. It appears in
/// [`PAGE_CONSOLE_JS`] as `webkit.messageHandlers.pageConsole`; the two spell
/// the same string and have to be changed together.
const PAGE_CONSOLE_HANDLER: &str = "pageConsole";

/// Compiled in rather than loaded from the `GResource` bundle: this is the only
/// caller, it needs the source before any widget exists, and `include_str!`
/// keeps the read infallible so the holder needs no error path for it.
const PAGE_CONSOLE_JS: &str = include_str!("../resources/js/page-console.js");

/// The size an authentication popup opens at before its own page resizes it.
const POPUP_WIDTH: i32 = 480;
/// The size an authentication popup opens at before its own page resizes it.
const POPUP_HEIGHT: i32 = 680;

/// The engine's own switch that stretches out a hidden page's timers. Read off
/// this machine's [`log_engine_features`] start-up log — `WebKitGTK` 2.52.6,
/// `webkit6` 0.6.1, on 2026-09-06 — not published anywhere the crate can read
/// at build time, so a future engine upgrade may spell or number it
/// differently; re-read the log rather than assuming this string still applies
/// (code standards rule 18).
const FEATURE_ID_HIDDEN_PAGE_TIMER_THROTTLING: &str = "HiddenPageDOMTimerThrottling";
/// The engine's own switch that suspends a hidden page's CSS animations. Read
/// off the same start-up log, same engine build, same date as
/// [`FEATURE_ID_HIDDEN_PAGE_TIMER_THROTTLING`].
const FEATURE_ID_HIDDEN_PAGE_CSS_ANIMATION_SUSPENSION: &str = "HiddenPageCSSAnimationSuspension";

/// One account's engine objects, held so the view can be destroyed and rebuilt
/// without losing the account's storage.
///
/// The [`NetworkSession`], the start address and the keep-awake flag are
/// permanent; the [`WebView`] is present only while the account is running.
/// Build one with [`SessionView::new`], read the current view with
/// [`SessionView::view`], move between running and parked with
/// [`SessionView::stop`] and [`SessionView::start`], and change keep-awake
/// with [`SessionView::set_keep_awake`].
#[derive(Debug)]
pub struct SessionView {
    id: SessionId,
    network_session: NetworkSession,
    start_address: String,
    /// Whether the account must keep running at full speed while hidden
    /// (`FR.6.1`). Remembered here, not just on the live view, so a view
    /// rebuilt by [`SessionView::start`] after a park carries it forward.
    keep_awake: bool,
    view: Option<WebView>,
}

impl SessionView {
    /// Builds the holder with its own persistent, isolated network session
    /// rooted at `data_dir` and `cache_dir`, then builds and starts its first
    /// view loading `start_address`. `id` names the account in the keep-awake
    /// log line, since the two hidden-page switches are per view rather than
    /// per context (`webkit6` 0.6.1, `src/auto/web_view.rs:157`) and a debug
    /// log naming only the feature would not say whose page it touched.
    #[must_use]
    pub fn new(id: &SessionId, data_dir: &Path, cache_dir: &Path, start_address: &str) -> Self {
        let mut holder = Self {
            id: id.clone(),
            network_session: build_network_session(data_dir, cache_dir),
            start_address: start_address.to_owned(),
            keep_awake: false,
            view: None,
        };
        holder.start();
        holder
    }

    /// The account's live view, or `None` while it is parked.
    #[must_use]
    pub fn view(&self) -> Option<&WebView> {
        self.view.as_ref()
    }

    /// Parks the account: ends the rendering process, then drops the view.
    ///
    /// The two happen in this order and it cannot be reversed. Dropping the
    /// view first leaves the engine to decide when — and whether — the process
    /// dies, which is the difference between memory returned to the system and
    /// memory handed to `WebKit`'s process cache (`FR.5.2`, code-standards
    /// rule 18). A no-op if the account is already parked.
    pub fn stop(&mut self) {
        let Some(view) = self.view.take() else {
            return;
        };
        view.terminate_web_process();
    }

    /// Starts the account: builds a fresh view bound to the kept network
    /// session, applies the shared settings and the remembered keep-awake
    /// switches, and loads the start address. Returns the new view. Replaces
    /// any view already held.
    ///
    /// A new view, never a revived one: `network-session` is construct-only
    /// (`webkit6` 0.6.1, `web_view.rs:141`), so the binding to the kept storage
    /// area cannot be remade on the old view (`FR.5.4`, code-standards
    /// rule 18). Applying keep-awake here — not just in
    /// [`SessionView::set_keep_awake`] — is what makes a parked-and-restarted
    /// account come back with the same switches already off (`FR.6.1`).
    pub fn start(&mut self) -> &WebView {
        let view = WebView::builder()
            .network_session(&self.network_session)
            .user_content_manager(&page_console_bridge())
            .build();

        configure(&view);
        if self.keep_awake {
            apply_keep_awake(&view, true, &self.id);
        }
        view.load_uri(&self.start_address);

        self.view.insert(view)
    }

    /// Turns keep-awake on or off for this account: flips the two hidden-page
    /// engine switches on the current view's settings and reloads, since
    /// there is no API to make a settings change reach a page that has
    /// already loaded (`FR.6.2`, `webkit6` 0.6.1,
    /// `src/auto/user_script.rs:22`'s doc explains the same limit for the
    /// script half of this feature). Returns the reloaded view, or `None`
    /// while the account is parked.
    ///
    /// Remembers `on` regardless of whether a view exists, so a later
    /// [`SessionView::start`] builds its fresh view with the same switches
    /// already applied (`FR.6.1`).
    pub fn set_keep_awake(&mut self, on: bool) -> Option<&WebView> {
        self.keep_awake = on;
        let view = self.view.as_ref()?;

        apply_keep_awake(view, on, &self.id);
        view.reload();

        Some(view)
    }
}

/// Builds the persistent, isolated network session for one account.
///
/// The construction order is forced: the [`NetworkSession`] takes both
/// directories at construction and cannot be told about them afterwards, and
/// its cookie manager is switched to on-disk storage before the first load, or
/// the login is only in memory and is lost on exit.
fn build_network_session(data_dir: &Path, cache_dir: &Path) -> NetworkSession {
    let data = data_dir.to_str().expect("profile data path is valid UTF-8");
    let cache = cache_dir
        .to_str()
        .expect("profile cache path is valid UTF-8");

    let session = NetworkSession::new(Some(data), Some(cache));

    let cookies = session
        .cookie_manager()
        .expect("a non-ephemeral network session has a cookie manager");
    let cookie_db = data_dir.join(COOKIE_DB);
    cookies.set_persistent_storage(
        cookie_db
            .to_str()
            .expect("cookie database path is valid UTF-8"),
        CookiePersistentStorage::Sqlite,
    );

    session
}

/// Applies the settings, popup handling and diagnostics every view in one
/// account shares — the account's view and any authentication popup it opens.
///
/// It deliberately leaves the user agent alone. An earlier version of this file
/// claimed to be a current desktop Chrome, on the reasoning that game sites
/// refuse a browser they do not recognise. Measured against a real login, that
/// string is what *breaks* the login: `WebKit` supplies no `window.chrome`, no
/// `navigator.userAgentData` and reports `navigator.vendor` as Apple, so a
/// Chrome claim contradicts everything else the page can see. Google Identity
/// Services answers a click on its own button by posting one telemetry record
/// and opening nothing, and Cloudflare Turnstile never issues a token. Under
/// the engine's own user agent the same click opens the OAuth window and the
/// same widget clears itself. Item 06 makes this a per-game preset field; a
/// game that genuinely needs a different string gets one there, tested against
/// that game, rather than every session claiming the same untested lie.
fn configure(view: &WebView) {
    let settings = webkit6::prelude::WebViewExt::settings(view)
        .expect("a web view always has a settings object");
    // Per-site compatibility workarounds WebKit ships. Off by default; a browser
    // ships with them on, and a game that renders wrong without them is a worse
    // failure than the quirk itself.
    settings.set_enable_site_specific_quirks(true);
    settings.set_enable_webgl(true);
    // Right-click -> Inspect Element, for diagnosing a page that will not load
    // or log in. Item 08 owns turning failures into UI; until then the inspector
    // and the traces below are how a login problem gets looked at.
    settings.set_enable_developer_extras(true);
    // Duplicates the page's own messages, which the bridge already reports, but
    // it is the only source of the engine's: "Blocked a frame with origin ...",
    // a load cancelled by a cross-origin policy, a mixed-content refusal. Those
    // never reach a page's `console` object, so overriding it cannot see them.
    settings.set_enable_write_console_messages_to_stdout(cfg!(debug_assertions));

    view.connect_create(|opener, action| Some(open_popup(opener, action)));
    wire_diagnostics(view);
}

/// A content manager carrying the document-start script that forwards the
/// page's console output and uncaught errors into `tracing`.
///
/// A page's `console.error` is the page's problem, not the application's, so it
/// arrives as a `warn`; everything quieter than that arrives below the level
/// `make dev` asks for, because one Cloudflare challenge frame alone logs
/// hundreds of lines per load.
fn page_console_bridge() -> UserContentManager {
    let content = UserContentManager::new();

    if content.register_script_message_handler(PAGE_CONSOLE_HANDLER, None) {
        content.connect_script_message_received(Some(PAGE_CONSOLE_HANDLER), |_, message| {
            let field = |name: &str| {
                message
                    .object_get_property(name)
                    .map(|value| value.to_str().to_string())
                    .unwrap_or_default()
            };
            let (level, origin, text) = (field("level"), field("origin"), field("text"));

            match level.as_str() {
                "error" => tracing::warn!(origin, text, "page console error"),
                "warn" => tracing::debug!(origin, text, "page console warning"),
                _ => tracing::trace!(level, origin, text, "page console"),
            }
        });
    } else {
        tracing::warn!(
            handler = PAGE_CONSOLE_HANDLER,
            "the page console bridge could not be registered; page errors will not be traced"
        );
    }

    content.add_script(&UserScript::new(
        PAGE_CONSOLE_JS,
        UserContentInjectedFrames::AllFrames,
        UserScriptInjectionTime::Start,
        &[],
        &[],
    ));
    content
}

/// Logs the load lifecycle, subresource loads, script dialogs, permission
/// requests, load and TLS failures and web-process death against `tracing`, so
/// `RUST_LOG=idle_manager_shell=debug` shows why a page stalled and
/// `...=trace` shows every request it made. None of these handlers change
/// behaviour — every one returns `false` so `WebKit` keeps its own.
fn wire_diagnostics(view: &WebView) {
    view.connect_load_changed(|view, event| {
        tracing::debug!(?event, uri = view.uri().as_deref(), "load changed");
    });

    view.connect_load_failed(|_, event, uri, error| {
        tracing::warn!(?event, uri, %error, "load failed");
        false
    });

    view.connect_load_failed_with_tls_errors(|_, host, _certificate, errors| {
        tracing::warn!(host, ?errors, "load failed on TLS errors");
        false
    });

    view.connect_resource_load_started(log_resource);
    view.connect_script_dialog(log_script_dialog);
    view.connect_permission_request(log_permission_request);

    view.connect_web_process_terminated(|_, reason| {
        // Item 08 attaches its automatic crash reload to this same signal. A
        // deliberate park raises it with `TerminatedByApi`, which must read as
        // routine here or the reload would fight the park (code-standards
        // rule 18).
        match reason {
            WebProcessTerminationReason::TerminatedByApi => {
                tracing::debug!("web process terminated by API (parked)");
            }
            other => tracing::error!(reason = ?other, "web process terminated"),
        }
    });
}

/// Traces one subresource request and whatever ends it. A login that silently
/// does nothing still makes requests, and which ones it makes — or which one
/// comes back refused — is the difference between a blocked popup and a site
/// that decided not to offer one.
fn log_resource(_: &WebView, resource: &WebResource, request: &URIRequest) {
    let uri = request.uri();
    tracing::trace!(uri = uri.as_deref(), "resource load started");

    resource.connect_failed(|resource, error| {
        tracing::debug!(uri = resource.uri().as_deref(), %error, "resource load failed");
    });
    resource.connect_failed_with_tls_errors(|resource, _certificate, errors| {
        tracing::debug!(
            uri = resource.uri().as_deref(),
            ?errors,
            "resource load failed on TLS errors"
        );
    });
}

/// Traces an `alert`, `confirm`, `prompt` or beforeunload prompt and lets
/// `WebKit` show its own. A game asking a question the window never displays
/// looks exactly like a game that has frozen.
fn log_script_dialog(_: &WebView, dialog: &ScriptDialog) -> bool {
    tracing::debug!(
        kind = ?dialog.dialog_type(),
        message = dialog.message().as_deref(),
        "page opened a script dialog"
    );
    false
}

/// Traces a permission request and leaves it to `WebKit`, which denies what it
/// is not told to allow. Naming which permission was asked for is the point:
/// a page that will not proceed until it is granted one is otherwise silent
/// about why.
fn log_permission_request(_: &WebView, request: &PermissionRequest) -> bool {
    tracing::debug!(kind = request.type_().name(), "page requested a permission");
    false
}

/// Opens a `window.open()` / `target="_blank"` navigation in a real top-level
/// window, the way a browser does — this is the "sign in with Google" flow
/// every game login uses, and without it the auth window never appears.
///
/// The popup is built with the construct-only `related-view` property, so it
/// shares the opener's network session and web process: it is the same
/// logged-in browser, and the cookie it is granted lands in the account's
/// profile. It inherits the opener's content manager too, so the console of an
/// auth window that fails is traced like any other. It is parented into its
/// window only once `WebKit` has sized it (`ready-to-show`), following the
/// `WebKitGTK` embedder convention.
fn open_popup(opener: &WebView, action: &NavigationAction) -> gtk::Widget {
    let uri = action.request().and_then(|request| request.uri());
    tracing::debug!(
        ?uri,
        user_gesture = action.is_user_gesture(),
        "opening a popup window"
    );

    // No gesture check here. `javascript-can-open-windows-automatically` is off
    // by default, so WebKit has already applied a browser's popup policy before
    // this signal is emitted; a second check in front of it can only reject a
    // window WebKit was willing to open, and every window this application is
    // asked for is a login it exists to complete.
    let mut popup = WebView::builder().related_view(opener);
    if let Some(content) = opener.user_content_manager() {
        popup = popup.user_content_manager(&content);
    }
    let popup = popup.build();
    configure(&popup);

    let parent = opener.root().and_downcast::<gtk::Window>();
    let popup_for_show = popup.clone();
    popup.connect_ready_to_show(move |_| {
        let window = gtk::Window::builder()
            .title("Sign in")
            .default_width(POPUP_WIDTH)
            .default_height(POPUP_HEIGHT)
            .destroy_with_parent(true)
            .child(&popup_for_show)
            .build();
        if let Some(parent) = &parent {
            window.set_transient_for(Some(parent));
        }

        let window_for_close = window.clone();
        popup_for_show.connect_close(move |_| window_for_close.close());

        window.present();
    });

    popup.upcast::<gtk::Widget>()
}

/// The two `Feature` handles that item 04's keep-awake flips on a session's
/// `Settings` object (task 03 is the only reader). Looked up once by
/// identifier because the feature list is a build-time property of the engine
/// and cannot change while the process runs, so searching it per view would
/// repeat work for nothing.
struct KeepAwakeFeatures {
    hidden_page_timer_throttling: Option<Feature>,
    hidden_page_css_animation_suspension: Option<Feature>,
}

thread_local! {
    // `Feature` is a glib boxed type and is not `Sync`, so the holder lives on
    // the GTK main context in a `thread_local`, never a `static OnceLock`
    // (architecture rule 10).
    static KEEP_AWAKE_FEATURES: OnceCell<KeepAwakeFeatures> = const { OnceCell::new() };
}

/// Looks up [`FEATURE_ID_HIDDEN_PAGE_TIMER_THROTTLING`] and
/// [`FEATURE_ID_HIDDEN_PAGE_CSS_ANIMATION_SUSPENSION`] in `list`.
///
/// The feature list is not a stable API and a future engine build may rename
/// or remove either identifier, so a miss is a `tracing::warn` naming it,
/// never a panic (code standards rules 14, 15) — a session that runs without
/// one switch is better than an application that will not start.
fn find_keep_awake_features(list: &FeatureList) -> KeepAwakeFeatures {
    let find = |identifier: &str| {
        let feature = (0..list.length())
            .filter_map(|index| list.get(index))
            .find(|feature| feature.identifier().as_deref() == Some(identifier));
        if feature.is_none() {
            tracing::warn!(
                identifier,
                "keep-awake feature not found in this engine build"
            );
        }
        feature
    };

    KeepAwakeFeatures {
        hidden_page_timer_throttling: find(FEATURE_ID_HIDDEN_PAGE_TIMER_THROTTLING),
        hidden_page_css_animation_suspension: find(FEATURE_ID_HIDDEN_PAGE_CSS_ANIMATION_SUSPENSION),
    }
}

/// Walks every feature this engine build exposes and logs it once at `debug`
/// — identifier, name, category and whether the engine's own default already
/// matches what keep-awake wants — then looks up and caches the two features
/// item 04 needs.
///
/// Call once, at shell start-up beside [`crate::configure_web_engine`]: the
/// feature list is a build-time property of the engine, so walking it per
/// view would repeat work that can never change while the process runs
/// (architecture rules 10, 12). The printing stays permanently — it is how the
/// two identifiers above get re-read after the next engine upgrade.
pub(crate) fn log_engine_features() {
    let Some(list) = Settings::all_features() else {
        tracing::warn!("WebKit reported no feature list; keep-awake cannot look up its features");
        return;
    };

    for index in 0..list.length() {
        let Some(feature) = list.get(index) else {
            continue;
        };
        tracing::debug!(
            identifier = feature.identifier().as_deref(),
            name = feature.name().as_deref(),
            category = feature.category().as_deref(),
            is_default_value = feature.is_default_value(),
            "engine feature"
        );
    }

    let features = find_keep_awake_features(&list);
    // Named field reads, not `?features`: a derived `Debug` impl does not
    // count as a use for dead-code analysis.
    tracing::debug!(
        timer_throttling_found = features.hidden_page_timer_throttling.is_some(),
        css_animation_suspension_found = features.hidden_page_css_animation_suspension.is_some(),
        "keep-awake feature lookup complete"
    );
    KEEP_AWAKE_FEATURES.with(|cell| {
        cell.get_or_init(|| features);
    });
}

/// Disables (`on = true`) or restores (`on = false`) both hidden-page engine
/// switches on `view`'s settings, from the list [`log_engine_features`] cached
/// at start-up, and logs which ones were touched for account `id`.
///
/// Never re-searches [`Settings::all_features`] — the whole reason the list is
/// cached once. A feature missing from this build was already warned about
/// there; here it is silently skipped, because a session running with one
/// switch missing is better than one that will not start (code standards
/// rules 14, 15).
fn apply_keep_awake(view: &WebView, on: bool, id: &SessionId) {
    let settings = webkit6::prelude::WebViewExt::settings(view)
        .expect("a web view always has a settings object");

    KEEP_AWAKE_FEATURES.with(|cell| {
        let Some(features) = cell.get() else {
            // `log_engine_features` runs once at start-up, before any session
            // is built; reaching here without it is a start-up ordering bug,
            // not a per-session condition, so it earns a warning rather than
            // silent skipping like a genuinely missing feature does.
            tracing::warn!(
                session = %id,
                "keep-awake features were never looked up; log_engine_features must run at start-up"
            );
            return;
        };

        let mut applied = Vec::new();
        if let Some(feature) = &features.hidden_page_timer_throttling {
            settings.set_feature_enabled(feature, !on);
            applied.push(FEATURE_ID_HIDDEN_PAGE_TIMER_THROTTLING);
        }
        if let Some(feature) = &features.hidden_page_css_animation_suspension {
            settings.set_feature_enabled(feature, !on);
            applied.push(FEATURE_ID_HIDDEN_PAGE_CSS_ANIMATION_SUSPENSION);
        }

        tracing::debug!(session = %id, keep_awake = on, features = ?applied, "keep-awake features set");
    });
}
