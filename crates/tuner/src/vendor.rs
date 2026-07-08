//! GPU vendor detection and super-resolution capability reporting.
//!
//! Resonance's core chain — above-native display modes + mode switch + DPI
//! compensation — is **vendor-agnostic**: it works for any GPU that exposes
//! above-native modes to Windows (NVIDIA DSR/DLDSR, AMD VSR, Intel custom
//! scaling). This module identifies the vendor and gives the user accurate,
//! vendor-specific guidance when those modes aren't present yet.

use std::ffi::c_void;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Vendor {
    Nvidia,
    Amd,
    Intel,
    Unknown,
}

impl Vendor {
    /// Identify the vendor from an adapter description string.
    pub fn detect(adapter: &str) -> Vendor {
        let a = adapter.to_ascii_lowercase();
        if a.contains("nvidia")
            || a.contains("geforce")
            || a.contains("quadro")
            || a.contains("rtx")
        {
            Vendor::Nvidia
        } else if a.contains("amd") || a.contains("radeon") || a.contains("ati ") {
            Vendor::Amd
        } else if a.contains("intel") || a.contains("arc") || a.contains("iris") {
            Vendor::Intel
        } else {
            Vendor::Unknown
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Vendor::Nvidia => "NVIDIA",
            Vendor::Amd => "AMD",
            Vendor::Intel => "Intel",
            Vendor::Unknown => "Unknown",
        }
    }

    /// The vendor's name for the super-resolution feature the user must enable.
    pub fn feature_name(self) -> &'static str {
        match self {
            Vendor::Nvidia => "DSR / DLDSR",
            Vendor::Amd => "Virtual Super Resolution (VSR)",
            Vendor::Intel => "Retro Scaling / custom resolutions",
            Vendor::Unknown => "GPU super-resolution",
        }
    }

    /// Where the user turns that feature on when no above-native modes exist yet.
    pub fn enable_hint(self) -> &'static str {
        match self {
            Vendor::Nvidia => {
                "NVIDIA Control Panel → Manage 3D settings → DSR - Factors (or the NVIDIA app)"
            }
            Vendor::Amd => "AMD Software → Display → Virtual Super Resolution → Enabled",
            Vendor::Intel => "Intel Graphics Command Center → Display → Custom Resolutions",
            Vendor::Unknown => "your GPU's control panel (enable super-resolution / GPU scaling)",
        }
    }
}

impl std::fmt::Display for Vendor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn LoadLibraryW(name: *const u16) -> *mut c_void;
    fn FreeLibrary(module: *mut c_void) -> i32;
}

/// Whether the NVIDIA driver API is present (informational — the core chain
/// does not require it, since above-native modes appear as standard modes).
pub fn nvapi_present() -> bool {
    let name: Vec<u16> = "nvapi64.dll\0".encode_utf16().collect();
    let handle = unsafe { LoadLibraryW(name.as_ptr()) };
    if handle.is_null() {
        false
    } else {
        unsafe { FreeLibrary(handle) };
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_common_adapters() {
        assert_eq!(Vendor::detect("NVIDIA GeForce RTX 3060 Ti"), Vendor::Nvidia);
        assert_eq!(Vendor::detect("AMD Radeon RX 7900 XTX"), Vendor::Amd);
        assert_eq!(
            Vendor::detect("Intel(R) Arc(TM) A770 Graphics"),
            Vendor::Intel
        );
        assert_eq!(Vendor::detect("Intel(R) UHD Graphics 630"), Vendor::Intel);
        assert_eq!(
            Vendor::detect("Microsoft Basic Render Driver"),
            Vendor::Unknown
        );
    }
}
