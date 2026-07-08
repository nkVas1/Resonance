//! System-state sampling for automation triggers.
//!
//! Polling-based by design: the foreground app, power source and process list
//! are cheap to read once a second, and polling avoids the fragile message-loop
//! threading that `SetWinEventHook`/WMI would require inside a library. Mode
//! switches cost ~1 s anyway, so sub-second precision buys nothing.

use crate::{PowerSource, SystemState};
use std::ffi::c_void;

#[repr(C)]
struct SystemPowerStatus {
    ac_line_status: u8,
    battery_flag: u8,
    battery_life_percent: u8,
    system_status_flag: u8,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

#[repr(C)]
struct ProcessEntry32W {
    dw_size: u32,
    cnt_usage: u32,
    th32_process_id: u32,
    th32_default_heap_id: usize,
    th32_module_id: u32,
    cnt_threads: u32,
    th32_parent_process_id: u32,
    pc_pri_class_base: i32,
    dw_flags: u32,
    sz_exe_file: [u16; 260],
}

const TH32CS_SNAPPROCESS: u32 = 0x00000002;
const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const INVALID_HANDLE: *mut c_void = usize::MAX as *mut c_void;

#[link(name = "kernel32")]
extern "system" {
    fn GetSystemPowerStatus(status: *mut SystemPowerStatus) -> i32;
    fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> *mut c_void;
    fn Process32FirstW(snapshot: *mut c_void, entry: *mut ProcessEntry32W) -> i32;
    fn Process32NextW(snapshot: *mut c_void, entry: *mut ProcessEntry32W) -> i32;
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
    fn QueryFullProcessImageNameW(
        proc: *mut c_void,
        flags: u32,
        buf: *mut u16,
        size: *mut u32,
    ) -> i32;
    fn CloseHandle(h: *mut c_void) -> i32;
}

#[link(name = "user32")]
extern "system" {
    fn GetForegroundWindow() -> *mut c_void;
    fn GetWindowThreadProcessId(hwnd: *mut c_void, pid: *mut u32) -> u32;
}

fn file_name_lower(path: &[u16]) -> String {
    let s = String::from_utf16_lossy(path);
    s.rsplit(['\\', '/'])
        .next()
        .unwrap_or(&s)
        .to_ascii_lowercase()
}

/// Power source, or `None` if the status is unknown (desktops report 255).
pub fn power_source() -> Option<PowerSource> {
    let mut status: SystemPowerStatus = unsafe { std::mem::zeroed() };
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return None;
    }
    match status.ac_line_status {
        0 => Some(PowerSource::Battery),
        1 => Some(PowerSource::Ac),
        _ => None,
    }
}

/// Lower-cased image name of the process owning the foreground window.
pub fn foreground_process() -> Option<String> {
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        return None;
    }
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
    if pid == 0 {
        return None;
    }
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return None;
    }
    let mut buf = [0u16; 260];
    let mut size = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) };
    unsafe { CloseHandle(handle) };
    if ok == 0 {
        return None;
    }
    Some(file_name_lower(&buf[..size as usize]))
}

/// Lower-cased image names of all running processes.
pub fn running_processes() -> Vec<String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE || snapshot.is_null() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(256);
    let mut entry: ProcessEntry32W = unsafe { std::mem::zeroed() };
    entry.dw_size = std::mem::size_of::<ProcessEntry32W>() as u32;
    if unsafe { Process32FirstW(snapshot, &mut entry) } != 0 {
        loop {
            out.push(file_name_lower(&entry.sz_exe_file));
            if unsafe { Process32NextW(snapshot, &mut entry) } == 0 {
                break;
            }
        }
    }
    unsafe { CloseHandle(snapshot) };
    out
}

/// Sample the full observable state. `needs_running` gates the (heavier) process
/// enumeration — skip it when no rule uses a `running` trigger.
pub fn sample(needs_running: bool, pinned: Option<String>) -> SystemState {
    SystemState {
        foreground: foreground_process(),
        running: if needs_running {
            running_processes()
        } else {
            Vec::new()
        },
        power: power_source(),
        pinned_profile: pinned,
    }
}
