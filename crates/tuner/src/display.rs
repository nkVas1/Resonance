//! Display topology, mode enumeration and switching. Documented Win32 only.

use resonance_core::Mode;
use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct DevModeW {
    dm_device_name: [u16; 32],
    dm_spec_version: u16,
    dm_driver_version: u16,
    dm_size: u16,
    dm_driver_extra: u16,
    dm_fields: u32,
    union1: [u8; 16],
    dm_color: i16,
    dm_duplex: i16,
    dm_y_resolution: i16,
    dm_tt_option: i16,
    dm_collate: i16,
    dm_form_name: [u16; 32],
    dm_log_pixels: u16,
    dm_bits_per_pel: u32,
    dm_pels_width: u32,
    dm_pels_height: u32,
    dm_display_flags: u32,
    dm_display_frequency: u32,
    dm_icm_method: u32,
    dm_icm_intent: u32,
    dm_media_type: u32,
    dm_dither_type: u32,
    dm_reserved1: u32,
    dm_reserved2: u32,
    dm_panning_width: u32,
    dm_panning_height: u32,
}

const _: () = assert!(std::mem::size_of::<DevModeW>() == 220);

const ENUM_CURRENT_SETTINGS: u32 = 0xFFFFFFFF;
const DISP_CHANGE_SUCCESSFUL: i32 = 0;

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Luid {
    pub low: u32,
    pub high: i32,
}

#[repr(C)]
struct PathSourceInfo {
    adapter_id: Luid,
    id: u32,
    mode_info_idx: u32,
    status_flags: u32,
}

#[repr(C)]
struct PathTargetInfo {
    adapter_id: Luid,
    id: u32,
    mode_info_idx: u32,
    output_technology: u32,
    rotation: u32,
    scaling: u32,
    refresh_rate_num: u32,
    refresh_rate_den: u32,
    scanline_ordering: u32,
    target_available: i32,
    status_flags: u32,
}

#[repr(C)]
struct PathInfo {
    source: PathSourceInfo,
    target: PathTargetInfo,
    flags: u32,
}

#[repr(C, align(8))]
struct ModeInfo {
    info_type: u32,
    id: u32,
    adapter_id: Luid,
    payload: [u8; 48],
}

const _: () = assert!(std::mem::size_of::<PathInfo>() == 72);
const _: () = assert!(std::mem::size_of::<ModeInfo>() == 64);

#[repr(C)]
pub(crate) struct DeviceInfoHeader {
    pub info_type: i32,
    pub size: u32,
    pub adapter_id: Luid,
    pub id: u32,
}

/// DISPLAYCONFIG_TARGET_PREFERRED_MODE (documented, type = 8).
#[repr(C, align(8))]
struct TargetPreferredMode {
    header: DeviceInfoHeader,
    width: u32,
    height: u32,
    _pad: u32, // targetMode (contains u64) aligns to 8
    target_mode: [u8; 48],
}

const _: () = assert!(std::mem::size_of::<TargetPreferredMode>() == 80);

const QDC_ONLY_ACTIVE_PATHS: u32 = 2;
const GET_TARGET_PREFERRED_MODE: i32 = 3; // DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_PREFERRED_MODE

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

#[repr(C)]
struct DisplayDeviceW {
    cb: u32,
    device_name: [u16; 32],
    device_string: [u16; 128],
    state_flags: u32,
    device_id: [u16; 128],
    device_key: [u16; 128],
}

const _: () = assert!(std::mem::size_of::<DisplayDeviceW>() == 840);

#[link(name = "shcore")]
extern "system" {
    fn GetDpiForMonitor(
        hmonitor: *mut c_void,
        dpi_type: u32,
        dpi_x: *mut u32,
        dpi_y: *mut u32,
    ) -> i32;
}

