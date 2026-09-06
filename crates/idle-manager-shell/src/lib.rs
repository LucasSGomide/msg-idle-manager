//! The GTK 4 and `WebKitGTK` adapter: windows, the session grid, the sidebar, and
//! the web views that host each game.
//!
//! Everything that touches a widget lives here and runs on the GTK main
//! context. The shell reads domain state and emits intents back to it; it never
//! decides what a session's state should become.
