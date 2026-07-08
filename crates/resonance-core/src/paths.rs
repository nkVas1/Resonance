//! Filesystem locations. Config lives in roaming %APPDATA%, volatile state in %LOCALAPPDATA%.

use std::path::PathBuf;

fn env_dir(var: &str) -> Result<PathBuf, String> {
    std::env::var_os(var)
        .map(PathBuf::from)
        .ok_or_else(|| format!("%{var}% is not set"))
}

pub fn config_dir() -> Result<PathBuf, String> {
    Ok(env_dir("APPDATA")?.join("Resonance"))
}

pub fn config_file() -> Result<PathBuf, String> {
    Ok(config_dir()?.join("config.toml"))
}

pub fn state_dir() -> Result<PathBuf, String> {
    Ok(env_dir("LOCALAPPDATA")?.join("Resonance").join("state"))
}

pub fn guard_file() -> Result<PathBuf, String> {
    Ok(state_dir()?.join("pending-revert.toml"))
}
