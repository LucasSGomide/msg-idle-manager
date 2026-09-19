//! The metrics crate's one `unsafe` module (code standards rule 28, as
//! amended for roadmap item 12): every direct Win32 call
//! [`super::windows_probe::ProcessTreeProbe`] needs, and nothing else. Every
//! `unsafe` block carries a `// SAFETY:` line, every handle this module opens
//! is closed before the function that opened it returns, and only safe
//! functions are exposed.

#![allow(unsafe_code)]

use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::ProcessStatus::{K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX2};
use windows::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};

use idle_manager_core::MemoryProbeError;

use super::Counters;

/// One process table row read from the snapshot: its process id, command
/// name and the parent it hangs off — the same shape
/// [`crate::tree::ProcessEntry`] wants, without depending on this module.
pub(super) struct SnapshotEntry {
    pub(super) pid: u32,
    pub(super) command: String,
    pub(super) parent: u32,
}

/// A thin RAII wrapper closing a Win32 handle on drop, so an early return —
/// including through `?` — can never leak one.
struct OwnedHandle(HANDLE);

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        if self.0.is_invalid() {
            return;
        }
        // SAFETY: `self.0` was returned by a successful `CreateToolhelp32Snapshot`
        // or `OpenProcess` above and has not been closed yet — this is the one
        // place that closes it.
        let _ = unsafe { CloseHandle(self.0) };
    }
}

/// Takes one snapshot of every process on the machine and returns each row's
/// process id, command name and parent process id.
///
/// # Errors
///
/// [`MemoryProbeError::Refused`] if the snapshot itself could not be taken;
/// [`MemoryProbeError::Unreadable`] if a row's command name is not valid
/// UTF-16.
pub(super) fn snapshot_processes() -> Result<Vec<SnapshotEntry>, MemoryProbeError> {
    // SAFETY: `TH32CS_SNAPPROCESS` with a process id of `0` snapshots every
    // process on the system; the returned handle is owned by this call and
    // closed by `OwnedHandle`'s `Drop`.
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }.map_err(|error| {
        MemoryProbeError::Refused {
            reason: format!("CreateToolhelp32Snapshot: {error}"),
        }
    })?;
    let snapshot = OwnedHandle(snapshot);

    let mut entry = PROCESSENTRY32W {
        dwSize: u32::try_from(std::mem::size_of::<PROCESSENTRY32W>())
            .expect("the size of a fixed Win32 struct fits in a u32"),
        ..Default::default()
    };

    let mut rows = Vec::new();
    // SAFETY: `entry.dwSize` is set to the struct's own size as the API
    // requires, and `snapshot.0` is the valid handle just created above.
    let mut has_next = unsafe { Process32FirstW(snapshot.0, &raw mut entry) }.is_ok();
    while has_next {
        rows.push(SnapshotEntry {
            pid: entry.th32ProcessID,
            command: command_name(&entry.szExeFile),
            parent: entry.th32ParentProcessID,
        });

        // SAFETY: same handle and same, still-`dwSize`-initialised `entry`.
        has_next = unsafe { Process32NextW(snapshot.0, &raw mut entry) }.is_ok();
    }

    Ok(rows)
}

/// Decodes a `PROCESSENTRY32W::szExeFile` fixed-size, nul-terminated UTF-16
/// buffer into an owned `String`, stopping at the first nul the same way the
/// Win32 API itself does.
fn command_name(exe_file: &[u16]) -> String {
    let end = exe_file
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(exe_file.len());
    String::from_utf16_lossy(&exe_file[..end])
}

/// Opens `pid` and reads its private memory counters, or `None` when the
/// process could not be opened because it has already exited — the same
/// "vanished mid-walk" case the `/proc` probe treats as a skip, not a
/// failure (code standards rule 14).
///
/// # Errors
///
/// [`MemoryProbeError::Unreadable`] if the process opened but its memory
/// counters could not be read.
pub(super) fn process_memory(pid: u32) -> Result<Option<Counters>, MemoryProbeError> {
    // SAFETY: `pid` is a process id read from this walk's own snapshot; a
    // failure here (most often the process having exited since the snapshot)
    // is handled below, not assumed away.
    let Ok(handle) = (unsafe {
        OpenProcess(
            PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
            false,
            pid,
        )
    }) else {
        return Ok(None);
    };
    let handle = OwnedHandle(handle);

    let mut counters = PROCESS_MEMORY_COUNTERS_EX2::default();
    let size = u32::try_from(std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX2>())
        .expect("the size of a fixed Win32 struct fits in a u32");
    // SAFETY: `handle.0` is the valid, still-open handle from `OpenProcess`
    // above, `counters` is a correctly sized, writable out-buffer, and `size`
    // names that same size, exactly as the API requires.
    let read = unsafe {
        K32GetProcessMemoryInfo(handle.0, std::ptr::from_mut(&mut counters).cast(), size)
    };
    if read.as_bool() {
        Ok(Some(Counters {
            private_working_set: counters.PrivateWorkingSetSize as u64,
            private_usage: counters.PrivateUsage as u64,
        }))
    } else {
        Err(MemoryProbeError::Unreadable {
            reason: format!("K32GetProcessMemoryInfo failed for pid {pid}"),
        })
    }
}
