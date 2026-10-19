//! The last thing the app showed, kept on disk so the next start draws real
//! numbers in its very first frame instead of empty tiles: the last sensor
//! readings, system information, screen and window size. Fresh readings
//! replace it within a second. Demo and tooling runs never read or write it.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::Store;

/// Bump when `Snapshot` or `Store` change shape: an old file is then ignored.
const VERSION: u32 = 2;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshot {
    pub version: u32,
    pub store: Store,
    pub screen: String,
    /// Window size in logical points while not maximized.
    pub window: Option<[f32; 2]>,
    pub maximized: bool,
}

/// `%LOCALAPPDATA%\Tunefast\snapshot.json` on Windows, the cache folder elsewhere.
pub fn path() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
    };
    base.map(|folder| folder.join(crate::APP_ID).join("snapshot.json"))
}

/// The snapshot from the last run, if there is a readable one of this version.
pub fn load() -> Option<Snapshot> {
    let bytes = std::fs::read(path()?).ok()?;
    let snapshot: Snapshot = serde_json::from_slice(&bytes).ok()?;
    (snapshot.version == VERSION).then_some(snapshot)
}

/// Writes the snapshot now (on close). A temporary file plus a rename means a
/// crash mid-write never leaves a broken file behind.
pub fn save_now(mut snapshot: Snapshot) -> Result<(), String> {
    snapshot.version = VERSION;
    let path = path().ok_or_else(|| "no cache folder".to_owned())?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, &path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_snapshot_round_trips_through_json() {
        let snapshot = Snapshot {
            version: VERSION,
            store: crate::demo::store(),
            screen: "vitals".to_owned(),
            window: Some([1280.0, 800.0]),
            maximized: false,
        };
        let json = serde_json::to_vec(&snapshot).unwrap();
        let back: Snapshot = serde_json::from_slice(&json).unwrap();
        assert_eq!(back.screen, "vitals");
        assert_eq!(back.store.vitals.processes.len(), snapshot.store.vitals.processes.len());
        assert_eq!(back.window, Some([1280.0, 800.0]));
    }
}
