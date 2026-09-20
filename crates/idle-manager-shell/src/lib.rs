//! The GTK 4 adapter: windows, the session grid, the sidebar, and the web
//! views that host each game, drawn by whichever engine `web_engine` selects
//! at compile time (`WebKitGTK` on Linux).
//!
//! Everything that touches a widget lives here and runs on the GTK main
//! context. The shell reads domain state and emits intents back to it; it never
//! decides what a session's state should become.

// The gtk4-rs `object_subclass` and `wrapper!` macros expand to `unsafe impl`
// blocks for glib's type-system traits, and its `ObjectSubclass` convention
// requires the private `imp` type be `pub` inside its module. Both are relaxed
// for this crate only, never for the workspace (code-standards rule 28); every
// macro used here is a vetted gtk4-rs entry point.
#![allow(unsafe_code, unreachable_pub)]

mod account_deletion;
mod add_game_dialog;
mod delete_account_dialog;
mod frame_dump;
mod memory_footer;
mod message_strip;
mod phone_dialog;
mod rename_dialog;
mod save_on_change;
mod session_grid;
mod session_sidebar;
mod slot_placeholder;
mod start_queue;
mod web_engine;
mod web_view;
mod window;

use gtk::gio;
use gtk::glib;
#[cfg(windows)]
use gtk::prelude::*;
use gtk4 as gtk;

pub use window::{PhonePorts, Window, WindowPorts};

/// The compiled-in UI resource bundle could not be registered.
#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    /// The `GResource` data linked into the binary at build time is not valid.
    #[error("the compiled UI resource bundle is invalid")]
    Invalid(#[from] glib::Error),
}

/// Registers the compiled-in UI templates so composite widgets can load them.
///
/// Call once, before the first widget is constructed. It needs no display
/// server.
///
/// # Errors
///
/// [`ResourceError::Invalid`] if the bundle linked at build time is corrupt —
/// a build problem, not a runtime one.
pub fn register_resources() -> Result<(), ResourceError> {
    let bytes = glib::Bytes::from_static(include_bytes!(concat!(
        env!("OUT_DIR"),
        "/idle-manager.gresource"
    )));
    let resource = gio::Resource::from_data(&bytes)?;
    gio::resources_register(&resource);
    Ok(())
}

/// Applies the engine-wide settings the whole application shares. Call once,
/// after GTK is initialised and before the first web view is built.
///
/// A thin entry point onto [`web_engine::EngineShared::configure`] — kept
/// here as the crate's one public start-up call so `main.rs` never has to
/// know which engine, or which of its types, backs a session's view
/// (architecture rules 3, 6).
#[cfg(target_os = "linux")]
pub fn configure_web_engine() {
    web_engine::EngineShared::configure();
}

/// Applies the engine-wide settings the whole application shares. Call once,
/// after GTK is initialised and before the first web view is built.
///
/// `engine_data_root` roots the one `WebView2` environment every account's
/// view shares — `main.rs` resolves it through `idle-manager-store`'s
/// `engine_data_root`, since only the composition root may know that
/// (architecture rule 3); `None` when it could not be resolved logs a
/// warning and falls back to the engine's own default location.
#[cfg(windows)]
pub fn configure_web_engine(engine_data_root: Option<std::path::PathBuf>) {
    web_engine::EngineShared::configure(engine_data_root);
}

/// Asks the `WebView2` loader which runtime version is installed, without
/// building a view. `main.rs` calls this before opening any store (`FR.1.10`)
/// and shows [`show_missing_engine`] on failure.
///
/// # Errors
///
/// One line naming why no runtime answered — most often that the `WebView2`
/// Runtime is not installed at all.
#[cfg(windows)]
pub fn runtime_version() -> Result<String, String> {
    wry::webview_version().map_err(|error| error.to_string())
}

