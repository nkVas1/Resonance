//! Crash-safe revert guard.
//!
//! Before any non-native switch the previous [`DisplayState`] is persisted here.
//! If the process dies mid-switch (crash, power loss), the next invocation finds
//! the pending file and restores the saved state. Cleared on explicit confirm.

use crate::{paths, DisplayState};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PendingRevert {
    pub saved: DisplayState,
    /// Unix seconds when the switch was made.
    pub created: u64,
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn save(saved: DisplayState) -> Result<(), String> {
    let path = paths::guard_file()?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    }
    let pending = PendingRevert {
        saved,
        created: now_unix(),
    };
    let text = toml::to_string(&pending).map_err(|e| format!("serialize guard: {e}"))?;
    std::fs::write(&path, text).map_err(|e| format!("write {}: {e}", path.display()))
}

pub fn pending() -> Result<Option<PendingRevert>, String> {
    let path = paths::guard_file()?;
    if !path.exists() {
        return Ok(None);
    }
    let text =
        std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let parsed = toml::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
    Ok(Some(parsed))
}

pub fn clear() -> Result<(), String> {
    let path = paths::guard_file()?;
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("remove {}: {e}", path.display())),
    }
}
