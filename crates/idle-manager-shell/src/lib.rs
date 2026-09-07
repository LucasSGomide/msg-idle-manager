//! The GTK 4 and `WebKitGTK` adapter: windows, the session grid, the sidebar, and
//! the web views that host each game.
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

mod add_game_dialog;
mod message_strip;
mod save_on_change;
mod session_grid;
mod session_sidebar;
mod slot_placeholder;
mod start_queue;
mod web_view;
mod window;

use gtk::gio;
use gtk::glib;
use gtk4 as gtk;

pub use window::Window;

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
/// Sets [`webkit6::CacheModel::DocumentViewer`], the lowest of the three cache
/// models and the one `WebKit` associates with keeping no cache of terminated
/// rendering processes, so a parked account's memory returns to the operating
/// system rather than staying with the engine (`FR.5.3`). It is a global on the
/// shared web context, never a per-session setting.
pub fn configure_web_engine() {
    let Some(context) = webkit6::WebContext::default() else {
        tracing::warn!("no default WebKitWebContext; cache model left at its default");
        return;
    };
    context.set_cache_model(webkit6::CacheModel::DocumentViewer);

    web_view::log_engine_features();
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
