//! The shell's one `unsafe` module (code standards rule 28, as amended for
//! roadmap item 12): every direct Win32 or COM call the Windows engine needs,
//! and nothing else. Every `unsafe` block carries a `// SAFETY:` line, and
//! only safe functions are exposed.
//!
//! [`EngineHost`]: super::host::EngineHost

#![allow(unsafe_code)]

use gtk::glib;
use gtk4 as gtk;
use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_KEY_EVENT_KIND, COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN,
    COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE, ICoreWebView2, ICoreWebView2_13,
    ICoreWebView2Controller, ICoreWebView2Profile8,
};
use webview2_com::{
    AcceleratorKeyPressedEventHandler, ProcessFailedEventHandler, ProfileDeletedEventHandler,
};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL};
use windows::core::Interface;
use wry::WebViewExtWindows;

/// Borrows `hwnd` as a [`HasWindowHandle`] for exactly the duration of one
/// `WebViewBuilder::build_as_child` call — `wry` never stores the reference
/// past that call, and the GTK toplevel that owns `hwnd` is realized before
/// `EngineHost` itself is, and outlives it.
pub(super) struct BorrowedHwnd(HWND);

impl BorrowedHwnd {
    pub(super) fn new(hwnd: HWND) -> Self {
        Self(hwnd)
    }
}

impl HasWindowHandle for BorrowedHwnd {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        let Some(hwnd) = std::num::NonZeroIsize::new(self.0.0 as isize) else {
            return Err(HandleError::Unavailable);
        };
        let handle = Win32WindowHandle::new(hwnd);
        // SAFETY: `self.0` came from `gdk4_win32::Win32Surface::handle` on a
        // realized toplevel surface, so it is a valid, non-dangling `HWND`
        // for the lifetime of this borrow.
        Ok(unsafe { WindowHandle::borrow_raw(RawWindowHandle::Win32(handle)) })
    }
}

/// Turns off `WebView2`'s own zoom control, so it never adds to the
/// application's own zoom steps (`FR.1.7`) — the two would otherwise both
/// answer `Ctrl`+wheel, doubling every notch.
pub(super) fn disable_zoom_control(view: &wry::WebView) -> windows::core::Result<()> {
    let webview = view.webview();
    // SAFETY: `webview` is the live `ICoreWebView2` behind a `wry::WebView`
    // this module just built; `Settings` and `SetIsZoomControlEnabled` are
    // ordinary COM property calls on it with no further preconditions.
    unsafe { webview.Settings()?.SetIsZoomControlEnabled(false) }
}

/// Subscribes `AcceleratorKeyPressed` on `controller`. For a key-down held
/// with `Ctrl`, it maps the Win32 virtual key and calls `on_key` with it;
/// `on_key` returning `true` — it consumed the key — marks the event handled,
/// so the page underneath never also sees it (`FR.1.7`). Every other key
/// event is left alone, for `WebView2` to handle as it normally would.
///
/// `on_key` is `'static` and owns whatever it needs (a `glib::WeakRef`, code
/// standards rule 18) — this subscription outlives the call that installs it,
/// for as long as the controller itself does.
pub(super) fn watch_accelerator_keys(
    controller: &ICoreWebView2Controller,
    on_key: impl Fn(u32) -> bool + 'static,
) -> windows::core::Result<()> {
    let handler = AcceleratorKeyPressedEventHandler::create(Box::new(move |_controller, args| {
        let Some(args) = args else {
            return Ok(());
        };

        let mut kind = COREWEBVIEW2_KEY_EVENT_KIND::default();
        // SAFETY: `args` is the live event-args COM object `WebView2` handed
        // this callback for the event it is currently firing; every getter
        // below writes through a valid, correctly typed out-pointer, exactly
        // as the API requires.
        unsafe {
            args.KeyEventKind(&raw mut kind)?;
        }
        if kind != COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN {
            return Ok(());
        }

        // `AcceleratorKeyPressedEventArgs` carries no modifier state of its
        // own; `GetKeyState` reads the key's state as of the last message the
        // thread's own queue processed, which for a key currently held is
        // "down" (high bit set) — the same source Win32 accelerator handling
        // itself uses.
        // SAFETY: `VK_CONTROL` is a valid, constant virtual-key code;
        // `GetKeyState` has no further preconditions.
        let ctrl_held = unsafe { GetKeyState(i32::from(VK_CONTROL.0)) } < 0;
        if !ctrl_held {
            return Ok(());
        }

        let mut virtual_key = 0u32;
        // SAFETY: same `args`, same single-call-per-event contract as above.
        unsafe {
            args.VirtualKey(&raw mut virtual_key)?;
        }

        if on_key(virtual_key) {
            // SAFETY: same `args`.
            unsafe {
                args.SetHandled(true)?;
            }
        }
        Ok(())
    }));

    let mut token = 0i64;
    // SAFETY: `controller` is the live controller for a `wry::WebView` this
    // module built, and `handler` is a freshly created, single-owner callback
    // object of exactly the type `add_AcceleratorKeyPressed` expects.
    unsafe { controller.add_AcceleratorKeyPressed(&handler, &raw mut token) }
}

