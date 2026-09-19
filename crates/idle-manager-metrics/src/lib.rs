//! Memory accounting: proportional set size sampled from `/proc`, per process
//! and aggregated.
//!
//! Proportional set size rather than resident set size, because the shell and
//! every web process map the same `WebKit` libraries and RSS would count those
//! pages once per process.

#[cfg(target_os = "linux")]
mod proc_pss;
mod process_tree;
mod tree;

#[cfg(target_os = "linux")]
pub use proc_pss::{ProcPssError, ProcPssProbe, ProcessKind, ProcessMemory, ProcessTreeReading};
#[cfg(windows)]
pub use process_tree::ProcessTreeProbe;
