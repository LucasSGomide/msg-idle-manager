//! The shell's one `unsafe` module (code standards rule 28, as amended for
//! roadmap item 12): every direct Win32 or COM call the Windows engine needs,
//! and nothing else. Every `unsafe` block carries a `// SAFETY:` line, and
//! only safe functions are exposed.
//!
//! [`EngineHost`]: super::host::EngineHost

#![allow(unsafe_code)]

use gtk::gdk;
use gtk::glib;
use gtk4 as gtk;
use raw_window_handle::{
    HandleError, HasWindowHandle, RawWindowHandle, Win32WindowHandle, WindowHandle,
};
use webview2_com::Microsoft::Web::WebView2::Win32::{
    COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_JPEG, COREWEBVIEW2_KEY_EVENT_KIND,
    COREWEBVIEW2_KEY_EVENT_KIND_KEY_DOWN, COREWEBVIEW2_PHYSICAL_KEY_STATUS,
    COREWEBVIEW2_PROCESS_FAILED_KIND, COREWEBVIEW2_PROCESS_FAILED_KIND_BROWSER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_EXITED,
    COREWEBVIEW2_PROCESS_FAILED_KIND_RENDER_PROCESS_UNRESPONSIVE, ICoreWebView2, ICoreWebView2_13,
    ICoreWebView2Controller, ICoreWebView2Profile8,
};
use webview2_com::{
    AcceleratorKeyPressedEventHandler, CapturePreviewCompletedHandler, ProcessFailedEventHandler,
    ProfileDeletedEventHandler,
};
use windows::Win32::Foundation::{HGLOBAL, HWND};
use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
use windows::Win32::System::Com::{IStream, STATFLAG_NONAME, STATSTG, STREAM_SEEK_SET};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetKeyState, VK_CONTROL, VK_SHIFT};
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

