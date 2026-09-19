//! `EngineHost`'s private state (architecture rule 12): the pending view
//! configuration until realize, the built `wry::WebView` after, and the
//! bounds math wired to size-allocate and the toplevel's scale factor.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;
use wry::WebViewExtWindows;

use crate::web_engine::host_bounds::{self, Bounds};
use crate::web_engine::ipc_message::IpcAction;
use crate::web_engine::virtual_key::gdk_key_for_virtual_key;
use crate::window::{Window, ZoomStep};

use super::super::ffi::{self, BorrowedHwnd};
use super::super::{EngineBuildError, EngineShared};
use super::PendingView;

/// A handler queued for the hosted view's first paint, or the marker that it
/// has already fired — [`EngineHost::connect_painted`] fires a handler
/// immediately in that case, matching [`super::super::EngineView::connect_painted`]'s
/// "fires once, ever" contract regardless of when it is called relative to
/// realize.
enum Painted {
    Waiting(Vec<Box<dyn Fn()>>),
    Fired,
}

impl Default for Painted {
    fn default() -> Self {
        Self::Waiting(Vec::new())
    }
}

#[derive(Default)]
pub struct EngineHost {
    pending: RefCell<Option<PendingView>>,
    view: RefCell<Option<wry::WebView>>,
    last_bounds: Cell<Option<Bounds>>,
    painted: RefCell<Painted>,
    /// Handlers registered through [`EngineHost::connect_terminated`]. Unlike
    /// [`Painted`], this can fire more than once over the host's life — a
    /// `ProcessFailed` a browser-process exit can raise again after a
    /// restart — so it is never marked "fired" the way painting is.
    terminated: RefCell<Vec<Box<dyn Fn()>>>,
    /// The last background state [`EngineHost::set_background`] was asked
    /// for, applied to the controller as soon as one exists — remembered
    /// here, not just pushed straight to the controller, because a caller
    /// may set it before the view is even built (roadmap item 12 task 04):
    /// `SessionView::start` calls it on the [`super::EngineHost`] it just
    /// got back from [`EngineHost::new`], which realize has not necessarily
    /// run for yet.
    background: Cell<bool>,
}

#[glib::object_subclass]
impl ObjectSubclass for EngineHost {
    const NAME: &'static str = "IdleManagerEngineHost";
    type Type = super::EngineHost;
    type ParentType = gtk::Widget;
}

impl std::fmt::Debug for EngineHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineHost").finish_non_exhaustive()
    }
}

impl ObjectImpl for EngineHost {
    fn constructed(&self) {
        self.parent_constructed();
        // Off-grid placement clips through the parent overlay's own
        // `set_overflow`, the same as `SessionGrid` on Linux; this widget
        // itself draws nothing that needs clipping.
        self.obj().set_overflow(gtk::Overflow::Hidden);

        let host = self.obj().downgrade();
        self.obj()
            .connect_notify_local(Some("scale-factor"), move |_, _| {
                if let Some(host) = host.upgrade() {
                    host.imp().apply_current_bounds();
                }
            });
    }

    fn dispose(&self) {
        // Dropping `self.view` here closes the `wry::WebView` and its native
        // child window; nothing else in this widget owns a GTK child to
        // unparent (`EngineHost` draws nothing of its own).
        self.view.take();
    }
}

impl WidgetImpl for EngineHost {
    fn realize(&self) {
        self.parent_realize();

        let Some(pending) = self.pending.borrow_mut().take() else {
            // Realized again with nothing new pending: the existing view, if
            // any, is kept exactly as it was.
            return;
        };

        let Some(hwnd) = toplevel_hwnd(&self.obj()) else {
            tracing::error!(
                "EngineHost realized with no Win32 toplevel surface; the game view was not built"
            );
            return;
        };

        match self.build_view(hwnd, &pending) {
            Ok(view) => {
                self.view.replace(Some(view));
                self.apply_current_bounds();
            }
            Err(error) => {
                tracing::error!(
                    %error,
                    session = pending.profile_name,
                    "could not build the WebView2 view"
                );
            }
        }
    }

    fn unrealize(&self) {
        // The view closes with the surface it was parented to; dropping it
        // here rather than waiting for `dispose` keeps a park (which
        // unrealizes without disposing) from leaving a dangling native
        // window behind.
        self.view.take();
        self.parent_unrealize();
    }

    fn size_allocate(&self, width: i32, height: i32, baseline: i32) {
        self.parent_size_allocate(width, height, baseline);
        self.apply_current_bounds();
    }
}

