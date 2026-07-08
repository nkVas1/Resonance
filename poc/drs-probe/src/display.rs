//! Display mode enumeration/switching (documented Win32) and per-monitor DPI
//! scaling (undocumented DisplayConfig{Get,Set}DeviceInfo types -3/-4, as used
//! by the Windows Settings app; verified by lihas/windows-DPI-scaling-sample).

use std::ffi::c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DevModeW {
    pub dm_device_name: [u16; 32],
    pub dm_spec_version: u16,
    pub dm_driver_version: u16,
    pub dm_size: u16,
    pub dm_driver_extra: u16,
    pub dm_fields: u32,
    pub union1: [u8; 16], // printer/display union (position, orientation, fixed output)
    pub dm_color: i16,
    pub dm_duplex: i16,
    pub dm_y_resolution: i16,
    pub dm_tt_option: i16,
    pub dm_collate: i16,
    pub dm_form_name: [u16; 32],
    pub dm_log_pixels: u16,
    pub dm_bits_per_pel: u32,
    pub dm_pels_width: u32,
    pub dm_pels_height: u32,
    pub dm_display_flags: u32,
    pub dm_display_frequency: u32,
    pub dm_icm_method: u32,
    pub dm_icm_intent: u32,
    pub dm_media_type: u32,
    pub dm_dither_type: u32,
    pub dm_reserved1: u32,
    pub dm_reserved2: u32,
    pub dm_panning_width: u32,
    pub dm_panning_height: u32,
}

const _: () = assert!(std::mem::size_of::<DevModeW>() == 220);

const ENUM_CURRENT_SETTINGS: u32 = 0xFFFFFFFF; // (DWORD)-1
const DISP_CHANGE_SUCCESSFUL: i32 = 0;

#[repr(C)]
#[derive(Clone, Copy, Default)]
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
struct DeviceInfoHeader {
    info_type: i32,
    size: u32,
    adapter_id: Luid,
    id: u32,
}

#[repr(C)]
struct DpiGetPacket {
    header: DeviceInfoHeader,
    min_rel: i32,
    cur_rel: i32,
    max_rel: i32,
}

#[repr(C)]
struct DpiSetPacket {
    header: DeviceInfoHeader,
    rel: i32,
}

const DPI_GET: i32 = -3;
const DPI_SET: i32 = -4;
const QDC_ONLY_ACTIVE_PATHS: u32 = 2;

/// The fixed DPI ladder used by the Windows Settings app.
pub const DPI_STEPS: [u32; 12] = [100, 125, 150, 175, 200, 225, 250, 300, 350, 400, 450, 500];

#[repr(C)]
struct Point {
    x: i32,
    y: i32,
}

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
    fn DisplayConfigGetDeviceInfo(packet: *mut c_void) -> i32;
    fn DisplayConfigSetDeviceInfo(packet: *mut c_void) -> i32;
}

fn empty_devmode() -> DevModeW {
    let mut dm: DevModeW = unsafe { std::mem::zeroed() };
    dm.dm_size = std::mem::size_of::<DevModeW>() as u16;
    dm
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub hz: u32,
    pub bpp: u32,
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
        bpp: dm.dm_bits_per_pel,
    })
}

pub fn list_modes() -> Vec<Mode> {
    let mut modes = Vec::new();
    let mut i = 0u32;
    loop {
        let mut dm = empty_devmode();
        if unsafe { EnumDisplaySettingsExW(std::ptr::null(), i, &mut dm, 0) } == 0 {
            break;
        }
        modes.push(Mode {
            width: dm.dm_pels_width,
            height: dm.dm_pels_height,
            hz: dm.dm_display_frequency,
            bpp: dm.dm_bits_per_pel,
        });
        i += 1;
    }
    modes.sort_by_key(|m| (m.width, m.height, m.hz));
    modes.dedup();
    modes
}

/// Dynamic (non-persisted) mode switch: registry untouched, reverts on reboot.
pub fn switch_mode(width: u32, height: u32) -> Result<Mode, String> {
    let best = list_modes()
        .into_iter()
        .filter(|m| m.width == width && m.height == height && m.bpp == 32)
        .max_by_key(|m| m.hz)
        .ok_or(format!("mode {width}x{height} not offered by the driver"))?;

    let mut dm = empty_devmode();
    dm.dm_pels_width = best.width;
    dm.dm_pels_height = best.height;
    dm.dm_display_frequency = best.hz;
    dm.dm_bits_per_pel = best.bpp;
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
        return Err(format!("ChangeDisplaySettingsExW -> {rc}"));
    }
    Ok(best)
}