/// Subscribes `ProcessFailed` on `webview`. `RenderProcessExited`,
/// `RenderProcessUnresponsive` and `BrowserProcessExited` run `on_failed`,
/// mirroring the `WebProcessTerminationReason` branch `web_engine/webkit.rs`
/// already has on Linux (this replaces task 02's stub and lands in that same
/// terminated branch) — a browser-process exit fires it for every account
/// sharing the environment (`FR.1.6`). Every other kind (a GPU or utility
/// process, for instance) is only logged: it is not this account's page that
/// failed. `on_failed` runs through `glib::MainContext::default().invoke_local`
/// (architecture rule 10) since `ProcessFailed` is a COM event `WebView2` can
/// raise from off the GTK main context.
pub(super) fn watch_process_failed(
    webview: &webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2,
    on_failed: impl Fn() + 'static,
) -> windows::core::Result<()> {
    let on_failed = std::rc::Rc::new(on_failed);
    let handler = ProcessFailedEventHandler::create(Box::new(move |_webview, args| {
        let Some(args) = args else {
            return Ok(());
        };

        let mut kind = COREWEBVIEW2_PROCESS_FAILED_KIND::default();
        // SAFETY: `args` is the live event-args COM object `WebView2` handed
        // this callback for the event it is currently firing; `ProcessFailedKind`
        // writes through a valid, correctly typed out-pointer.
        unsafe {
            args.ProcessFailedKind(&raw mut kind)?;
        }

        match kind {
            COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED
            | COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED
            | COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE => {
                tracing::error!(?kind, "web process terminated");
                let on_failed = std::rc::Rc::clone(&on_failed);
                glib::MainContext::default().invoke_local(move || on_failed());
            }
            other => tracing::debug!(kind = ?other, "process failed outside the page itself"),
        }
        Ok(())
    }));

    let mut token = 0i64;
    // SAFETY: `webview` is the live `ICoreWebView2` behind a `wry::WebView`
    // this module built, and `handler` is a freshly created, single-owner
    // callback object of exactly the type `add_ProcessFailed` expects.
    unsafe { webview.add_ProcessFailed(&handler, &raw mut token) }
}

/// Asks `WebView2` to delete the profile `webview` runs under, running
/// `on_deleted` once the engine reports it gone (roadmap item 12 task 06,
/// `FR.21.8`).
///
/// Every account's storage lives inside one shared user-data folder, held
/// open by the one browser process every account shares (`FR.1.3`,
/// `FR.2.4`), so removing an account's files from under the running engine
/// is not an option the way it is on Linux — this call is the engine's own
/// way to drop exactly one profile and leave every sibling untouched.
///
/// Three steps this runtime may refuse: the cast to `ICoreWebView2_13` (the
/// interface carrying `Profile`), the cast to `ICoreWebView2Profile8` (the
/// one carrying `Delete`, whose minimum runtime version is unconfirmed —
/// task 06's own context), and the `Deleted` subscription. An `Err` from any
/// of them is the caller's signal to take the folder fallback, and nothing
/// is half-deleted by one: `Delete` is the last call made, so a refusal
/// always happens before anything is destroyed.
///
/// `on_deleted` runs through `glib::MainContext::default().invoke_local`
/// (architecture rule 10), since `Deleted` is a COM event `WebView2` may
/// raise from off the GTK main context.
pub(super) fn delete_profile(
    webview: &ICoreWebView2,
    on_deleted: impl FnOnce() + 'static,
) -> windows::core::Result<()> {
    let webview: ICoreWebView2_13 = webview.cast()?;
    // SAFETY: `webview` is the live `ICoreWebView2_13` this module just cast
    // from the `ICoreWebView2` behind a `wry::WebView` it built; `Profile`
    // writes through a valid, correctly typed out-pointer, exactly as the
    // API requires.
    let profile = unsafe { webview.Profile()? };
    let profile: ICoreWebView2Profile8 = profile.cast()?;

    // The handler is an `Fn` the engine owns, but `on_deleted` may run only
    // once: taken out of the cell on the first `Deleted`, so a second event
    // — or one raised after the profile is already gone — finds nothing left
    // to run.
    let on_deleted = std::cell::RefCell::new(Some(on_deleted));
    let handler = ProfileDeletedEventHandler::create(Box::new(move |_profile, _args| {
        if let Some(on_deleted) = on_deleted.borrow_mut().take() {
            glib::MainContext::default().invoke_local(on_deleted);
        }
        Ok(())
    }));

    let mut token = 0i64;
    // SAFETY: `profile` is the live `ICoreWebView2Profile8` cast above, and
    // `handler` is a freshly created, single-owner callback object of
    // exactly the type `add_Deleted` expects.
    unsafe { profile.add_Deleted(&handler, &raw mut token)? };
    // SAFETY: same `profile`, subscribed for its `Deleted` event just above.
    // `Delete` takes no arguments and has no further preconditions; the
    // engine closes the profile's own views itself as part of it.
    unsafe { profile.Delete() }
}
