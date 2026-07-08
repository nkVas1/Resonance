//! Automation rule data model (behavior lives in the `conductor` crate).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PowerSource {
    Ac,
    Battery,
}

/// What causes a rule to fire.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Trigger {
    /// Process owns the foreground window (image name, case-insensitive).
    Foreground(String),
    /// Process is running anywhere (image name, case-insensitive).
    Running(String),
    /// Machine power source.
    Power(PowerSource),
}

/// What to restore when the trigger stops matching.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Restore {
    /// Return to the state that was active before this rule fired.
    #[default]
    OnExit,
    /// Keep the profile until something else changes it.
    Manual,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Rule {
    pub name: String,
    pub trigger: Trigger,
    /// Profile name from the Resonance config.
    pub profile: String,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub restore: Restore,
}
