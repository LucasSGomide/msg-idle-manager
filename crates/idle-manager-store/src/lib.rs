//! Persistence: XDG locations, the on-disk preset catalogue, and the session
//! file that survives a restart.
//!
//! The record types here are the file format, deliberately separate from the
//! domain types they map to. That separation is what lets the domain be
//! refactored without rewriting files a user already has on disk.
