//! The Windows implementation of the [`super`] seam: every `WebView2` call
//! the interface needs, through `wry`. [`EngineShared`] wraps the one shared
//! `wry::WebContext` (and the environment every view after the first reuses),
//! [`EngineProfile`] wraps one account's validated profile name, and
//! [`EngineView`] wraps one [`host::EngineHost`] widget hosting a
//! `wry::WebView` as a native child window.

mod ffi;
mod host;

use std::cell::{OnceCell, RefCell};
use std::path::PathBuf;

use gtk::gio;
use gtk::prelude::*;
use gtk4 as gtk;
use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Environment;
use wry::{WebViewBuilder, WebViewBuilderExtWindows, WebViewExtWindows};

use idle_manager_core::{ProfileDirectories, SessionId, ZoomLevel};

use crate::web_engine::ipc_message::{self, IpcAction};
use crate::web_engine::profile_name::is_valid_profile_name;
use crate::web_engine::{CapturedFrame, EngineCaptureError, EngineDeleteError, script_set};
use crate::web_view::{AccountSettings, watched_background};
use ffi::BorrowedHwnd;
pub(crate) use host::EngineHost;
use host::PendingView;

/// The browser arguments every view in the shared environment must agree on
/// — `WebView2` refuses a view whose arguments differ from the environment's
/// (Technical References). `wry`'s own default plus
/// `--disable-backgrounding-occluded-windows`, so a covered or off-grid view
/// is never treated by Chromium as backgrounded on its own (`FR.1.8`).
const BROWSER_ARGS: &str = "--disable-features=msWebOOUI,msPdfOOUI,msSmartScreenProtection --disable-backgrounding-occluded-windows";

/// The folder `WebView2` gives each named profile inside the one shared
/// user-data folder: `<data root>/EBWebView/<profile name>/`. Read only by
/// [`EngineProfile::delete`]'s fallback, for a runtime with no profile-
/// deletion call of its own.
///
/// `TODO(12)`: documented, not measured — no Windows runtime was available
/// to confirm the name when this was written, and task 06's own technical
/// details ask for it to be verified and recorded. The manual step in
/// `docs/tasks/12-windows-support/test-script.md` is what settles it; until
/// then a wrong name here costs a stale folder, never a wrong deletion, since
/// a missing folder counts as already gone.
const PROFILE_SUBFOLDER: &str = "EBWebView";

thread_local! {
    /// The one [`EngineShared`] every account's view is built against. A
    /// `thread_local` rather than a `static OnceLock` because both the
    /// `wry::WebContext` and the cached `ICoreWebView2Environment` are COM /
    /// glib objects and not `Sync` (architecture rule 10); it lives on the
    /// GTK main context, set once by [`EngineShared::configure`] before the
    /// first view.
    static SHARED: OnceCell<EngineShared> = const { OnceCell::new() };
}

/// A view could not be built.
#[derive(Debug, thiserror::Error)]
enum EngineBuildError {
    /// [`EngineShared::configure`] has not run yet.
    #[error("the engine has not been configured")]
    NotConfigured,
    /// `wry` itself refused the view.
    #[error(transparent)]
    Wry(#[from] wry::Error),
}

/// The process-wide engine state: the one `wry::WebContext`, rooted at the
/// data directory `main.rs` resolves through `idle-manager-store`'s
/// `engine_data_root` and hands to [`EngineShared::configure`], every
/// account's view is built against.
#[derive(Debug)]
pub struct EngineShared {
    context: RefCell<wry::WebContext>,
    /// The one user-data folder every account's profile lives inside, as
    /// `main.rs` resolved it. Kept because `wry::WebContext` never hands its
    /// own path back, and [`EngineProfile::delete`]'s fallback needs it to
    /// find one profile's folder. `None` only when the application could not
    /// resolve a data directory at all, in which case `WebView2` picks its
    /// own and the fallback cannot run.
    data_root: Option<PathBuf>,
    /// The first view's environment, cached so every later view can share it
    /// through `with_environment` rather than a fresh `&mut` borrow of
    /// `context` — `wry::WebContext` offers no other way to hand out its
    /// environment once a view already exists (`FR.1.3`, `FR.2.4`).
    environment: RefCell<Option<ICoreWebView2Environment>>,
}

impl EngineShared {
    /// Builds the one `wry::WebContext` every account's view is built
    /// against, rooted at `data_root`. Call once, after GTK is initialised
    /// and before the first web view is built.
    pub(crate) fn configure(data_root: Option<PathBuf>) {
        let shared = EngineShared {
            context: RefCell::new(wry::WebContext::new(data_root.clone())),
            data_root,
            environment: RefCell::new(None),
        };
        SHARED.with(|cell| {
            if cell.set(shared).is_err() {
                tracing::warn!("configure_web_engine ran twice; keeping the first engine state");
            }
        });
    }