impl EngineHost {
    pub(super) fn set_pending(&self, pending: PendingView) {
        self.pending.replace(Some(pending));
    }

    pub(super) fn connect_painted(&self, f: impl Fn() + 'static) {
        let mut painted = self.painted.borrow_mut();
        match &mut *painted {
            Painted::Fired => f(),
            Painted::Waiting(handlers) => handlers.push(Box::new(f)),
        }
    }

    /// Queues `f` to run on every `ProcessFailed` this host's view reports,
    /// for as long as the host lives — registered here rather than only
    /// after the view exists, the same reasoning as
    /// [`EngineHost::connect_painted`].
    pub(super) fn connect_terminated(&self, f: impl Fn() + 'static) {
        self.terminated.borrow_mut().push(Box::new(f));
    }

    pub(super) fn load_uri(&self, uri: &str) {
        if let Some(view) = self.view.borrow().as_ref()
            && let Err(error) = view.load_url(uri)
        {
            tracing::warn!(%error, "could not load the requested address");
        }
    }

    pub(super) fn reload(&self) {
        if let Some(view) = self.view.borrow().as_ref()
            && let Err(error) = view.reload()
        {
            tracing::warn!(%error, "could not reload the view");
        }
    }

    pub(super) fn set_zoom(&self, zoom: f64) {
        if let Some(view) = self.view.borrow().as_ref()
            && let Err(error) = view.zoom(zoom)
        {
            tracing::warn!(%error, "could not apply zoom");
        }
    }

    pub(super) fn focus_view(&self) {
        if let Some(view) = self.view.borrow().as_ref()
            && let Err(error) = view.focus()
        {
            tracing::warn!(%error, "could not focus the view");
        }
    }

    /// Remembers `background`, then applies it to the live controller if the
    /// view already exists — [`EngineHost::build_view`] applies the
    /// remembered value itself, for the case where this ran first.
    pub(super) fn set_background(&self, background: bool) {
        self.background.set(background);
        if let Some(view) = self.view.borrow().as_ref() {
            apply_background(view, background);
        }
    }

    /// Sizes the hosted view to `0×0` in place — a no-op before the view
    /// exists. `last_bounds` is left untouched, so [`EngineHost::restore`]
    /// still has the shrink-worthy allocation to fall back to if this host's
    /// own `size_allocate` has not run again by the time it is called.
    pub(super) fn collapse(&self) {
        if self.view.borrow().is_none() {
            return;
        }
        let bounds = wry::Rect {
            position: wry::dpi::LogicalPosition::new(0.0, 0.0).into(),
            size: wry::dpi::LogicalSize::new(0.0, 0.0).into(),
        };
        if let Some(view) = self.view.borrow().as_ref()
            && let Err(error) = view.set_bounds(bounds)
        {
            tracing::warn!(%error, "could not collapse the view");
        }
    }

    /// Asks the engine to delete the profile this host's view runs under,
    /// calling `done` exactly once: with `Ok` when the engine reports the
    /// profile gone, or with `Err(reason)` when there is nothing here to ask
    /// — no view built yet — or when the runtime refuses one of the COM
    /// steps ([`ffi::delete_profile`], roadmap item 12 task 06). An
    /// `Err(reason)` is not the end of the deletion: it is the caller's
    /// signal to remove the profile's folder itself, and the reason is what
    /// it logs to say which path ran.
    pub(super) fn delete_profile(&self, done: impl FnOnce(Result<(), String>) + 'static) {
        // `done` behind a shared cell, not moved straight into the
        // subscription: it has to survive `ffi::delete_profile` taking the
        // callback and then failing anyway, which is exactly the case the
        // fallback exists for.
        let done: DeleteOutcome = Rc::new(RefCell::new(Some(Box::new(done))));

        let webview = self
            .view
            .borrow()
            .as_ref()
            .map(wry::WebViewExtWindows::webview);
        let Some(webview) = webview else {
            finish_delete(
                &done,
                Err("this account has no live view to reach its profile through".to_owned()),
            );
            return;
        };

        let on_deleted = {
            let done = Rc::clone(&done);
            move || finish_delete(&done, Ok(()))
        };
        if let Err(error) = ffi::delete_profile(&webview, on_deleted) {
            finish_delete(&done, Err(error.to_string()));
        }
    }