/// Subscribes `AcceleratorKeyPressed` on `controller`. For a key-down, it
/// reads the held modifiers and the Win32 virtual key and calls `on_key` with
/// both; `on_key` returning `true` — it consumed the key — marks the event
/// handled, so the page underneath never also sees it (`FR.1.7`). Every other
/// key event is left alone, for `WebView2` to handle as it normally would.
///
/// A key already down when this fires again — the platform's own auto-repeat,
/// reported here as `PhysicalKeyStatus().WasKeyDown` rather than through a
/// latch the way GTK's key controller needs one (GTK 4 exposes no repeat flag
/// of its own, code standards rule 18) — is dropped before `on_key` ever sees
/// it, so a shortcut held down runs once.
///
/// `on_key` is `'static` and owns whatever it needs (a `glib::WeakRef`, code
/// standards rule 18) — this subscription outlives the call that installs it,
/// for as long as the controller itself does.
pub(super) fn watch_accelerator_keys(
    controller: &ICoreWebView2Controller,
    on_key: impl Fn(u32, gdk::ModifierType) -> bool + 'static,
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
        // SAFETY: `VK_SHIFT` is a valid, constant virtual-key code, and
        // `GetKeyState` has no further preconditions — same call as `Ctrl`'s
        // above, whose behaviour with this key is item 14's second Blocker,
        // settled only in the Windows VM.
        let shift_held = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
        let mut modifiers = gdk::ModifierType::empty();
        if ctrl_held {
            modifiers |= gdk::ModifierType::CONTROL_MASK;
        }
        if shift_held {
            modifiers |= gdk::ModifierType::SHIFT_MASK;
        }

        let mut status = COREWEBVIEW2_PHYSICAL_KEY_STATUS::default();
        // SAFETY: same `args`, same single-call-per-event contract as above;
        // `status` is a valid, correctly typed out-pointer.
        unsafe {
            args.PhysicalKeyStatus(&raw mut status)?;
        }
        if status.WasKeyDown.as_bool() {
            return Ok(());
        }

        let mut virtual_key = 0u32;
        // SAFETY: same `args`, same single-call-per-event contract as above.
        unsafe {
            args.VirtualKey(&raw mut virtual_key)?;
        }

        if on_key(virtual_key, modifiers) {
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

/// Asks `WebView2` for a JPEG of `webview`'s page as it is right now, running
/// `on_captured` once with the encoded bytes — or with one line saying why
/// there are none (roadmap item 13 task 02, `FR.4.3`).
///
/// `CapturePreview` writes into an `IStream` the caller supplies; a memory
/// stream from `CreateStreamOnHGlobal` keeps the round trip on the heap and
/// off the disk. The environment already runs with
/// `--disable-backgrounding-occluded-windows` (`BROWSER_ARGS`), which is what
/// should keep the page painting while covered; whether it also paints while
/// the window is minimised is the item's first Blocker, which `frame_dump.rs`
/// measures. `on_captured` runs through
/// `glib::MainContext::default().invoke_local` (architecture rule 10), since
/// the completion is a COM callback.
pub(super) fn capture_preview(
    webview: &ICoreWebView2,
    on_captured: impl FnOnce(Result<Vec<u8>, String>) + 'static,
) -> windows::core::Result<()> {
    // SAFETY: a null `HGLOBAL` asks COM to allocate the stream's own memory,
    // and `true` hands that memory to the stream to free with itself, so
    // nothing here owns a raw allocation.
    let stream = unsafe { CreateStreamOnHGlobal(HGLOBAL(std::ptr::null_mut()), true)? };

    let stream_for_read = stream.clone();
    let handler = CapturePreviewCompletedHandler::create(Box::new(move |result| {
        let outcome = result
            .map_err(|error| error.to_string())
            .and_then(|()| read_whole_stream(&stream_for_read));
        glib::MainContext::default().invoke_local(move || on_captured(outcome));
        Ok(())
    }));

    // SAFETY: `webview` is the live `ICoreWebView2` behind a `wry::WebView`
    // this module built, `stream` is the valid memory stream created above,
    // and `handler` is a freshly created, single-owner callback object of
    // exactly the type `CapturePreview` expects; the engine keeps its own
    // references to both for as long as the capture runs.
    unsafe {
        webview.CapturePreview(
            COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_JPEG,
            &stream,
            &handler,
        )
    }
}

/// Everything `stream` holds, from its start: its size from `Stat`, then a
/// rewind, then `Read` until the stream reports nothing more — the engine
/// leaves the position at the end of what it wrote.
fn read_whole_stream(stream: &IStream) -> Result<Vec<u8>, String> {
    let mut stat = STATSTG::default();
    // SAFETY: `stat` is a valid, correctly typed out-pointer, and `NONAME`
    // asks the stream not to allocate a name this function would otherwise
    // have to free.
    unsafe { stream.Stat(&raw mut stat, STATFLAG_NONAME) }.map_err(|error| error.to_string())?;
    let size = usize::try_from(stat.cbSize).map_err(|_| {
        format!(
            "the captured stream's {} bytes do not fit in memory",
            stat.cbSize
        )
    })?;

    // SAFETY: rewinding to offset 0 from the start has no preconditions; the
    // new position is not wanted, so no out-pointer is passed.
    unsafe { stream.Seek(0, STREAM_SEEK_SET, None) }.map_err(|error| error.to_string())?;

    let mut bytes = vec![0u8; size];
    let mut filled = 0usize;
    while filled < size {
        let remaining = u32::try_from(size - filled).unwrap_or(u32::MAX);
        let mut read = 0u32;
        // SAFETY: the destination is the unread tail of `bytes`, which is at
        // least `remaining` bytes long, and `read` is a valid out-pointer the
        // stream writes the count it actually copied into.
        let status = unsafe {
            stream.Read(
                bytes[filled..].as_mut_ptr().cast(),
                remaining,
                Some(&raw mut read),
            )
        };
        status.ok().map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        filled += read as usize;
    }
    bytes.truncate(filled);
    Ok(bytes)
}
