//! Versioned TOML config with schema migration on load.

use crate::rules::Rule;
use crate::{paths, Profile};
use serde::{Deserialize, Serialize};

pub const CURRENT_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    /// Seconds before an unconfirmed switch auto-reverts.
    #[serde(default = "default_confirm_timeout")]
    pub confirm_timeout_s: u32,
    #[serde(default)]
    pub profiles: Vec<Profile>,
    /// Automation rules (evaluated by the conductor). Empty by default.
    #[serde(default)]
    pub rules: Vec<Rule>,
}

fn default_confirm_timeout() -> u32 {
    12
}

impl Config {
    /// Default profile set, themed after the harmonic series:
    /// fundamental (native) · fifth (3:2 linear) · octave (2:1 linear).
    pub fn default_for(native_w: u32, native_h: u32) -> Self {
        Config {
            version: CURRENT_VERSION,
            confirm_timeout_s: default_confirm_timeout(),
            profiles: vec![
                Profile {
                    name: "fundamental".into(),
                    resolution: None,
                    refresh: None,
                    scale: Some(100),
                },
                Profile {
                    name: "fifth".into(),
                    resolution: Some((native_w * 3 / 2, native_h * 3 / 2)),
                    refresh: None,
                    scale: Some(150),
                },
                Profile {
                    name: "octave".into(),
                    resolution: Some((native_w * 2, native_h * 2)),
                    refresh: None,
                    scale: Some(200),
                },
            ],
            rules: Vec::new(),
        }
    }

    /// Load config, creating the default one on first run.
    pub fn load_or_init(native_w: u32, native_h: u32) -> Result<Self, String> {
        let path = paths::config_file()?;
        if !path.exists() {
            let config = Self::default_for(native_w, native_h);
            config.save()?;
            return Ok(config);
        }
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let config: Config =
            toml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
        if config.version > CURRENT_VERSION {
            return Err(format!(
                "config version {} is newer than this build supports ({CURRENT_VERSION})",
                config.version
            ));
        }
        // Future migrations branch on config.version here.
        Ok(config)
    }

    pub fn save(&self) -> Result<(), String> {
        let path = paths::config_file()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
        }
        let text = toml::to_string_pretty(self).map_err(|e| format!("serialize config: {e}"))?;
        std::fs::write(&path, text).map_err(|e| format!("write {}: {e}", path.display()))
    }

    pub fn profile(&self, name: &str) -> Option<&Profile> {
        self.profiles
            .iter()
            .find(|p| p.name.eq_ignore_ascii_case(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profiles_follow_harmonic_ratios() {
        let c = Config::default_for(1920, 1080);
        assert_eq!(c.profile("fifth").unwrap().resolution, Some((2880, 1620)));
        assert_eq!(c.profile("octave").unwrap().resolution, Some((3840, 2160)));
        assert!(c.profile("fundamental").unwrap().is_native());
    }

    #[test]
    fn roundtrips_through_toml() {
        let c = Config::default_for(2560, 1440);
        let text = toml::to_string_pretty(&c).unwrap();
        let back: Config = toml::from_str(&text).unwrap();
        assert_eq!(back.profiles, c.profiles);
        assert_eq!(back.version, CURRENT_VERSION);
    }
}