/// Shows the lone start-up dialog for a missing `WebView2` runtime: a
/// heading, `reason` on its own line, the download address, and one `Quit`
/// button that ends the application. No account data is read or written
/// before or after it (`FR.1.10`).
///
/// Called instead of building the main window, before any store is opened —
/// there is no window yet for `docs/design.md` rule 9's message strip to sit
/// in, which is why this is a dialog of its own rather than that strip (the
/// new pattern roadmap item 12's front-end section flags).
#[cfg(windows)]
pub fn show_missing_engine(app: &gtk::Application, reason: &str) {
    let dialog = gtk::AlertDialog::builder()
        .message("Microsoft Edge WebView2 Runtime is required")
        .detail(format!(
            "{reason}\n\nInstall it from https://developer.microsoft.com/microsoft-edge/webview2/"
        ))
        .buttons(["Quit"])
        .build();

    let app = app.clone();
    dialog.choose(None::<&gtk::Window>, None::<&gio::Cancellable>, move |_| {
        app.quit();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_template_is_readable_from_the_registered_bundle() {
        register_resources().expect("register the compiled bundle");

        let data = gio::resources_lookup_data(
            "/org/idlemanager/IdleManager/ui/window.ui",
            gio::ResourceLookupFlags::NONE,
        )
        .expect("look up the window template by its resource path");

        assert!(!data.is_empty());
    }

    #[test]
    fn the_rename_dialog_template_is_readable_from_the_registered_bundle() {
        register_resources().expect("register the compiled bundle");

        let data = gio::resources_lookup_data(
            "/org/idlemanager/IdleManager/ui/rename-dialog.ui",
            gio::ResourceLookupFlags::NONE,
        )
        .expect("look up the rename dialog template by its resource path");

        assert!(!data.is_empty());
    }

    #[test]
    fn the_delete_account_dialog_template_is_readable_from_the_registered_bundle() {
        register_resources().expect("register the compiled bundle");

        let data = gio::resources_lookup_data(
            "/org/idlemanager/IdleManager/ui/delete-account-dialog.ui",
            gio::ResourceLookupFlags::NONE,
        )
        .expect("look up the delete-account dialog template by its resource path");

        assert!(!data.is_empty());
    }

    #[test]
    fn the_phone_dialog_template_is_readable_from_the_registered_bundle() {
        register_resources().expect("register the compiled bundle");

        let data = gio::resources_lookup_data(
            "/org/idlemanager/IdleManager/ui/phone-dialog.ui",
            gio::ResourceLookupFlags::NONE,
        )
        .expect("look up the phone dialog template by its resource path");

        assert!(!data.is_empty());
    }

    #[test]
    fn the_slot_placeholder_template_is_readable_from_the_registered_bundle() {
        register_resources().expect("register the compiled bundle");

        let data = gio::resources_lookup_data(
            "/org/idlemanager/IdleManager/ui/slot-placeholder.ui",
            gio::ResourceLookupFlags::NONE,
        )
        .expect("look up the slot placeholder template by its resource path");

        assert!(!data.is_empty());
    }

    #[test]
    fn the_memory_footer_template_is_readable_from_the_registered_bundle() {
        register_resources().expect("register the compiled bundle");

        let data = gio::resources_lookup_data(
            "/org/idlemanager/IdleManager/ui/memory-footer.ui",
            gio::ResourceLookupFlags::NONE,
        )
        .expect("look up the memory footer template by its resource path");

        assert!(!data.is_empty());
    }

    #[test]
    fn the_message_strip_template_is_readable_from_the_registered_bundle() {
        register_resources().expect("register the compiled bundle");

        let data = gio::resources_lookup_data(
            "/org/idlemanager/IdleManager/ui/message-strip.ui",
            gio::ResourceLookupFlags::NONE,
        )
        .expect("look up the message strip template by its resource path");

        assert!(!data.is_empty());
    }

    #[test]
    fn the_session_grid_stylesheet_is_readable_from_the_registered_bundle() {
        register_resources().expect("register the compiled bundle");

        let data = gio::resources_lookup_data(
            "/org/idlemanager/IdleManager/css/session-grid.css",
            gio::ResourceLookupFlags::NONE,
        )
        .expect("look up the session-grid stylesheet by its resource path");
        let css = std::str::from_utf8(&data).expect("the stylesheet is UTF-8");

        assert!(
            css.contains(".zoom-readout"),
            "session-grid.css must style the transient .zoom-readout figure"
        );
    }

    #[test]
    fn the_sidebar_stylesheet_carries_a_distinct_queued_dot_class() {
        register_resources().expect("register the compiled bundle");

        let data = gio::resources_lookup_data(
            "/org/idlemanager/IdleManager/css/sidebar.css",
            gio::ResourceLookupFlags::NONE,
        )
        .expect("look up the sidebar stylesheet by its resource path");
        let css = std::str::from_utf8(&data).expect("the stylesheet is UTF-8");

        assert!(
            css.contains(".status-queued") && css.contains("#dc8add"),
            "sidebar.css must style a status-queued dot in a colour that is \
             neither the starting #62a0ea nor the parked #9a9996"
        );
    }
}
