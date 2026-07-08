//! Per-monitor DPI scale control.
//!
//! Reads current scale via documented `GetDpiForMonitor`; writes via the
//! undocumented `DisplayConfigSetDeviceInfo(type=-4)` protocol used by the
//! Windows Settings app. Every write is verified by polling the effective DPI
//! (canary) — if the protocol ever changes, we fail loudly instead of drifting.
//!
//! Empirical notes (Win11 26200): the -3 GET packet must be exactly 32 bytes;
//! its `cur_rel` field is garbage when no explicit override exists — only
//! min/max are trustworthy. See docs/research/phase0-findings.md.

use crate::display::{
    self, DeviceInfoHeader, DisplayConfigGetDeviceInfo, DisplayConfigSetDeviceInfo,
};
use std::ffi::c_void;

/// The fixed DPI ladder used by the Windows Settings app.
pub const DPI_STEPS: [u32; 12] = [100, 125, 150, 175, 200, 225, 250, 300, 350, 400, 450, 500];

const DPI_GET: i32 = -3;
const DPI_SET: i32 = -4;

#[repr(C)]
struct DpiGetPacket {
    header: DeviceInfoHeader,
    min_rel: i32,
    cur_rel: i32, // untrustworthy — see module docs
    max_rel: i32,
}

#[repr(C)]
struct DpiSetPacket {
    header: DeviceInfoHeader,
    rel: i32,
}

struct ScaleRange {
    recommended_idx: i32,
    min_rel: i32,
    max_rel: i32,
}

fn probe_range() -> Result<ScaleRange, String> {
    let ids = display::primary_path()?;
    let mut pkt = DpiGetPacket {
        header: DeviceInfoHeader {
            info_type: DPI_GET,
            size: std::mem::size_of::<DpiGetPacket>() as u32,
            adapter_id: ids.adapter,
            id: ids.source_id,
        },
        min_rel: 0,
        cur_rel: 0,
        max_rel: 0,
    };
    let rc = unsafe { DisplayConfigGetDeviceInfo(&mut pkt as *mut _ as *mut c_void) };
    if rc != 0 {
        return Err(format!("DisplayConfigGetDeviceInfo(DPI) -> {rc}"));
    }
    Ok(ScaleRange {
        recommended_idx: -pkt.min_rel,
        min_rel: pkt.min_rel,
        max_rel: pkt.max_rel,
    })
}

fn step_at(range: &ScaleRange, rel: i32) -> u32 {
    let idx =
        (range.recommended_idx as i64 + rel as i64).clamp(0, DPI_STEPS.len() as i64 - 1) as usize;
    DPI_STEPS[idx]
}

/// (min%, max%) supported scale for the primary display.
pub fn scale_range() -> Result<(u32, u32), String> {
    let range = probe_range()?;
    Ok((
        step_at(&range, range.min_rel),
        step_at(&range, range.max_rel),
    ))
}

pub use crate::display::current_scale;

/// Set the primary display scale, verified by canary read-back.
pub fn set_scale(percent: u32) -> Result<(), String> {
    if current_scale()? == percent {
        return Ok(());
    }
    let target_idx = DPI_STEPS
        .iter()
        .position(|&s| s == percent)
        .ok_or_else(|| format!("unsupported scale {percent}% (valid: {DPI_STEPS:?})"))?
        as i32;

    let range = probe_range()?;
    let rel = target_idx - range.recommended_idx;
    if rel < range.min_rel || rel > range.max_rel {
        return Err(format!(
            "scale {percent}% is outside this display's supported range ({}%..{}%)",
            step_at(&range, range.min_rel),
            step_at(&range, range.max_rel)
        ));
    }

    let ids = display::primary_path()?;
    let mut pkt = DpiSetPacket {
        header: DeviceInfoHeader {
            info_type: DPI_SET,
            size: std::mem::size_of::<DpiSetPacket>() as u32,
            adapter_id: ids.adapter,
            id: ids.source_id,
        },
        rel,
    };
    let rc = unsafe { DisplayConfigSetDeviceInfo(&mut pkt as *mut _ as *mut c_void) };
    if rc != 0 {
        return Err(format!("DisplayConfigSetDeviceInfo(DPI) -> {rc}"));
    }

    for _ in 0..20 {
        if current_scale()? == percent {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Err(format!(
        "DPI write did not stick (wanted {percent}%, still {}%)",
        current_scale()?
    ))
}
