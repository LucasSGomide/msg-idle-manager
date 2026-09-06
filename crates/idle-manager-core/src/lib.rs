//! The domain: what a session is, how it moves between states, and the traits
//! the rest of the application must satisfy to serve it.
//!
//! This crate knows nothing about GTK, `WebKit`, the filesystem or `/proc`. It
//! depends on no other crate in the workspace, which is what lets its tests run
//! in milliseconds without a display server. `make arch-check` fails the build
//! if a UI, serialisation or I/O dependency ever reaches it.

mod layout;
mod ports;
mod session;

pub use layout::{Layout, Outcome, Placement, SlotId, arrange};
pub use ports::{ProfileDirectories, ProfileError, ProfileLocator};
pub use session::{Liveness, Session, SessionBook, SessionId, Visibility};