    /// Builds one `wry::WebView` for `pending`, parented to `window`, wired
    /// to `on_load` and `on_ipc`.
    ///
    /// The first view ever built takes the kept `wry::WebContext` directly
    /// (`WebViewBuilder::new_with_web_context`) and caches its resulting
    /// environment; every later view reuses that cached environment through
    /// `with_environment` instead, so every account shares one browser
    /// process and one GPU process (`FR.1.3`, `FR.2.4`).
    fn build_view(
        pending: &PendingView,
        window: &BorrowedHwnd,
        on_load: impl Fn(wry::PageLoadEvent, String) + 'static,
        on_ipc: impl Fn(IpcAction) + 'static,
    ) -> Result<wry::WebView, EngineBuildError> {
        SHARED.with(|cell| {
            let shared = cell.get().ok_or(EngineBuildError::NotConfigured)?;

            let cached_environment = shared.environment.borrow().clone();
            if let Some(environment) = cached_environment {
                let builder = configure_builder(
                    WebViewBuilder::new().with_environment(environment),
                    pending,
                    on_load,
                    on_ipc,
                );
                return Ok(builder.build_as_child(window)?);
            }

            let mut context = shared.context.borrow_mut();
            let builder = configure_builder(
                WebViewBuilder::new_with_web_context(&mut context)
                    .with_additional_browser_args(BROWSER_ARGS),
                pending,
                on_load,
                on_ipc,
            );
            let view = builder.build_as_child(window)?;
            drop(context);
            shared.environment.replace(Some(view.environment()));
            Ok(view)
        })
    }
}

/// The folder `profile_name` occupies inside the shared user-data folder,
/// or `None` when [`EngineShared::configure`] never ran or ran without a
/// data directory. Only [`EngineProfile::delete`]'s fallback reads this.
fn profile_folder(profile_name: &str) -> Option<PathBuf> {
    SHARED.with(|cell| {
        let root = cell.get()?.data_root.clone()?;
        Some(root.join(PROFILE_SUBFOLDER).join(profile_name))
    })
}

/// Applies every setting common to a fresh `wry::WebViewBuilder` regardless
/// of which environment it was constructed against: the address, the
/// validated profile name, the account's identity when it has one, the
/// document-start scripts [`script_set::webview2_script_set`] names — the
/// bridge, the page console, the frame shim, and its arming prelude for a
/// keep-awake account — the load-event handler
/// [`host::EngineHost::connect_painted`] fires through, and the ipc handler
/// that parses what the bridge script posts and hands the result to
/// `on_ipc`, silently dropping anything [`ipc_message::parse_ipc_message`]
/// does not recognise (code standards rules 14, 15). Built with zero bounds
/// and hidden until `EngineHost`'s first `size_allocate` places it for real
/// (`FR.11.5`, code standards rule 18).
fn configure_builder<'a>(
    builder: WebViewBuilder<'a>,
    pending: &PendingView,
    on_load: impl Fn(wry::PageLoadEvent, String) + 'static,
    on_ipc: impl Fn(IpcAction) + 'static,
) -> WebViewBuilder<'a> {
    let mut builder = builder
        .with_url(&pending.start_address)
        .with_profile_name(&pending.profile_name)
        .with_bounds(wry::Rect::default())
        .with_on_page_load_handler(on_load)
        .with_ipc_handler(move |request| {
            if let Some(action) = ipc_message::parse_ipc_message(request.body()) {
                on_ipc(action);
            }
        })
        .with_new_window_req_handler(|_uri, _features| {
            // `NewWindowResponse::Create` hands back the *already-created*
            // `ICoreWebView2` WebView2 made for the request — there is no
            // `wry::WebView` to build ourselves the way every other view
            // here is built, so the `EngineHost`-hosted, `POPUP_WIDTH`×
            // `POPUP_HEIGHT` popup window `web_view.rs`'s WebKit backend
            // builds on Linux has no direct Windows equivalent through this
            // hook (Technical References). `Allow` — WebView2's own default
            // popup window — is the deliberate simplification instead: it
            // still opens on the opener's environment and profile (same
            // login, same cookies), just without our own sizing or chrome.
            // `TODO(12)`: revisit if `Create`'s raw `ICoreWebView2` can be
            // reparented into an `EngineHost` after the fact.
            wry::NewWindowResponse::Allow
        });
    if let Some(user_agent) = &pending.user_agent {
        builder = builder.with_user_agent(user_agent);
    }
    for script in script_set::webview2_script_set(pending.keep_awake) {
        builder = builder.with_initialization_script(script);
    }
    builder
}

