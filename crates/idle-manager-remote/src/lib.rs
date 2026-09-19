//! The phone server: the second adapter that drives the application from
//! outside, beside the shell (roadmap item 13).
//!
//! It turns network messages into [`idle_manager_core::RemoteIntent`]s and
//! [`idle_manager_core::RemoteState`] into network messages, serving three
//! routes over plain `std::net` sockets on its own threads — no async runtime
//! and no GTK. It depends on `idle-manager-core` alone and never on the shell,
//! the store or metrics (architecture rule 2); `make arch-check` fails the
//! build if that changes.
