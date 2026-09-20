//! The Linux implementation of the [`super`] seam: every `WebKitGTK` call in
//! the interface, gathered into one module. [`EngineShared`] wraps the shared
//! [`webkit6::WebContext`], [`EngineProfile`] wraps one account's persistent
//! [`webkit6::NetworkSession`], and [`EngineView`] wraps one running
//! [`webkit6::WebView`].

use std::cell::{OnceCell, RefCell};
use std::path::Path;
use std::rc::Rc;

use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk4 as gtk;
use webkit6::prelude::*;
use webkit6::{
    CookiePersistentStorage, Feature, FeatureList, LoadEvent, NavigationAction, NetworkSession,
    PermissionRequest, ScriptDialog, Settings, SnapshotOptions, SnapshotRegion, URIRequest,
    UserContentInjectedFrames, UserContentManager, UserScript, UserScriptInjectionTime,
    WebProcessTerminationReason, WebResource, WebView, WebsiteDataTypes,
};

use idle_manager_core::{ProfileDirectories, SessionId, ZoomLevel};

use crate::web_engine::script_set;
use crate::web_engine::{CapturedFrame, EngineCaptureError};
use crate::web_view::{AccountSettings, watched_background};

/// The memory limit the engine watches its rendering processes against, in
/// mebibytes (`FR.19.4`, code standards rule 5).
// `0` is not "no limit": WebKitGTK's `set_memory_limit` guards its argument
// with `g_return_if_fail(memoryLimit)` — passing `0` logs a GLib critical and
// leaves the engine's own default in place, documented as "the system's RAM
// size with a maximum of 3GB" (confirmed against WebKit's
// `WebKitMemoryPressureSettings.cpp`). That default was silently governing
// every rendering process instead of a number chosen for this application.
//
// The limit is only the base the two fractions below multiply, and what sits
// past the *strict* fraction is not a one-off trim: on every poll a process
// above it runs WebKit's critical release — it deletes all compiled
// JavaScript, destroys decoded image data and collects (WTF
// `MemoryPressureHandler::measurementTimerFired`, WebCore
// `releaseCriticalMemory`). A game held there recompiles its code and
// re-decodes its sprites every poll, forever. The earlier 1024 MiB put strict
// at 512 MiB, and four Huntera accounts in play measured 557-589 MiB private
// each (2026-09-12) — every one sat past it permanently, the JIT workers
// burst every 30 s in step with the poll, and the four rendering processes
// held the machine at roughly 6.6 of its 8 cores. 3072 MiB puts strict at
// 1536 MiB, twice the largest rendering process on record (751 MiB), so only
// a runaway process pays the critical release, and conservative at ~1 GiB,
// whose release is only style and font caches. It equals the engine's own
// default ("the system's RAM size with a maximum of 3GB"), now chosen rather
// than inherited (code standards rule 18).
const WEB_PROCESS_MEMORY_LIMIT_MIB: u32 = 3072;
/// A rendering process's measured working set while its game is actually
/// played — the largest figure on record, one account at 751 MiB
/// (docs/roadmap/05-memory-accounting/README.md). The strict threshold must
/// sit above it or the engine discards the game's compiled code on every poll
/// (see [`WEB_PROCESS_MEMORY_LIMIT_MIB`]'s comment).
#[cfg(test)]
const PLAYED_GAME_WORKING_SET_MIB: u32 = 751;
/// The fraction of the limit at which the engine starts shedding caches it
/// would otherwise keep (`FR.19.4`). The type's own default, kept: past it the
/// engine drops only style, font and selector caches — no compiled code, no
/// decoded images — so crossing it is cheap.
const CONSERVATIVE_PRESSURE_THRESHOLD: f64 = 0.33;
/// The fraction of the limit at which the engine collects harder and drops
/// more (`FR.19.4`) — the expensive release, repeated every poll while the
/// process stays above it. The type's own default, against the 3072 MiB limit.
const STRICT_PRESSURE_THRESHOLD: f64 = 0.5;
/// The kill threshold, held explicitly at `0.0` — disabled. Past a kill
/// threshold the engine ends the rendering process, discarding whatever the
/// game has not sent to its own server, which is exactly the loss `UN.9`
/// exists to prevent; a runaway account is reported by the footer and left to
/// the user (`FR.19.5`, `FR.20.2`, code standards rule 18). This is a
/// deliberate product decision, not a placeholder — never raise it above
/// `0.0` without item 08's crash recovery landing first (see task 06's
/// context in
/// docs/tasks/05-memory-accounting/06-telling-the-engine-it-has-a-limit.md).
const KILL_PRESSURE_THRESHOLD: f64 = 0.0;
/// How often the engine samples a rendering process's footprint, in seconds
/// (`FR.19.4`, code standards rule 5). Kept at the type's own default and set
/// equal to `scripts/memory-report.sh`'s own `DEFAULT_SOAK_INTERVAL_SECS`, so
/// the engine's internal sampling cadence and this project's external
/// measurement cadence agree — a before-and-after soak comparison (task 06's
/// own acceptance criterion) is then comparing samples taken on the same
/// clock.
const WEB_PROCESS_MEMORY_POLL_INTERVAL_SECS: f64 = 30.0;