/// One account's persistent engine profile.
///
/// On Windows there is no per-account network session to hold — every
/// account's storage lives inside the one shared environment, named by
/// `with_profile_name`, not on disk under `directories.data`/`.cache` the way
/// `WebKit`'s `NetworkSession` uses them (`FR.1.4`) — so this is little more
/// than the validated profile name [`EngineProfile::build_view`] hands to
/// [`host::EngineHost::new`].
#[derive(Debug)]
pub struct EngineProfile {
    id: SessionId,
}

impl EngineProfile {
    /// Validates `id` as a `WebView2` profile name (Technical References)
    /// and builds the profile. `directories` is accepted only so this
    /// constructor stays call-compatible with the Linux backend's — Windows
    /// ignores it (`FR.1.4`).
    #[must_use]
    pub(crate) fn new(id: &SessionId, _directories: &ProfileDirectories) -> Self {
        assert!(
            is_valid_profile_name(id.as_str()),
            "session id {id} is not a valid WebView2 profile name"
        );
        Self { id: id.clone() }
    }

    /// Builds a fresh view for `start_address`, applying `settings`'
    /// identity and zoom, hosted by a new [`host::EngineHost`] widget.
    #[must_use]
    pub(crate) fn build_view(
        &self,
        start_address: &str,
        settings: &AccountSettings,
        keep_awake: bool,
    ) -> EngineView {
        // WebView2 exposes no per-view WebGL toggle — Chromium's own
        // `--disable-webgl` is environment-wide, and every account shares one
        // environment (`FR.1.3`), so a game's own preset field cannot be
        // honoured here the way `web_engine/webkit.rs`'s
        // `apply_account_settings` honours it on Linux. Logged, not silently
        // dropped, so a preset that actually depends on this is diagnosable.
        if !settings.webgl_enabled {
            tracing::debug!(
                session = %self.id,
                "this account's preset disables WebGL, which WebView2 cannot do per account"
            );
        }

        let pending = PendingView {
            id: self.id.clone(),
            start_address: start_address.to_owned(),
            user_agent: settings.identity.clone(),
            profile_name: self.id.as_str().to_owned(),
            zoom: settings.zoom.multiplier(),
            keep_awake,
        };
        EngineView {
            host: EngineHost::new(pending),
            id: self.id.clone(),
        }
    }

    /// Deletes everything the engine holds for this account (roadmap item 12
    /// task 06, `FR.21.8`).
    ///
    /// Every account's storage sits inside one shared user-data folder that
    /// the one browser process keeps open for as long as it runs (`FR.1.3`,
    /// `FR.2.4`), so Linux's "clear the account's own session, then remove
    /// its folder" has no equivalent here — removing files from under the
    /// running engine would fail or corrupt what a sibling account is still
    /// using. The engine's own per-profile deletion is the first path, asked
    /// through `view`, the only handle a profile can be reached from.
    ///
    /// The fallback, for a runtime whose `ICoreWebView2Profile8` is missing
    /// (its minimum version is unconfirmed — this task's own context) or for
    /// an account with no live view left to ask through: remove that one
    /// profile's folder inside the shared user-data folder, and nothing else.
    /// Which path ran is logged at `info` with the runtime's version, so the
    /// answer is readable from a session's log (this task's manual criteria).
    pub(crate) async fn delete(&self, view: Option<&EngineView>) -> Result<(), EngineDeleteError> {
        let runtime = wry::webview_version().unwrap_or_else(|_| "unknown".to_owned());

        let unavailable = match view {
            Some(view) => match view.delete_profile().await {
                Ok(()) => {
                    tracing::info!(
                        session = %self.id,
                        path = "engine",
                        runtime,
                        "the engine deleted this account's profile"
                    );
                    return Ok(());
                }
                Err(reason) => reason,
            },
            None => "the account has no live view to reach its profile through".to_owned(),
        };

        tracing::info!(
            session = %self.id,
            path = "fallback",
            runtime,
            reason = unavailable,
            "removing this account's profile folder by hand"
        );
        self.remove_profile_folder().await
    }

