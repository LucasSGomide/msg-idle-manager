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

use idle_manager_core::{ProfileDirectories, SessionId, ZoomLevel};

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

/// The frame-callback shim (`FR.6.3`), brought in the same way and for the same
/// reason as [`PAGE_CONSOLE_JS`]: this is its only caller, it is needed before
/// any widget exists, and `include_str!` keeps the read infallible.
const KEEP_AWAKE_JS: &str = include_str!("../resources/js/keep-awake.js");

/// The size an authentication popup opens at before its own page resizes it.
const POPUP_WIDTH: i32 = 480;
/// The size an authentication popup opens at before its own page resizes it.
const POPUP_HEIGHT: i32 = 680;

/// Setting this in the environment turns the memory-costing diagnostics —
/// the web inspector backend and per-resource load logging — back on in a
/// release run (`FR.19.6`). A debug build has them regardless.
const DIAGNOSTICS_ENV: &str = "IDLE_MANAGER_DIAGNOSTICS";

/// Whether the diagnostics that cost memory in every account should be on.
///
/// The inspector backend makes the engine instrument every page differently,
/// and the two per-resource signal handlers are attached for every resource a
/// page loads — a cost that scales with uptime, not with account count, which
/// is the worst shape a cost can have in an application meant to be left
/// running for days (`FR.19.6`, code standards rule 18). On in a debug build,
/// where anyone debugging already is; otherwise only when the environment asks,
/// through the same channel the tracing filter reads.
fn diagnostics_enabled() -> bool {
    cfg!(debug_assertions) || std::env::var_os(DIAGNOSTICS_ENV).is_some()
}

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
/// The [`NetworkSession`], the start address, the keep-awake flag, the zoom and
/// the browser identity are permanent; the [`WebView`] is present only while the
/// account is running. Build one with [`SessionView::new`], read the current
/// view with [`SessionView::view`], move between running and parked with
/// [`SessionView::stop`] and [`SessionView::start`], and change keep-awake with
/// [`SessionView::set_keep_awake`].
#[derive(Debug)]
pub struct SessionView {
    id: SessionId,
    network_session: NetworkSession,
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
    view: Option<WebView>,
}