#[link(name = "user32")]
extern "system" {
    fn SetProcessDpiAwarenessContext(context: isize) -> i32;
    fn MonitorFromPoint(pt: Point, flags: u32) -> *mut c_void;
    fn EnumDisplayDevicesW(
        device: *const u16,
        dev_num: u32,
        display_device: *mut DisplayDeviceW,
        flags: u32,
    ) -> i32;
    fn EnumDisplaySettingsExW(
        device: *const u16,
        mode_num: u32,
        devmode: *mut DevModeW,
        flags: u32,
    ) -> i32;
    fn ChangeDisplaySettingsExW(
        device: *const u16,
        devmode: *mut DevModeW,
        hwnd: *mut c_void,
        flags: u32,
        param: *mut c_void,
    ) -> i32;
    fn GetDisplayConfigBufferSizes(flags: u32, num_paths: *mut u32, num_modes: *mut u32) -> i32;
    fn QueryDisplayConfig(
        flags: u32,
        num_paths: *mut u32,
        paths: *mut PathInfo,
        num_modes: *mut u32,
        modes: *mut ModeInfo,
        topology: *mut u32,
    ) -> i32;
    pub(crate) fn DisplayConfigGetDeviceInfo(packet: *mut c_void) -> i32;
    pub(crate) fn DisplayConfigSetDeviceInfo(packet: *mut c_void) -> i32;
}

/// Opt into per-monitor-v2 DPI awareness (required for truthful GetDpiForMonitor).
/// Call once at process start.
pub fn enable_dpi_awareness() {
    const PER_MONITOR_AWARE_V2: isize = -4;
    unsafe { SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2) };
}

/// Adapter LUID + source/target ids of the primary active display path.
#[derive(Clone, Copy, Debug)]
pub struct PathIds {
    pub adapter: Luid,
    pub source_id: u32,
    pub target_id: u32,
}

pub fn primary_path() -> Result<PathIds, String> {
    let mut n_paths = 0u32;
    let mut n_modes = 0u32;
    if unsafe { GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut n_paths, &mut n_modes) }
        != 0
    {
        return Err("GetDisplayConfigBufferSizes failed".into());
    }
    let mut paths: Vec<PathInfo> = Vec::with_capacity(n_paths as usize);
    let mut modes: Vec<ModeInfo> = Vec::with_capacity(n_modes as usize);
    let rc = unsafe {
        QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut n_paths,
            paths.as_mut_ptr(),
            &mut n_modes,
            modes.as_mut_ptr(),
            std::ptr::null_mut(),
        )
    };
    if rc != 0 {
        return Err(format!("QueryDisplayConfig -> {rc}"));
    }
    unsafe { paths.set_len(n_paths as usize) };
    let p = paths.first().ok_or("no active display paths")?;
    Ok(PathIds {
        adapter: p.source.adapter_id,
        source_id: p.source.id,
        target_id: p.target.id,
    })
}

/// The panel's native resolution (EDID preferred mode) — documented API.
pub fn native_resolution() -> Result<(u32, u32), String> {
    let ids = primary_path()?;
    let mut pkt: TargetPreferredMode = unsafe { std::mem::zeroed() };
    pkt.header = DeviceInfoHeader {
        info_type: GET_TARGET_PREFERRED_MODE,
        size: std::mem::size_of::<TargetPreferredMode>() as u32,
        adapter_id: ids.adapter,
        id: ids.target_id,
    };
    let rc = unsafe { DisplayConfigGetDeviceInfo(&mut pkt as *mut _ as *mut c_void) };
    if rc != 0 {
        return Err(format!(
            "DisplayConfigGetDeviceInfo(preferred mode) -> {rc}"
        ));
    }
    Ok((pkt.width, pkt.height))
}