    /// Removes `<data root>/EBWebView/<profile name>/` and nothing else, off
    /// the main thread — a folder removal can block on slow storage, and this
    /// runs while the delete-account dialog is up (architecture rule 10).
    ///
    /// A folder already gone counts as deleted, the same rule
    /// `XdgProfileRemoval::remove` keeps, so a retry after a half-finished
    /// attempt can still finish (`FR.21.11`).
    async fn remove_profile_folder(&self) -> Result<(), EngineDeleteError> {
        let Some(folder) = profile_folder(self.id.as_str()) else {
            return Err(EngineDeleteError {
                reason: "the engine has no data folder to delete this account's profile from"
                    .to_owned(),
            });
        };

        let outcome = gio::spawn_blocking(move || match std::fs::remove_dir_all(&folder) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        })
        .await;

        match outcome {
            Ok(Ok(())) => Ok(()),
            Ok(Err(reason)) => Err(EngineDeleteError { reason }),
            Err(_) => Err(EngineDeleteError {
                reason: "an internal error interrupted the deletion".to_owned(),
            }),
        }
    }
}

/// The view of a single account start: an [`host::EngineHost`] widget
/// hosting one `wry::WebView` as a native child window. Built by
/// [`EngineProfile::build_view`]; dropped and rebuilt on every park/start
/// cycle.
#[derive(Debug, Clone)]
pub struct EngineView {
    host: EngineHost,
    /// The account this view belongs to, for the `session` field on every
    /// failure logged from a callback (code standards rule 15).
    id: SessionId,
}

impl EngineView {
    /// The widget the interface places in a slot.
    #[must_use]
    pub(crate) fn widget(&self) -> gtk::Widget {
        self.host.clone().upcast()
    }

    /// Loads `uri` in this view, once it exists.
    // Unused until a later slice needs to navigate a live view somewhere
    // other than its first `start_address` — `PendingView::start_address`
    // covers every caller today, same reasoning as `web_engine/webkit.rs`'s
    // own currently-unused methods.
    #[allow(dead_code)]
    pub(crate) fn load_uri(&self, uri: &str) {
        self.host.load_uri(uri);
    }

    /// Asks the engine to delete the profile this view runs under, resolving
    /// once it reports the profile gone.
    ///
    /// `Err(reason)` says nothing was deleted and why — a runtime without the
    /// call, or a view that was never built — which is
    /// [`EngineProfile::delete`]'s signal to take the folder fallback. The
    /// engine's `Deleted` event is a COM callback, so this bridges it into
    /// the one `await` `account_deletion.rs` runs both engines through, the
    /// same way `web_engine/webkit.rs` bridges `WebKit`'s own async clear.
    async fn delete_profile(&self) -> Result<(), String> {
        gio::GioFuture::new(&self.host, move |host, _cancellable, result| {
            host.delete_profile(move |outcome| result.resolve(outcome));
        })
        .await
    }

    /// Reloads the page currently loaded in this view.
    pub(crate) fn reload(&self) {
        self.host.reload();
    }

    /// Resizes this view to `zoom`, in place, with no reload.
    pub(crate) fn set_zoom(&self, zoom: ZoomLevel) {
        self.host.set_zoom(zoom.multiplier());
    }

    /// A no-op on Windows: every account shares one `WebView2` browser
    /// process (`FR.1.3`), so there is no per-account rendering process to
    /// end the way Linux's `terminate_web_process` ends one. Parking still
    /// frees the account's own native view — `SessionView::stop` drops this
    /// `EngineView` immediately after calling this, and dropping
    /// [`host::EngineHost`] is what actually closes its `wry::WebView`.
    // `&self` kept to stay call-compatible with `web_engine/webkit.rs`'s own
    // `terminate`, which does need it.
    #[allow(clippy::unused_self)]
    pub(crate) fn terminate(&self) {}

    /// Marks whether this view should be treated as backgrounded: `true`
    /// turns `ICoreWebView2Controller::IsVisible` off, which is what actually
    /// carries the per-account keep-awake choice on Windows, since `WebView2`
    /// has no per-view hidden-page switches of its own to flip the way
    /// `web_engine/webkit.rs`'s `apply_keep_awake` does (roadmap item 12 task
    /// 04, `FR.1.9`).
    pub(crate) fn set_background(&self, background: bool) {
        self.host.set_background(background);
    }