    /// Reads this host's current allocation fresh and applies it — never the
    /// bounds remembered from before [`EngineHost::collapse`], so a host
    /// whose slot moved during a drag restores into its new place.
    pub(super) fn restore(&self) {
        self.apply_current_bounds();
    }

    /// Fires every queued painted handler once, on the hosted view's first
    /// `Finished` load event, and remembers that it has fired so a handler
    /// registered afterward runs immediately instead of waiting forever.
    fn fire_painted(&self) {
        let mut painted = self.painted.borrow_mut();
        let Painted::Waiting(handlers) = &mut *painted else {
            return;
        };
        let handlers = std::mem::take(handlers);
        *painted = Painted::Fired;
        drop(painted);
        for handler in handlers {
            handler();
        }
    }

    /// Runs every registered terminated handler — never marked "fired",
    /// unlike [`EngineHost::fire_painted`], since a browser-process exit can
    /// raise `ProcessFailed` again after a restart (`FR.1.6`).
    fn fire_terminated(&self) {
        for handler in self.terminated.borrow().iter() {
            handler();
        }
    }

    /// Finds the toplevel [`Window`] this host is (or was) parented under, if
    /// any — used to route a Windows-only key event into the same shortcut
    /// handling the GTK key controller already calls (roadmap item 12 task
    /// 03), since a `WebView2` child window's native keyboard messages never
    /// reach that controller.
    fn toplevel_window(&self) -> Option<Window> {
        self.obj().root()?.downcast::<Window>().ok()
    }

    /// Builds the hosted `wry::WebView` for `pending`, parented to `hwnd`,
    /// sharing the process-wide environment through [`EngineShared`]
    /// (`FR.1.3`, `FR.2.4`), then wires the four Windows-only hooks task 03
    /// adds: `WebView2`'s own zoom control turned off, `Ctrl`-accelerator
    /// keys routed to the window's shortcuts, and process failures routed to
    /// [`EngineHost::fire_terminated`]. A failure past the view itself
    /// building is logged, not propagated — the game still shows; only the
    /// one hook that failed to wire is missing (code standards rules 14, 15).
    fn build_view(
        &self,
        hwnd: windows::Win32::Foundation::HWND,
        pending: &PendingView,
    ) -> Result<wry::WebView, EngineBuildError> {
        let window = BorrowedHwnd::new(hwnd);
        let host = self.obj().downgrade();
        let on_load = move |event: wry::PageLoadEvent, _url: String| {
            if matches!(event, wry::PageLoadEvent::Finished)
                && let Some(host) = host.upgrade()
            {
                host.imp().fire_painted();
            }
        };

        // A `glib::WeakRef`, not the `Window` itself: this closure is owned
        // by the `wry::WebView`/`ICoreWebView2`, which lives under this
        // host, which lives under the window — a strong reference here would
        // be a cycle the window could never be dropped out of (code
        // standards rule 18).
        let toplevel = self.toplevel_window().map(|window| window.downgrade());
        let session = pending.id.clone();
        let on_ipc = move |action: IpcAction| {
            let Some(window) = toplevel.as_ref().and_then(glib::object::WeakRef::upgrade) else {
                return;
            };
            match action {
                // Negative is a notch up (`Ctrl`+wheel zooming in), matching
                // `window/imp.rs`'s `connect_zoom_scrolled` convention on
                // Linux exactly, so the two engines can never disagree.
                IpcAction::ZoomStep(delta) if delta < 0 => {
                    window.imp().apply_zoom_step(&session, ZoomStep::In);
                }
                IpcAction::ZoomStep(delta) if delta > 0 => {
                    window.imp().apply_zoom_step(&session, ZoomStep::Out);
                }
                IpcAction::ZoomStep(_) => {}
                IpcAction::PageConsole(entry) => match entry.level.as_str() {
                    "error" => {
                        tracing::warn!(
                            origin = entry.origin,
                            text = entry.text,
                            "page console error"
                        );
                    }
                    "warn" => {
                        tracing::debug!(
                            origin = entry.origin,
                            text = entry.text,
                            "page console warning"
                        );
                    }
                    _ => {
                        tracing::trace!(
                            level = entry.level,
                            origin = entry.origin,
                            text = entry.text,
                            "page console"
                        );
                    }
                },
            }
        };

        let view = EngineShared::build_view(pending, &window, on_load, on_ipc)?;
        if let Err(error) = view.zoom(pending.zoom) {
            tracing::warn!(%error, "could not apply the account's remembered zoom");
        }

        if let Err(error) = ffi::disable_zoom_control(&view) {
            tracing::warn!(%error, "could not disable WebView2's own zoom control");
        }

        apply_background(&view, self.background.get());

        let controller = view.controller();
        let host_for_keys = self.obj().downgrade();
        let key_result = ffi::watch_accelerator_keys(&controller, move |virtual_key| {
            let Some(host) = host_for_keys.upgrade() else {
                return false;
            };
            let Some(window) = host.imp().toplevel_window() else {
                return false;
            };
            let Some(key) = gdk_key_for_virtual_key(virtual_key) else {
                return false;
            };
            window
                .imp()
                .handle_shortcut_key(key, gdk::ModifierType::CONTROL_MASK)
        });
        if let Err(error) = key_result {
            tracing::warn!(%error, "could not subscribe AcceleratorKeyPressed");
        }

        let host_for_failure = self.obj().downgrade();
        let failure_result = ffi::watch_process_failed(&view.webview(), move || {
            if let Some(host) = host_for_failure.upgrade() {
                host.imp().fire_terminated();
            }
        });
        if let Err(error) = failure_result {
            tracing::warn!(%error, "could not subscribe ProcessFailed");
        }

        Ok(view)
    }