/// Effective DPI scale % of the primary monitor (documented GetDpiForMonitor).
pub fn current_scale() -> Result<u32, String> {
    const MDT_EFFECTIVE_DPI: u32 = 0;
    const MONITOR_DEFAULTTOPRIMARY: u32 = 1;
    let hmon = unsafe { MonitorFromPoint(Point { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
    let (mut dx, mut dy) = (0u32, 0u32);
    let hr = unsafe { GetDpiForMonitor(hmon, MDT_EFFECTIVE_DPI, &mut dx, &mut dy) };
    if hr != 0 {
        return Err(format!("GetDpiForMonitor -> 0x{hr:08X}"));
    }
    Ok((dy * 100 + 48) / 96)
}

fn empty_devmode() -> DevModeW {
    let mut dm: DevModeW = unsafe { std::mem::zeroed() };
    dm.dm_size = std::mem::size_of::<DevModeW>() as u16;
    dm
}

pub fn current_mode() -> Result<Mode, String> {
    let mut dm = empty_devmode();
    if unsafe { EnumDisplaySettingsExW(std::ptr::null(), ENUM_CURRENT_SETTINGS, &mut dm, 0) } == 0 {
        return Err("EnumDisplaySettingsExW(current) failed".into());
    }
    Ok(Mode {
        width: dm.dm_pels_width,
        height: dm.dm_pels_height,
        hz: dm.dm_display_frequency,
    })
}

/// All 32-bpp modes the driver offers, deduplicated and sorted.
pub fn list_modes() -> Vec<Mode> {
    let mut modes = Vec::new();
    let mut i = 0u32;
    loop {
        let mut dm = empty_devmode();
        if unsafe { EnumDisplaySettingsExW(std::ptr::null(), i, &mut dm, 0) } == 0 {
            break;
        }
        if dm.dm_bits_per_pel == 32 {
            modes.push(Mode {
                width: dm.dm_pels_width,
                height: dm.dm_pels_height,
                hz: dm.dm_display_frequency,
            });
        }
        i += 1;
    }
    modes.sort_by_key(|m| (m.width, m.height, m.hz));
    modes.dedup();
    modes
}

/// Best (highest-refresh) mode matching a resolution, optionally pinning refresh.
pub fn resolve_mode(width: u32, height: u32, refresh: Option<u32>) -> Result<Mode, String> {
    list_modes()
        .into_iter()
        .filter(|m| m.width == width && m.height == height)
        .filter(|m| refresh.is_none_or(|hz| m.hz == hz))
        .max_by_key(|m| m.hz)
        .ok_or_else(|| match refresh {
            Some(hz) => format!("mode {width}x{height}@{hz}Hz is not offered by the driver"),
            None => format!("mode {width}x{height} is not offered by the driver"),
        })
}

/// Dynamic (non-persisted) mode switch: the registry is untouched, so a reboot
/// always returns to the OS-configured mode — a deliberate safety property.
pub fn switch_mode(mode: Mode) -> Result<(), String> {
    let mut dm = empty_devmode();
    dm.dm_pels_width = mode.width;
    dm.dm_pels_height = mode.height;
    dm.dm_display_frequency = mode.hz;
    dm.dm_bits_per_pel = 32;
    const DM_BITSPERPEL: u32 = 0x00040000;
    const DM_PELSWIDTH: u32 = 0x00080000;
    const DM_PELSHEIGHT: u32 = 0x00100000;
    const DM_DISPLAYFREQUENCY: u32 = 0x00400000;
    dm.dm_fields = DM_BITSPERPEL | DM_PELSWIDTH | DM_PELSHEIGHT | DM_DISPLAYFREQUENCY;

    let rc = unsafe {
        ChangeDisplaySettingsExW(
            std::ptr::null(),
            &mut dm,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
        )
    };
    if rc != DISP_CHANGE_SUCCESSFUL {
        return Err(format!("ChangeDisplaySettingsExW({mode}) -> {rc}"));
    }
    Ok(())
}

fn utf16_str(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}

/// (adapter name, monitor name) of the primary display, e.g.
/// ("NVIDIA GeForce RTX 3060 Ti", "ASUS VG259").
pub fn device_names() -> Result<(String, String), String> {
    let mut adapter: DisplayDeviceW = unsafe { std::mem::zeroed() };
    adapter.cb = std::mem::size_of::<DisplayDeviceW>() as u32;
    if unsafe { EnumDisplayDevicesW(std::ptr::null(), 0, &mut adapter, 0) } == 0 {
        return Err("EnumDisplayDevicesW(adapter) failed".into());
    }
    let adapter_name = utf16_str(&adapter.device_string);
    let device_name = adapter.device_name;

    let mut monitor: DisplayDeviceW = unsafe { std::mem::zeroed() };
    monitor.cb = std::mem::size_of::<DisplayDeviceW>() as u32;
    let monitor_name =
        if unsafe { EnumDisplayDevicesW(device_name.as_ptr(), 0, &mut monitor, 0) } != 0 {
            utf16_str(&monitor.device_string)
        } else {
            String::from("(unknown monitor)")
        };
    Ok((adapter_name, monitor_name))
}