/// (adapter LUID, source id) of the first active display path.
fn primary_source() -> Result<(Luid, u32), String> {
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
    Ok((p.source.adapter_id, p.source.id))
}

/// Probe the DPI GET packet with an arbitrary payload size and dump every dword —
/// for reverse-engineering the exact layout on this Windows build.
pub fn dpi_get_raw(payload_dwords: usize) -> Result<Vec<i32>, String> {
    #[repr(C)]
    struct Probe {
        header: DeviceInfoHeader,
        vals: [i32; 16],
    }
    let n = payload_dwords.min(16);
    let (adapter, id) = primary_source()?;
    let mut pkt = Probe {
        header: DeviceInfoHeader {
            info_type: DPI_GET,
            size: (std::mem::size_of::<DeviceInfoHeader>() + n * 4) as u32,
            adapter_id: adapter,
            id,
        },
        vals: [0x5A5A5A5A_u32 as i32; 16], // sentinel to spot untouched fields
    };
    let rc = unsafe { DisplayConfigGetDeviceInfo(&mut pkt as *mut _ as *mut c_void) };
    if rc != 0 {
        return Err(format!(
            "DisplayConfigGetDeviceInfo(DPI, size={}) -> {rc}",
            pkt.header.size
        ));
    }
    Ok(pkt.vals[..n].to_vec())
}

/// Opt this process into per-monitor DPI awareness so GetDpiForMonitor
/// reports real values instead of virtualized 96. Call once at startup.
pub fn enable_dpi_awareness() {
    const PER_MONITOR_AWARE_V2: isize = -4;
    unsafe { SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2) };
}

/// Current effective scale % of the primary monitor via documented GetDpiForMonitor.
/// (The undocumented -3 packet's cur_rel field is garbage on Win11 26200 when no
/// explicit override is set — verified empirically; min/max from it are fine.)
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

/// (min%, current%, max%) scale for the primary display.
pub fn dpi_get() -> Result<(u32, u32, u32), String> {
    let (adapter, id) = primary_source()?;
    let mut pkt = DpiGetPacket {
        header: DeviceInfoHeader {
            info_type: DPI_GET,
            size: std::mem::size_of::<DpiGetPacket>() as u32,
            adapter_id: adapter,
            id,
        },
        min_rel: 0,
        cur_rel: 0,
        max_rel: 0,
    };
    let rc = unsafe { DisplayConfigGetDeviceInfo(&mut pkt as *mut _ as *mut c_void) };
    if rc != 0 {
        return Err(format!("DisplayConfigGetDeviceInfo(DPI) -> {rc}"));
    }
    let recommended = (-pkt.min_rel) as usize; // index of 100%-relative recommended scale
    let step = |rel: i32| -> u32 {
        let idx = (recommended as i64 + rel as i64).clamp(0, DPI_STEPS.len() as i64 - 1) as usize;
        DPI_STEPS[idx]
    };
    Ok((step(pkt.min_rel), current_scale()?, step(pkt.max_rel)))
}

pub fn dpi_set(percent: u32) -> Result<(), String> {
    let target_idx = DPI_STEPS.iter().position(|&s| s == percent).ok_or(format!(
        "unsupported scale {percent}% (valid: {DPI_STEPS:?})"
    ))? as i32;

    let (adapter, id) = primary_source()?;
    let mut probe = DpiGetPacket {
        header: DeviceInfoHeader {
            info_type: DPI_GET,
            size: std::mem::size_of::<DpiGetPacket>() as u32,
            adapter_id: adapter,
            id,
        },
        min_rel: 0,
        cur_rel: 0,
        max_rel: 0,
    };
    if unsafe { DisplayConfigGetDeviceInfo(&mut probe as *mut _ as *mut c_void) } != 0 {
        return Err("DPI probe failed".into());
    }
    let recommended = -probe.min_rel;
    let rel = (target_idx - recommended).clamp(probe.min_rel, probe.max_rel);

    let mut pkt = DpiSetPacket {
        header: DeviceInfoHeader {
            info_type: DPI_SET,
            size: std::mem::size_of::<DpiSetPacket>() as u32,
            adapter_id: adapter,
            id,
        },
        rel,
    };
    let rc = unsafe { DisplayConfigSetDeviceInfo(&mut pkt as *mut _ as *mut c_void) };
    if rc != 0 {
        return Err(format!("DisplayConfigSetDeviceInfo(DPI) -> {rc}"));
    }
    // Canary: poll effective DPI until the write propagates (or time out).
    if rel == target_idx - recommended {
        for _ in 0..20 {
            if current_scale()? == percent {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        return Err(format!(
            "DPI write did not stick (wanted {percent}%, still {}%)",
            current_scale()?
        ));
    }
    Ok(())
}