    /// Wakes this view for a watching phone (`on`) or hands it back to the
    /// account's own `keep_awake` choice (`FR.4.3`, roadmap item 13 task 02):
    /// the controller is forced visible while watched and put back to
    /// `background_for(minimised, keep_awake)` when the phone leaves
    /// ([`watched_background`]) — `IsVisible` being what carries keep-awake on
    /// Windows at all (`web_engine/webview2.rs`'s `set_background`). The
    /// frame shim is armed and disarmed through [`EngineView::run_script`] as
    /// well, so the runtime flag means the same on both engines.
    pub(crate) fn set_watched(&self, on: bool, keep_awake: bool, minimised: bool) {
        self.host
            .set_background(watched_background(on, keep_awake, minimised));
        self.run_script(&script_set::watched_script(on, keep_awake));
        tracing::debug!(session = %self.id, watched = on, keep_awake, "view watched state set");
    }

    /// Runs `source` in this view's page through `wry::WebView::evaluate_script`
    /// (roadmap item 13 task 02). A failure — or a view not built yet — is
    /// logged with the session field and never surfaced, since nothing the
    /// caller could do differs by the reason.
    pub(crate) fn run_script(&self, source: &str) {
        if let Err(reason) = self.host.run_script(source) {
            tracing::warn!(session = %self.id, reason, "script did not run in the page");
        }
    }

    /// Takes a picture of this view's page as it is right now and hands it
    /// to `done` exactly once, as a JPEG the engine encoded itself
    /// (`ICoreWebView2::CapturePreview`, roadmap item 13 task 02).
    pub(crate) fn capture_frame(
        &self,
        done: impl FnOnce(Result<CapturedFrame, EngineCaptureError>) + 'static,
    ) {
        self.host.capture_frame(move |outcome| {
            done(
                outcome
                    .map(CapturedFrame::Jpeg)
                    .map_err(|reason| EngineCaptureError { reason }),
            );
        });
    }

    /// Grabs keyboard focus for this view's hosted native window.
    // Unused until a later slice calls it, same reasoning as
    // `web_engine/webkit.rs`'s own currently-unused `grab_focus`.
    #[allow(dead_code)]
    pub(crate) fn grab_focus(&self) {
        self.host.focus_view();
    }

    /// Runs `f` once, the first time this view's page finishes loading —
    /// `wry`'s `PageLoadEvent::Finished`, the load event this interface has
    /// always treated as "painted." Self-disconnects after firing, so a
    /// caller never has to track and disconnect a handler by hand before
    /// arming the next one, the same contract `web_engine/webkit.rs`'s
    /// `connect_painted` keeps.
    pub(crate) fn connect_painted(&self, f: impl Fn() + 'static) {
        self.host.connect_painted(f);
    }

    /// Runs `f` on every `ProcessFailed` this view's host reports, for
    /// whatever reason — a live subscription since task 03, through
    /// [`host::EngineHost::connect_terminated`]. No caller registers one yet
    /// on either engine (`web_engine/webkit.rs`'s own `connect_terminated` is
    /// unused the same way) — item 08's crash recovery is what will.
    // Unused until item 08 calls it, same reasoning as `web_engine/webkit.rs`'s
    // own currently-unused `connect_terminated`.
    #[allow(dead_code)]
    pub(crate) fn connect_terminated(&self, f: impl Fn() + 'static) {
        self.host.connect_terminated(f);
    }

    /// Reloads this view — the one part of Linux's own
    /// `EngineView::set_keep_awake` (`rebuild_script_set` plus a reload) that
    /// carries over to a *live* `WebView2` view, since `wry` only accepts an
    /// initialization script at `WebViewBuilder` time (`configure_builder`'s
    /// own doc comment) and offers no way to add or remove one from a view
    /// that already exists (roadmap item 12 task 04, `FR.6.2`, `FR.6.3`).
    /// [`host::PendingView::keep_awake`] is what actually adds
    /// `script_set::KEEP_AWAKE_PRELUDE_JS` — this reload does not by itself
    /// change whether the prelude is present; `SessionView::start` is what next builds the view
    /// with the switch's new value baked in, at the account's next park and
    /// restart. `TODO(12)`: the Windows VM measurement this task's own
    /// context calls for should also settle whether that gap (a live toggle
    /// only fully taking effect on the next restart) is acceptable, or
    /// whether it is worth reaching for the raw, asynchronous
    /// `AddScriptToExecuteOnDocumentCreated`/`RemoveScriptToExecuteOnDocumentCreated`
    /// COM calls instead. Since roadmap item 13 the shim itself is in every
    /// view and only the arming prelude depends on the switch, so the gap is
    /// narrower than it was: the running page can be armed or disarmed at
    /// once through [`EngineView::set_watched`]'s runtime path.
    pub(crate) fn set_keep_awake(&self, _on: bool, id: &SessionId) {
        tracing::debug!(session = %id, "reloading for a keep-awake toggle");
        self.host.reload();
    }
}
