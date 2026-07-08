//! Tuner — the Resonance display control plane.
//!
//! High-level operations over documented Win32 primitives: query state,
//! resolve profiles into concrete modes, apply and restore display states.
//! No driver mutation, no elevation. `unsafe` FFI is confined to `display`/`dpi`.

pub mod display;
pub mod doctor;
pub mod dpi;
pub mod vendor;

use resonance_core::{DisplayState, Mode, Profile};

/// Call once at process start (DPI awareness etc.).
pub fn init() {
    display::enable_dpi_awareness();
}

/// Current display state (mode + scale) of the primary display.
pub fn state() -> Result<DisplayState, String> {
    Ok(DisplayState {
        mode: display::current_mode()?,
        scale: display::current_scale()?,
    })
}

/// Resolve a profile into a concrete target state for this machine.
pub fn resolve(profile: &Profile) -> Result<DisplayState, String> {
    let (w, h) = match profile.resolution {
        Some(res) => res,
        None => display::native_resolution()?,
    };
    let mode = display::resolve_mode(w, h, profile.refresh)?;
    let scale = match profile.scale {
        Some(s) => s,
        None => display::current_scale()?,
    };
    Ok(DisplayState { mode, scale })
}

/// Apply a display state: mode first, then scale. Returns the previous state.
///
/// The scale is clamped to the range the display supports *at the new mode* —
/// Windows widens the DPI ladder as resolution grows (e.g. 175% max at 1080p
/// but 300% at 2160p), so the range can only be known after the switch.
///
/// The caller is responsible for guard bookkeeping (see `resonance_core::guard`) —
/// this function only performs the transition.
pub fn apply(target: DisplayState) -> Result<DisplayState, String> {
    let previous = state()?;
    if previous == target {
        return Ok(previous);
    }
    if previous.mode != target.mode {
        display::switch_mode(target.mode)?;
    }
    let (min, max) = dpi::scale_range()?;
    let scale = target.scale.clamp(min, max);
    if display::current_scale()? != scale {
        if let Err(e) = dpi::set_scale(scale) {
            // Scale failed after a successful mode switch — roll the mode back
            // rather than leaving a half-applied state.
            if previous.mode != target.mode {
                display::switch_mode(previous.mode)?;
            }
            return Err(e);
        }
    }
    Ok(previous)
}

/// Convenience: the native state (panel-preferred mode at max refresh, 100%).
pub fn native_state() -> Result<DisplayState, String> {
    let (w, h) = display::native_resolution()?;
    let mode = display::resolve_mode(w, h, None)?;
    Ok(DisplayState { mode, scale: 100 })
}

pub use resonance_core::guard;

/// If a pending revert exists (crash mid-switch, unconfirmed switch from a dead
/// process), restore it. Returns the restored state if a restore happened.
pub fn restore_pending() -> Result<Option<DisplayState>, String> {
    match guard::pending()? {
        None => Ok(None),
        Some(p) => {
            apply(p.saved)?;
            guard::clear()?;
            Ok(Some(p.saved))
        }
    }
}

/// Guarded transition: persists the previous state before switching so that a
/// crash at any point leaves the system recoverable. Returns the previous state.
pub fn apply_guarded(target: DisplayState) -> Result<DisplayState, String> {
    let previous = state()?;
    if previous == target {
        return Ok(previous);
    }
    guard::save(previous)?;
    // On failure the guard file is intentionally kept so a later invocation
    // can still restore the saved state.
    apply(target)
}

/// Confirm the current state as intentional: drop the pending revert.
pub fn confirm() -> Result<(), String> {
    guard::clear()
}

/// Revert to the guarded previous state (or native if no guard exists).
pub fn revert() -> Result<DisplayState, String> {
    let target = match guard::pending()? {
        Some(p) => p.saved,
        None => native_state()?,
    };
    apply(target)?;
    guard::clear()?;
    Ok(target)
}

/// Check that a mode is at (or above) another in both dimensions.
pub fn is_above(mode: Mode, base: (u32, u32)) -> bool {
    mode.width > base.0 || mode.height > base.1
}