impl SessionView {
    /// Builds the holder and its own persistent, isolated network session
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
            network_session: build_network_session(&directories.data, &directories.cache),
            start_address: start_address.to_owned(),
            keep_awake: false,
            settings,
            view: None,
        }
    }

    /// [`SessionView::dormant`] plus a first [`SessionView::start`]: the holder
    /// and its first running view, loading `start_address` at `zoom` and
    /// presenting `identity` when it is `Some`. The one path that builds a
    /// view for a brand-new account.
    #[must_use]
    pub fn new(
        id: &SessionId,
        directories: &ProfileDirectories,
        start_address: &str,
        settings: AccountSettings,
    ) -> Self {
        let mut holder = Self::dormant(id, directories, start_address, settings);
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
    /// rule 18). Applying keep-awake, the zoom and the identity here — not just
    /// once at construction — is what makes a parked-and-restarted account come
    /// back with the same switches, the same size and the same identity
    /// (`FR.6.1`, `FR.10.5`). The content manager is built with the
    /// frame-callback shim already in its script set when `keep_awake` is on,
    /// because the manager is a construct-only property of the view (`webkit6`
    /// 0.6.1, `web_view.rs:163`) and cannot gain a document-start script
    /// afterwards (`FR.6.3`).
    pub fn start(&mut self) -> &WebView {
        let mut builder = WebView::builder()
            .network_session(&self.network_session)
            .user_content_manager(&build_content_manager(self.keep_awake));
        // Built against the shared context carrying the memory-pressure settings
        // (`FR.19.4`); a missing context means `configure_web_engine` has not
        // run, and the engine's default is the same fallback the cache-model
        // path takes (code standards rule 14).
        if let Some(context) = crate::shared_web_context() {
            builder = builder.web_context(&context);
        } else {
            tracing::warn!("no shared web context; view built on the engine default");
        }
        let view = builder.build();

        configure(&view);
        apply_account_settings(&view, &self.settings);
        if self.keep_awake {
            apply_keep_awake(&view, true, &self.id);
        }
        view.load_uri(&self.start_address);

        self.view.insert(view)
    }

    /// Turns keep-awake on or off for this account: flips the two hidden-page
    /// engine switches on the current view's settings, rebuilds the script set
    /// on its content manager to match, and reloads, since there is no API to
    /// make either change reach a page that has already loaded (`FR.6.2`,
    /// `FR.6.3`, `webkit6` 0.6.1, `src/auto/user_script.rs:22`). Returns the
    /// reloaded view, or `None` while the account is parked.
    ///
    /// Remembers `on` regardless of whether a view exists, so a later
    /// [`SessionView::start`] builds its fresh view with the same switches
    /// already applied (`FR.6.1`).
    pub fn set_keep_awake(&mut self, on: bool) -> Option<&WebView> {
        self.keep_awake = on;
        let view = self.view.as_ref()?;

        apply_keep_awake(view, on, &self.id);
        rebuild_script_set(view, on);
        view.reload();

        Some(view)
    }

    /// Sets this account's page zoom to `zoom`.
    ///
    /// Remembers it on the holder whether or not a view exists, so a parked
    /// account started later opens at the chosen size (`FR.11.5`, `FR.12.7`).
    /// When a view is live it is resized where it stands with **no reload** —
    /// `set_zoom_level` takes effect on the running page (`webkit6` 0.6.1,
    /// `web_view.rs:267` already calls it before the first load), so the
    /// account never leaves `Live`. Returns the live view, or `None` while the
    /// account is parked.
    pub fn set_zoom(&mut self, zoom: ZoomLevel) -> Option<&WebView> {
        self.settings.zoom = zoom;
        let view = self.view.as_ref()?;
        view.set_zoom_level(zoom.multiplier());
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
/// same widget clears itself. Item 06 made this a per-game preset field: a game
/// that genuinely needs a different string gets one in its preset file, tested
/// against that game, and [`apply_account_settings`] applies it — every other
/// account keeps the engine's own identity, which this measurement is the
/// record of (code standards rule 18).
fn configure(view: &WebView) {
    let settings = webkit6::prelude::WebViewExt::settings(view)
        .expect("a web view always has a settings object");
    // Per-site compatibility workarounds WebKit ships. Off by default; a browser
    // ships with them on, and a game that renders wrong without them is a worse
    // failure than the quirk itself.
    settings.set_enable_site_specific_quirks(true);
    // WebGL per account is applied in `apply_account_settings` from the preset
    // (`FR.19.7`); a popup, which never runs a game, keeps the engine default.
    // Right-click -> Inspect Element, for diagnosing a page that will not load
    // or log in — off in a release run unless asked for, because the engine
    // instruments a page differently when something might attach (`FR.19.6`).
    settings.set_enable_developer_extras(diagnostics_enabled());
    // Duplicates the page's own messages, which the bridge already reports, but
    // it is the only source of the engine's: "Blocked a frame with origin ...",
    // a load cancelled by a cross-origin policy, a mixed-content refusal. Those
    // never reach a page's `console` object, so overriding it cannot see them.
    // Folded into the one diagnostics switch rather than a second, differently
    // spelled notion of "we are debugging".
    settings.set_enable_write_console_messages_to_stdout(diagnostics_enabled());

    view.connect_create(|opener, action| Some(open_popup(opener, action)));
    wire_diagnostics(view);
}

/// Applies the two view settings an account carries from its preset: the page
/// zoom, always, and the browser identity **only when the account has one**.
///
/// Both before the first `load_uri` so the page is never drawn at the wrong
/// size or under the wrong identity and then corrected in view. `set_user_agent`
/// rather than `set_user_agent_with_application_details` (`webkit6` 0.6.1,
/// `src/auto/settings.rs:1322` and `:1329`): the appending call cannot produce a
/// string the engine contradicts, which makes it the safer call but not what an
/// override is for — a game that turns the engine away turns away an engine with
/// a suffix too. When `identity` is `None` neither setter is touched and the
/// engine keeps its own (`FR.10.5`, code standards rule 18).
fn apply_account_settings(view: &WebView, account: &AccountSettings) {
    view.set_zoom_level(account.zoom.multiplier());

    let settings = webkit6::prelude::WebViewExt::settings(view)
        .expect("a web view always has a settings object");

    // A graphics context is not free, and which games draw with WebGL is a fact
    // about the game, kept in its preset file (`FR.19.7`). Applied before the
    // first `load_uri` so a page is never drawn with a context it then loses.
    settings.set_enable_webgl(account.webgl_enabled);

    let Some(identity) = account.identity.as_deref() else {
        return;
    };
    settings.set_user_agent(Some(identity));
    tracing::debug!(user_agent = identity, "account identity override applied");
}

/// Copies `opener`'s user-agent string onto `popup`'s settings.
///
/// A popup built with `related-view` shares the opener's network session and
/// web process but gets its own settings object, so an identity override does
/// not reach the sign-in window unless copied here. Copying unconditionally is
/// harmless: with no override the opener already reports the engine's own
/// string and the popup would too (task 05, code standards rule 18).
fn copy_user_agent(opener: &WebView, popup: &WebView) {
    let (Some(from), Some(to)) = (
        webkit6::prelude::WebViewExt::settings(opener),
        webkit6::prelude::WebViewExt::settings(popup),
    ) else {
        return;
    };
    if let Some(user_agent) = from.user_agent() {
        to.set_user_agent(Some(user_agent.as_str()));
    }
}

/// A fresh content manager carrying the page-console bridge, and the
/// frame-callback shim as well when `keep_awake` is on.
///
/// The manager is a construct-only property of the view (`webkit6` 0.6.1,
/// `web_view.rs:163`), so a view that wants the shim from its first load must
/// be built with it already in the set — there is no way to add a
/// document-start script to a view afterwards (`FR.6.3`).
fn build_content_manager(keep_awake: bool) -> UserContentManager {
    let content = UserContentManager::new();
    register_page_console_handler(&content);
    install_script_set(&content, keep_awake);
    content
}

/// Registers the handler [`PAGE_CONSOLE_JS`] posts to, forwarding the page's
/// console output and uncaught errors into `tracing`.
///
/// A page's `console.error` is the page's problem, not the application's, so
/// it arrives as a `warn`; everything quieter than that arrives below the
/// level `make dev` asks for, because one Cloudflare challenge frame alone
/// logs hundreds of lines per load. Registered once per manager — calling
/// this again on a manager that already has the handler would only repeat the
/// warning below for nothing, so [`rebuild_script_set`] never calls it.
fn register_page_console_handler(content: &UserContentManager) {
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
}

/// Adds the manager's document-start scripts: the page-console bridge always,
/// and [`KEEP_AWAKE_JS`] as well when `keep_awake` is on.
///
/// Split out of [`build_content_manager`] so [`rebuild_script_set`] can call
/// it again on a manager that already exists, after
/// [`UserContentManager::remove_all_scripts`] has cleared it — that call
/// clears every script the manager holds, the bridge included, so turning
/// keep-awake off has to put the bridge back rather than leaving the page's
/// console silently unforwarded (code standards rule 18).
fn install_script_set(content: &UserContentManager, keep_awake: bool) {
    content.add_script(&UserScript::new(
        PAGE_CONSOLE_JS,
        UserContentInjectedFrames::AllFrames,
        UserScriptInjectionTime::Start,
        &[],
        &[],
    ));

    if keep_awake {
        content.add_script(&UserScript::new(
            KEEP_AWAKE_JS,
            UserContentInjectedFrames::AllFrames,
            UserScriptInjectionTime::Start,
            &[],
            &[],
        ));
    }
}

/// Replaces `view`'s script set to match `keep_awake`, ready for the reload
/// [`SessionView::set_keep_awake`] performs next.
///
/// `remove_all_scripts` is the only call available and it clears everything
/// on the manager, the page-console bridge included, so every call here
/// rebuilds the full set from scratch rather than only adding or removing the
/// shim — that is what keeps the bridge alive across a keep-awake toggle. The
/// message handler itself is untouched: it was registered once when the view
/// was built and `remove_all_scripts` does not reach it.
fn rebuild_script_set(view: &WebView, keep_awake: bool) {
    let Some(content) = view.user_content_manager() else {
        // Every view is built with a manager in `SessionView::start`; reaching
        // here would mean that construct-only property was somehow absent.
        tracing::warn!("view has no content manager; keep-awake script not applied");
        return;
    };

    content.remove_all_scripts();
    install_script_set(&content, keep_awake);
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

    if diagnostics_enabled() {
        // Two signal handlers per resource, for a failure that almost never
        // comes, on a page that loads resources for as long as it runs
        // (`FR.19.6`, code standards rule 18).
        view.connect_resource_load_started(log_resource);
    }
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
    copy_user_agent(opener, &popup);

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
