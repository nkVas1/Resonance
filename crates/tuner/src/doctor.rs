//! Capability probe: everything Resonance needs to know about this machine,
//! with honest "why not" diagnostics.

use crate::vendor::Vendor;
use crate::{display, dpi, vendor};
use resonance_core::Mode;

#[derive(Debug)]
pub struct Report {
    pub adapter: String,
    pub monitor: String,
    pub vendor: Vendor,
    pub nvapi_present: bool,
    pub native: (u32, u32),
    pub current: Mode,
    pub current_scale: u32,
    pub scale_range: (u32, u32),
    /// Distinct above-native resolutions (the super-resolution ladder).
    pub above_native: Vec<(u32, u32, u32)>, // (w, h, max hz)
}

pub fn run() -> Result<Report, String> {
    let (adapter, monitor) = display::device_names()?;
    let vendor = Vendor::detect(&adapter);
    let nvapi_present = vendor == Vendor::Nvidia && vendor::nvapi_present();
    let native = display::native_resolution()?;
    let current = display::current_mode()?;
    let current_scale = display::current_scale()?;
    let scale_range = dpi::scale_range()?;

    let mut above: Vec<(u32, u32, u32)> = Vec::new();
    for m in display::list_modes() {
        if m.width > native.0 || m.height > native.1 {
            match above
                .iter_mut()
                .find(|(w, h, _)| *w == m.width && *h == m.height)
            {
                Some(entry) => entry.2 = entry.2.max(m.hz),
                None => above.push((m.width, m.height, m.hz)),
            }
        }
    }

    Ok(Report {
        adapter,
        monitor,
        vendor,
        nvapi_present,
        native,
        current,
        current_scale,
        scale_range,
        above_native: above,
    })
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let nvapi = if self.nvapi_present {
            " · NVAPI present"
        } else {
            ""
        };
        writeln!(f, "adapter : {} [{}{nvapi}]", self.adapter, self.vendor)?;
        writeln!(
            f,
            "monitor : {} (native {}x{})",
            self.monitor, self.native.0, self.native.1
        )?;
        writeln!(
            f,
            "current : {} @ {}% scale",
            self.current, self.current_scale
        )?;
        writeln!(
            f,
            "scale   : {}%..{}%",
            self.scale_range.0, self.scale_range.1
        )?;
        if self.above_native.is_empty() {
            writeln!(
                f,
                "super-resolution: none available — no above-native modes exposed"
            )?;
            writeln!(f, "  enable {}:", self.vendor.feature_name())?;
            writeln!(f, "  {}", self.vendor.enable_hint())?;
        } else {
            writeln!(
                f,
                "super-resolution: {} above-native mode(s) ready:",
                self.above_native.len()
            )?;
            for (w, h, hz) in &self.above_native {
                let ratio = (*w as f64) / (self.native.0 as f64);
                writeln!(f, "  {w}x{h} up to {hz}Hz ({ratio:.2}x linear)")?;
            }
        }
        Ok(())
    }
}
