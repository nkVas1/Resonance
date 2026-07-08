//! Resonance domain model — shared by tuner, conductor, resctl and Chamber.

pub mod config;
pub mod guard;
pub mod paths;
pub mod rules;

use serde::{Deserialize, Serialize};

/// A concrete display mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Mode {
    pub width: u32,
    pub height: u32,
    pub hz: u32,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{} @{}Hz", self.width, self.height, self.hz)
    }
}

/// Full display state Resonance manages: a mode plus the DPI scale.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct DisplayState {
    pub mode: Mode,
    pub scale: u32,
}

/// A named target the user can switch to.
///
/// `resolution: None` means "the panel's native mode" resolved at apply time;
/// `refresh: None` means "highest available refresh rate for that resolution".
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    #[serde(default)]
    pub resolution: Option<(u32, u32)>,
    #[serde(default)]
    pub refresh: Option<u32>,
    #[serde(default)]
    pub scale: Option<u32>,
}

impl Profile {
    pub fn is_native(&self) -> bool {
        self.resolution.is_none()
    }
}