    /// Recomputes this host's bounds in window coordinates — its own bounds
    /// against the toplevel's root widget, plus the toplevel's own surface
    /// transform — and applies them to the hosted view, if one exists yet
    /// (roadmap item 12, "Airspace").
    fn apply_current_bounds(&self) {
        if self.view.borrow().is_none() {
            return;
        }
        let obj = self.obj();
        let Some(root) = obj.root() else {
            return;
        };
        let Some(widget_bounds) = obj.compute_bounds(&root) else {
            return;
        };
        let transform = root
            .clone()
            .dynamic_cast::<gtk::Native>()
            .ok()
            .map_or((0.0, 0.0), |native| native.surface_transform());

        let placed = host_bounds::place_bounds(
            Bounds {
                x: f64::from(widget_bounds.x()),
                y: f64::from(widget_bounds.y()),
                width: f64::from(widget_bounds.width()),
                height: f64::from(widget_bounds.height()),
            },
            Bounds {
                x: transform.0,
                y: transform.1,
                width: 0.0,
                height: 0.0,
            },
        );
        self.last_bounds.set(Some(placed));

        let bounds = wry::Rect {
            position: wry::dpi::LogicalPosition::new(placed.x, placed.y).into(),
            size: wry::dpi::LogicalSize::new(placed.width, placed.height).into(),
        };
        if let Some(view) = self.view.borrow().as_ref()
            && let Err(error) = view.set_bounds(bounds)
        {
            tracing::warn!(%error, "could not place the view's bounds");
        }
    }
}

/// The one-shot completion [`EngineHost::delete_profile`] hands to both its
/// own failure paths and the engine's `Deleted` event — whichever gets there
/// first runs it, and [`finish_delete`] makes sure the other finds it gone.
type DeleteOutcome = Rc<RefCell<Option<Box<dyn FnOnce(Result<(), String>)>>>>;

/// Runs `slot`'s completion with `outcome`, once and once only. A second
/// call is a no-op: an engine that raises `Deleted` after the caller already
/// gave up, or a `Delete` that fails after subscribing, must not report an
/// outcome twice.
fn finish_delete(slot: &DeleteOutcome, outcome: Result<(), String>) {
    if let Some(done) = slot.borrow_mut().take() {
        done(outcome);
    }
}

/// Applies `background` to `view` through `wry`'s own `set_visible` — `true`
/// turns the controller's `IsVisible` (and the native child window itself)
/// off, since backgrounded is the opposite of visible (roadmap item 12 task
/// 04, `FR.1.9`). A failure is logged, not propagated — the game still shows
/// either way (code standards rules 14, 15).
fn apply_background(view: &wry::WebView, background: bool) {
    if let Err(error) = view.set_visible(!background) {
        tracing::warn!(%error, "could not apply the account's background state");
    }
}

/// The GTK toplevel's `HWND`, through `gdk4_win32::Win32Surface`, or `None`
/// before the widget has a realized native surface at all.
fn toplevel_hwnd(widget: &super::EngineHost) -> Option<windows::Win32::Foundation::HWND> {
    let native = widget.root()?.dynamic_cast::<gtk::Native>().ok()?;
    let surface = native.surface()?;
    let surface = surface.downcast::<gdk4_win32::Win32Surface>().ok()?;
    Some(surface.handle())
}