/// The cookie database file, kept inside the account's data directory so it
/// survives a restart alongside the rest of the profile.
const COOKIE_DB: &str = "cookies.sqlite";

/// The name the injected bridge posts console messages under. It appears in
/// [`script_set::PAGE_CONSOLE_JS`] as `webkit.messageHandlers.pageConsole`; the two spell
/// the same string and have to be changed together.
const PAGE_CONSOLE_HANDLER: &str = "pageConsole";

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

/// Whether the page-console bridge ([`register_page_console_handler`]) should
/// forward a page's `console.*` output into `tracing`.
///
/// Deliberately **not** [`diagnostics_enabled`], even though it gates the same
/// environment variable and the two switches otherwise read the same:
/// [`diagnostics_enabled`] is `true` in every debug build unconditionally, and
/// a debug build (`make dev`) is exactly how this application is actually run
/// for real, day-long accounts (`docs/memory-budget.md`, "round 3") — so
/// reusing it here would leave the leak this switch exists to stop turned on
/// in precisely the scenario that found it. This one reads the same
/// environment variable but never turns on `cfg!(debug_assertions)`-only,
/// because doing so on every `console.log` the whole time an account runs is
/// not a bounded diagnostics cost the way the inspector backend or
/// resource-load logging are — it does not settle (see
/// [`register_page_console_handler`]'s doc comment for the measurement).
fn page_console_forwarding_enabled() -> bool {
    std::env::var_os(DIAGNOSTICS_ENV).is_some()
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

thread_local! {
    /// The one [`EngineShared`] every account's view is built against. A
    /// `thread_local` rather than a `static OnceLock` because `WebContext` is
    /// a glib object and not `Sync` (architecture rule 10); it lives on the
    /// GTK main context, set once by [`EngineShared::configure`] before the
    /// first view.
    static SHARED_WEB_CONTEXT: OnceCell<EngineShared> = const { OnceCell::new() };

    // `Feature` is a glib boxed type and is not `Sync`, so the holder lives on
    // the GTK main context in a `thread_local`, never a `static OnceLock`
    // (architecture rule 10).
    static KEEP_AWAKE_FEATURES: OnceCell<KeepAwakeFeatures> = const { OnceCell::new() };
}

/// The process-wide engine state: the shared [`webkit6::WebContext`] every
/// account's view is built against, carrying the memory-pressure settings
/// (`FR.19.4`). Built once by [`EngineShared::configure`], which must run
/// after GTK is initialised and before the first [`EngineProfile::build_view`]
/// call.
#[derive(Debug, Clone)]
pub struct EngineShared {
    context: webkit6::WebContext,
}

impl EngineShared {
    /// Applies the engine-wide settings the whole application shares. Call
    /// once, after GTK is initialised and before the first web view is built.
    ///
    /// Builds the one [`webkit6::WebContext`] every account's view is
    /// constructed against and sets on it:
    ///
    /// - [`webkit6::CacheModel::DocumentViewer`], the lowest of the three
    ///   cache models, so a parked account's memory returns to the operating
    ///   system rather than staying with the engine (`FR.5.3`);
    /// - the [`webkit6::MemoryPressureSettings`], a construct-only property,
    ///   so the engine sheds caches and collects harder as a rendering
    ///   process approaches its limit — which is why the context can no
    ///   longer be `WebContext::default()` and has to be built (`FR.19.4`).
    ///
    /// The same settings are handed to the one networking process through the
    /// static [`webkit6::NetworkSession::set_memory_pressure_settings`]
    /// (`FR.1.3`).
    pub(crate) fn configure() {
        let mut pressure = webkit6::MemoryPressureSettings::new();
        pressure.set_memory_limit(WEB_PROCESS_MEMORY_LIMIT_MIB);
        pressure.set_conservative_threshold(CONSERVATIVE_PRESSURE_THRESHOLD);
        pressure.set_strict_threshold(STRICT_PRESSURE_THRESHOLD);
        pressure.set_kill_threshold(KILL_PRESSURE_THRESHOLD);
        pressure.set_poll_interval(WEB_PROCESS_MEMORY_POLL_INTERVAL_SECS);

        // Engine-wide, set once, before any session is built (`FR.1.3`).
        let mut for_network = pressure.clone();
        webkit6::NetworkSession::set_memory_pressure_settings(&mut for_network);
        tracing::debug!(
            limit_mib = WEB_PROCESS_MEMORY_LIMIT_MIB,
            conservative = CONSERVATIVE_PRESSURE_THRESHOLD,
            strict = STRICT_PRESSURE_THRESHOLD,
            kill = KILL_PRESSURE_THRESHOLD,
            "memory-pressure settings applied to the networking process"
        );

        let context = webkit6::WebContext::builder()
            .memory_pressure_settings(&pressure)
            .build();
        context.set_cache_model(webkit6::CacheModel::DocumentViewer);

        SHARED_WEB_CONTEXT.with(|cell| {
            if cell.set(EngineShared { context }).is_err() {
                tracing::warn!("configure_web_engine ran twice; keeping the first web context");
            }
        });

        log_engine_features();
    }

    /// The shared engine state [`EngineShared::configure`] built, or `None` if
    /// it has not run yet. [`EngineProfile::build_view`] builds every
    /// account's view against its context so a loaded page runs under the
    /// memory-pressure settings rather than a default context created behind
    /// the application's back (`FR.19.4`).
    fn current() -> Option<Self> {
        SHARED_WEB_CONTEXT.with(|cell| cell.get().cloned())
    }
}

/// One account's persistent engine profile, held so the view can be destroyed
/// and rebuilt without losing the account's storage.
///
/// The [`NetworkSession`] outlives every view [`EngineProfile::build_view`]
/// hands back. Build one with [`EngineProfile::new`].
#[derive(Debug)]
pub struct EngineProfile {
    id: SessionId,
    network_session: NetworkSession,
}

impl EngineProfile {
    /// Builds the profile and its own persistent, isolated network session
    /// rooted at `directories`, without building a view.
    #[must_use]
    pub(crate) fn new(id: &SessionId, directories: &ProfileDirectories) -> Self {
        Self {
            id: id.clone(),
            network_session: build_network_session(&directories.data, &directories.cache),
        }
    }

    /// Builds a fresh view bound to the kept network session, applies the
    /// shared settings and `keep_awake`, and loads `start_address`.
    ///
    /// A new view, never a revived one: `network-session` is construct-only
    /// (`webkit6` 0.6.1, `web_engine/webkit.rs`), so the binding to the kept
    /// storage area cannot be remade on an old view (`FR.5.4`,
    /// code-standards rule 18). Applying keep-awake, the zoom and the
    /// identity here — not just once at construction — is what makes a
    /// parked-and-restarted account come back with the same switches, the
    /// same size and the same identity (`FR.6.1`, `FR.10.5`). The content
    /// manager is built with the frame-callback shim — and, when `keep_awake`
    /// is on, the prelude that arms it — already in its script set, because
    /// the manager is a construct-only property of the view (`webkit6`
    /// 0.6.1) and cannot gain a document-start script afterwards (`FR.6.3`).
    #[must_use]
    pub(crate) fn build_view(
        &self,
        start_address: &str,
        settings: &AccountSettings,
        keep_awake: bool,
    ) -> EngineView {
        let mut builder = WebView::builder()
            .network_session(&self.network_session)
            .user_content_manager(&build_content_manager(keep_awake));
        // Built against the shared context carrying the memory-pressure
        // settings (`FR.19.4`); a missing context means
        // `EngineShared::configure` has not run, and the engine's default is
        // the same fallback the cache-model path takes (code standards
        // rule 14).
        if let Some(shared) = EngineShared::current() {
            builder = builder.web_context(&shared.context);
        } else {
            tracing::warn!("no shared web context; view built on the engine default");
        }
        let view = builder.build();

        configure(&view);
        apply_account_settings(&view, settings);
        if keep_awake {
            apply_keep_awake(&view, true, &self.id);
        }

        let view = EngineView {
            view,
            id: self.id.clone(),
        };
        view.load_uri(start_address);
        view
    }

    /// Deletes everything the engine holds for this account (roadmap item 12
    /// task 06): on Linux, exactly item 11's measured step — a full
    /// [`EngineProfile::clear_website_data`] of the account's own network
    /// session — and nothing more.
    ///
    /// `view` is ignored here and accepted only to stay call-compatible with
    /// the Windows backend's own `delete`, where the account's profile can be
    /// reached only through a live view (`web_engine/webview2.rs`). Always
    /// `Ok`: `clear_website_data` logs what it could not clear rather than
    /// failing the deletion, which is item 11's behaviour, kept unchanged
    /// (`account_deletion::deletion_steps`).
    pub(crate) async fn delete(
        &self,
        _view: Option<&EngineView>,
    ) -> Result<(), super::EngineDeleteError> {
        self.clear_website_data().await;
        Ok(())
    }

    /// Wipes every kind of website data this account's network session holds
    /// — cookies, storage, caches, the lot — through the engine's own async
    /// clear call. Irreversible the moment it completes: this is the step
    /// that makes a deletion attempt unsafe to abandon half way (roadmap item
    /// 11, "Behind the spinner"). A no-op, logged, if the session reports no
    /// data manager — never expected, since a non-ephemeral [`NetworkSession`]
    /// always has one (code standards rule 1).
    async fn clear_website_data(&self) {
        let Some(manager) = self.network_session.website_data_manager() else {
            tracing::warn!(session = %self.id, "no website data manager; nothing to clear");
            return;
        };

        let result = gio::GioFuture::new(&manager, move |manager, cancellable, result| {
            manager.clear(
                WebsiteDataTypes::ALL,
                glib::TimeSpan(0),
                Some(cancellable),
                move |outcome| result.resolve(outcome),
            );
        })
        .await;

        if let Err(error) = result {
            tracing::warn!(session = %self.id, %error, "could not clear website data");
        }
    }
}

/// The view of a single account start: the widget `WebKit` draws to, and the
/// handful of things the interface does to it while it is live. Built by
/// [`EngineProfile::build_view`]; dropped and rebuilt on every park/start
/// cycle.
#[derive(Debug, Clone)]
pub struct EngineView {
    view: WebView,
    /// The account this view belongs to, for the `session` field on every
    /// failure logged from a callback (code standards rule 15).
    id: SessionId,
}

impl EngineView {
    /// The widget the interface places in a slot.
    #[must_use]
    pub(crate) fn widget(&self) -> gtk::Widget {
        self.view.clone().upcast()
    }

    /// Loads `uri` in this view.
    pub(crate) fn load_uri(&self, uri: &str) {
        self.view.load_uri(uri);
    }

    /// Reloads the page currently loaded in this view.
    pub(crate) fn reload(&self) {
        self.view.reload();
    }

    /// Resizes this view to `zoom`, in place, with no reload — `set_zoom_level`
    /// takes effect on the running page (`FR.11.5`, `FR.12.7`).
    pub(crate) fn set_zoom(&self, zoom: ZoomLevel) {
        self.view.set_zoom_level(zoom.multiplier());
    }

    /// Ends this view's rendering process. Dropping the [`EngineView`]
    /// afterwards leaves the engine to decide when — and whether — the
    /// process dies otherwise, which is the difference between memory
    /// returned to the system and memory handed to `WebKit`'s process cache
    /// (`FR.5.2`, code-standards rule 18).
    pub(crate) fn terminate(&self) {
        self.view.terminate_web_process();
    }

    /// Toggles keep-awake on this live view: flips the two hidden-page engine
    /// switches on its settings, rebuilds the script set on its content
    /// manager to match, and reloads, since there is no API to make a script
    /// set change reach a page that has already loaded (`FR.6.2`, `FR.6.3`,
    /// `webkit6` 0.6.1, `src/auto/user_script.rs:22`). The deliberate toggle
    /// keeps its reload on purpose; only [`EngineView::set_watched`] takes the
    /// run-time path (roadmap item 13).
    pub(crate) fn set_keep_awake(&self, on: bool, id: &SessionId) {
        apply_keep_awake(&self.view, on, id);
        rebuild_script_set(&self.view, on);
        self.view.reload();
    }

    /// Wakes this view for a watching phone (`on`) or hands it back to the
    /// account's own `keep_awake` choice, with no reload (`FR.4.3`, roadmap
    /// item 13 task 02): the two hidden-page engine switches go through
    /// [`apply_keep_awake`] with `on || keep_awake`, and the frame shim is
    /// armed or disarmed in the running page through [`EngineView::run_script`]
    /// ([`script_set::watched_script`]). Whether the engine switches take
    /// effect on an already-loaded page is the item's second Blocker; the
    /// shim's runtime flag covers animation frames either way.
    ///
    /// `minimised` reaches only [`EngineView::set_background`], the permanent
    /// no-op on Linux — `WebKit` derives hiddenness from the toplevel itself
    /// — kept so both engines take `watched_background`'s one decision.
    pub(crate) fn set_watched(&self, on: bool, keep_awake: bool, minimised: bool) {
        apply_keep_awake(&self.view, on || keep_awake, &self.id);
        // The document-start set follows too, without a reload: a page the
        // watched account navigates or reloads on its own must come up armed
        // from its first script, since a game that reads `document.hidden`
        // at boot settles into its paused state before any runtime arming
        // reaches it (measured 2026-09-20, item 13 task 08: `hidden0=true`
        // on a page loaded while minimised).
        rebuild_script_set(&self.view, on || keep_awake);
        self.set_background(watched_background(on, keep_awake, minimised));
        self.run_script(&script_set::watched_script(on, keep_awake));
        tracing::debug!(session = %self.id, watched = on, keep_awake, "view watched state set");
    }

    /// Runs `source` in this view's page, in the page's own world (roadmap
    /// item 13 task 02). A failure — a syntax error, a page with no script
    /// context yet — is logged with the session field and never surfaced,
    /// since nothing the caller could do differs by the reason.
    pub(crate) fn run_script(&self, source: &str) {
        let id = self.id.clone();
        self.view.evaluate_javascript(
            source,
            None,
            None,
            None::<&gio::Cancellable>,
            move |result| {
                if let Err(error) = result {
                    tracing::warn!(session = %id, %error, "script failed in the page");
                }
            },
        );
    }

    /// Takes a picture of this view's page as it is right now and hands it
    /// to `done` exactly once, as raw RGBA pixels (roadmap item 13 task 02).
    ///
    /// `webkit_web_view_get_snapshot` paints the visible region in the web
    /// process with compositing layers flattened, independently of the
    /// widget's own on-screen frame — which stops while the toplevel is
    /// minimised — so it is the capture path rather than GTK's widget
    /// rendering (Technical References). Whether it also paints while the
    /// view's activity state says hidden is the item's first Blocker, which
    /// `frame_dump.rs` measures.
    pub(crate) fn capture_frame(
        &self,
        done: impl FnOnce(Result<CapturedFrame, EngineCaptureError>) + 'static,
    ) {
        self.view.snapshot(
            SnapshotRegion::Visible,
            SnapshotOptions::NONE,
            None::<&gio::Cancellable>,
            move |result| {
                done(
                    result
                        .map_err(|error| EngineCaptureError {
                            reason: error.to_string(),
                        })
                        .and_then(|texture| rgba_frame(&texture)),
                );
            },
        );
    }

    /// Marks whether this view should be treated as backgrounded — a no-op
    /// here, always, because `WebKit`'s own hidden-page handling
    /// ([`apply_keep_awake`]) is what already covers minimising on Linux
    /// (roadmap item 12 task 04). Kept call-compatible with
    /// `web_engine/webview2.rs`'s own `set_background`, where the window's
    /// minimise handling in `window/imp.rs` really does call this, on every
    /// live account, for every minimise and restore.
    #[allow(clippy::unused_self)]
    pub(crate) fn set_background(&self, _background: bool) {}

    /// Grabs keyboard focus for this view's widget — a no-op wrapper today
    /// since `GtkWidget`'s own click-to-focus already reaches a `WebKit` view,
    /// kept for a later Windows backend where `WebView2` needs to be told
    /// explicitly.
    // Unused until a later slice's Windows backend needs it, same reasoning
    // as `set_background`.
    #[allow(dead_code)]
    pub(crate) fn grab_focus(&self) {
        self.view.grab_focus();
    }

    /// Runs `f` once, the first time this view's page reaches
    /// [`LoadEvent::Committed`] or [`LoadEvent::Finished`] — the load events
    /// this interface has always treated as "painted" (the cover and the
    /// slot placeholder both drop on the first of the two). Self-disconnects
    /// once fired, so a caller never has to track and disconnect a handler by
    /// hand before arming the next one.
    pub(crate) fn connect_painted(&self, f: impl Fn() + 'static) {
        let handler: Rc<RefCell<Option<glib::SignalHandlerId>>> = Rc::new(RefCell::new(None));
        let handler_for_closure = Rc::clone(&handler);
        let view_for_disconnect = self.view.clone();
        let signal_id = self.view.connect_load_changed(move |_, event| {
            if !matches!(event, LoadEvent::Committed | LoadEvent::Finished) {
                return;
            }
            if let Some(signal_id) = handler_for_closure.borrow_mut().take() {
                view_for_disconnect.disconnect(signal_id);
            }
            f();
        });
        *handler.borrow_mut() = Some(signal_id);
    }

    /// Runs `f` whenever this view's rendering process ends, for whatever
    /// reason — kept for a later slice's automatic crash reload (item 08) to
    /// attach to; [`wire_diagnostics`] already logs every termination on its
    /// own, independent connection to the same signal.
    // Unused until item 08's crash reload lands, same reasoning as
    // `set_background`.
    #[allow(dead_code)]
    pub(crate) fn connect_terminated(&self, f: impl Fn() + 'static) {
        self.view
            .connect_web_process_terminated(move |_, _reason| f());
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
    wire_inspector_key(view);
}

/// Binds F12 to open the inspector, when diagnostics are on.
///
/// `WebKitGTK` gives no key binding of its own for this — F12 and
/// Ctrl+Shift+I are conventions each browser wires up itself, not something
/// the engine or GTK provide — and a page's own right-click handler can call
/// `preventDefault()` on `contextmenu`, which suppresses `WebKit`'s native
/// "Inspect Element" item the same way it would in a real browser. Without
/// this, `set_enable_developer_extras` turns on an inspector backend with no
/// way to reach it (`FR.19.6`).
fn wire_inspector_key(view: &WebView) {
    let view_for_key = view.clone();
    let controller = gtk::EventControllerKey::new();
    controller.connect_key_pressed(move |_, key, _, _| {
        if key != gtk::gdk::Key::F12 || !diagnostics_enabled() {
            return glib::Propagation::Proceed;
        }
        if let Some(inspector) = view_for_key.inspector() {
            inspector.show();
        }
        glib::Propagation::Stop
    });
    view.add_controller(controller);
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

/// A fresh content manager carrying [`script_set::webkit_script_set`]: the
/// page-console bridge, the frame-callback shim, and the prelude that arms it
/// when `keep_awake` is on.
///
/// The manager is a construct-only property of the view (`webkit6` 0.6.1),
/// so a view that wants the prelude from its first load must be built with it
/// already in the set — there is no way to add a document-start script to a
/// view afterwards (`FR.6.3`).
fn build_content_manager(keep_awake: bool) -> UserContentManager {
    let content = UserContentManager::new();
    if page_console_forwarding_enabled() {
        register_page_console_handler(&content);
    }
    install_script_set(&content, keep_awake);
    content
}

/// Registers the handler [`PAGE_CONSOLE_JS`] posts to, forwarding the page's
/// console output and uncaught errors into `tracing`.
///
/// Gated behind [`page_console_forwarding_enabled`] — off unless
/// `IDLE_MANAGER_DIAGNOSTICS` is set, in *every* build profile. Measured
/// (task 05, round 3 of the memory investigation, `docs/memory-budget.md`) to
/// be the shell process's actual leak: no-opping this callback body entirely
/// on a live account (Huntera, 4 accounts, 9-minute soak) took the shell's
/// own PSS growth from roughly 690-1030 MB/hour down to about 0.45 MiB over
/// 7+ minutes — noise-level flat. `PAGE_CONSOLE_JS`'s own guard
/// (`if (!handler) return`) means the page never even installs the
/// `console.*` overrides when this handler is not registered, so turning
/// this off removes the cost at both ends, not just the host side.
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

/// Adds the manager's document-start scripts — exactly
/// [`script_set::webkit_script_set`], in its order, every one at document
/// start in all frames.
///
/// Split out of [`build_content_manager`] so [`rebuild_script_set`] can call
/// it again on a manager that already exists, after
/// [`UserContentManager::remove_all_scripts`] has cleared it — that call
/// clears every script the manager holds, the bridge included, so turning
/// keep-awake off has to put the bridge back rather than leaving the page's
/// console silently unforwarded (code standards rule 18).
fn install_script_set(content: &UserContentManager, keep_awake: bool) {
    for source in script_set::webkit_script_set(keep_awake) {
        content.add_script(&UserScript::new(
            source,
            UserContentInjectedFrames::AllFrames,
            UserScriptInjectionTime::Start,
            &[],
            &[],
        ));
    }
}

/// `texture` as straight (not premultiplied) RGBA rows, the shape
/// `CapturedFrame::Rgba` promises.
///
/// Through a [`gdk::TextureDownloader`] asked for
/// [`gdk::MemoryFormat::R8g8b8a8`], not `Texture::download`: that older call
/// only ever writes Cairo's `ARGB32`, which is premultiplied and, on this
/// machine's byte order, `B G R A` — every consumer would have to swizzle
/// and un-premultiply it. The downloader converts in GDK instead (GTK 4.10,
/// the `v4_10` feature the workspace already enables). A texture reporting a
/// non-positive size is an engine bug, not a frame, and is returned as an
/// error rather than an empty picture.
fn rgba_frame(texture: &gdk::Texture) -> Result<CapturedFrame, EngineCaptureError> {
    let (Ok(width), Ok(height)) = (
        u32::try_from(texture.width()),
        u32::try_from(texture.height()),
    ) else {
        return Err(EngineCaptureError {
            reason: format!(
                "the snapshot reported an impossible size {}×{}",
                texture.width(),
                texture.height()
            ),
        });
    };

    let mut downloader = gdk::TextureDownloader::new(texture);
    downloader.set_format(gdk::MemoryFormat::R8g8b8a8);
    let (bytes, stride) = downloader.download_bytes();
    let stride = u32::try_from(stride).map_err(|_| EngineCaptureError {
        reason: format!("the snapshot's row stride {stride} does not fit a frame"),
    })?;

    Ok(CapturedFrame::Rgba {
        width,
        height,
        stride,
        bytes: bytes.to_vec(),
    })
}

/// Replaces `view`'s script set to match `keep_awake`, ready for the reload
/// [`EngineView::set_keep_awake`] performs next.
///
/// `remove_all_scripts` is the only call available and it clears everything
/// on the manager, the page-console bridge included, so every call here
/// rebuilds the full set from scratch rather than only adding or removing the
/// shim — that is what keeps the bridge alive across a keep-awake toggle. The
/// message handler itself is untouched: it was registered once when the view
/// was built and `remove_all_scripts` does not reach it.
fn rebuild_script_set(view: &WebView, keep_awake: bool) {
    let Some(content) = view.user_content_manager() else {
        // Every view is built with a manager in `EngineProfile::build_view`;
        // reaching here would mean that construct-only property was somehow
        // absent.
        tracing::warn!("view has no content manager; keep-awake prelude not applied");
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
/// `Settings` object. Looked up once by identifier because the feature list
/// is a build-time property of the engine and cannot change while the
/// process runs, so searching it per view would repeat work for nothing.
struct KeepAwakeFeatures {
    hidden_page_timer_throttling: Option<Feature>,
    hidden_page_css_animation_suspension: Option<Feature>,
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
/// Call once, at shell start-up as the last step of [`EngineShared::configure`]:
/// the feature list is a build-time property of the engine, so walking it per
/// view would repeat work that can never change while the process runs
/// (architecture rules 10, 12). The printing stays permanently — it is how the
/// two identifiers above get re-read after the next engine upgrade.
fn log_engine_features() {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Past the strict threshold the engine deletes a page's compiled code and
    /// decoded images on every poll; a game in play must never sit there.
    #[test]
    fn the_strict_pressure_threshold_sits_above_a_played_games_working_set() {
        let strict_mib = f64::from(WEB_PROCESS_MEMORY_LIMIT_MIB) * STRICT_PRESSURE_THRESHOLD;

        assert!(
            strict_mib >= 2.0 * f64::from(PLAYED_GAME_WORKING_SET_MIB),
            "strict threshold {strict_mib} MiB must be at least twice the \
             {PLAYED_GAME_WORKING_SET_MIB} MiB a played game's rendering process uses"
        );
    }
}
