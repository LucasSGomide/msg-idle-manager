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
mod memory_footer;
mod message_strip;
mod save_on_change;
mod session_grid;
mod session_sidebar;
mod slot_placeholder;
mod start_queue;
mod web_view;
mod window;

use std::cell::OnceCell;

use gtk::gio;
use gtk::glib;
use gtk4 as gtk;

pub use window::{Window, WindowPorts};

thread_local! {
    /// The one web context every account's view is built against, carrying the
    /// memory-pressure settings (`FR.19.4`). A `thread_local` rather than a
    /// `static OnceLock` because `WebContext` is a glib object and not `Sync`
    /// (architecture rule 10); it lives on the GTK main context, set once by
    /// [`configure_web_engine`] before the first view.
    static SHARED_WEB_CONTEXT: OnceCell<webkit6::WebContext> = const { OnceCell::new() };
}

/// The shared web context [`configure_web_engine`] built, or `None` if it has
/// not run yet. `web_view.rs` builds every account's view against this so a
/// loaded page runs under the memory-pressure settings rather than a default
/// context created behind the application's back (`FR.19.4`).
pub(crate) fn shared_web_context() -> Option<webkit6::WebContext> {
    SHARED_WEB_CONTEXT.with(|cell| cell.get().cloned())
}

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

/// The memory limit the engine watches its rendering processes against, in
/// mebibytes (`FR.19.4`, code standards rule 5).
// `0` is not "no limit": WebKitGTK's `set_memory_limit` guards its argument
// with `g_return_if_fail(memoryLimit)` — passing `0` logs a GLib critical and
// leaves the engine's own default in place, documented as "the system's RAM
// size with a maximum of 3GB" (confirmed against WebKit's
// `WebKitMemoryPressureSettings.cpp`). That default was silently governing
// every rendering process instead of a number chosen for this application.
// docs/memory-budget.md found the working set does not settle (three samples
// on 2026-09-11 with 4 accounts live climbed from 57.8 MiB to 145.6 MiB
// shell-side and 2.17 GiB to 2.92 GiB descendant-side, with no sign of
// flattening), so there is no settled per-game figure to size this against.
// 1024 MiB is chosen with headroom above the one figure that IS on record —
// a single account's rendering process measured stable-ish at 751 MiB
// (docs/roadmap/05-memory-accounting/README.md, "Where the memory goes") —
// so a healthy game does not sit in cache-shedding territory continuously,
// while the limit still engages well short of exhausting the machine. This
// caps how bad the unbounded growth gets before the engine starts shedding
// caches; it does not fix the growth itself (docs/memory-budget.md).
const WEB_PROCESS_MEMORY_LIMIT_MIB: u32 = 1024;
/// The fraction of the limit at which the engine starts shedding caches it
/// would otherwise keep (`FR.19.4`). The type's own default, kept: nothing in
/// docs/memory-budget.md's non-settling curve argues for moving it, and a
/// third of the 1024 MiB limit above (~338 MiB) sits comfortably above the
/// allocator heap's own floor (138,616 KiB, unmoving across the twenty-minute
/// sample in the roadmap item's README) so ordinary operation does not idle
/// in cache-shedding territory.
const CONSERVATIVE_PRESSURE_THRESHOLD: f64 = 0.33;
/// The fraction of the limit at which the engine collects harder and drops
/// more (`FR.19.4`). The type's own default, kept for the same reason as
/// `CONSERVATIVE_PRESSURE_THRESHOLD` above.
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

/// Applies the engine-wide settings the whole application shares. Call once,
/// after GTK is initialised and before the first web view is built.
///
/// Builds the one [`webkit6::WebContext`] every account's view is constructed
/// against and sets on it:
///
/// - [`webkit6::CacheModel::DocumentViewer`], the lowest of the three cache
///   models, so a parked account's memory returns to the operating system
///   rather than staying with the engine (`FR.5.3`);
/// - the [`webkit6::MemoryPressureSettings`], a construct-only property, so the
///   engine sheds caches and collects harder as a rendering process approaches
///   its limit — which is why the context can no longer be
///   `WebContext::default()` and has to be built (`FR.19.4`).
///
/// The same settings are handed to the one networking process through the
/// static [`webkit6::NetworkSession::set_memory_pressure_settings`] (`FR.1.3`).
pub fn configure_web_engine() {
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
        if cell.set(context).is_err() {
            tracing::warn!("configure_web_engine ran twice; keeping the first web context");
        }
    });

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
