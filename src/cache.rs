//! The last thing the app showed, kept on disk so the next start draws real
//! content in its very first frame instead of an empty window or a spinner.
//! The app shows the snapshot at once and refreshes it from the network in
//! the background (stale-while-revalidate). Demo and tooling runs never read
//! or write it.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::model::Store;

/// Bump when `Snapshot` or `Store` change shape: an old file is then ignored.
const VERSION: u32 = 1;
/// Newest messages kept per conversation, so reading the file never delays
/// the first frame (a few hundred KB parse in a couple of milliseconds).
const MESSAGES_PER_CONVERSATION: usize = 60;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Snapshot {
    pub version: u32,
    pub store: Store,
    pub selected: Option<u64>,
    pub screen: String,
    /// Window size in logical points while not maximized.
    pub window: Option<[f32; 2]>,
    pub maximized: bool,
}

/// `%LOCALAPPDATA%\<app id>\snapshot.json` on Windows, the cache folder elsewhere.
pub fn path() -> Option<PathBuf> {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Caches"))
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

/// Writes the snapshot from a background thread (after a refresh).
pub async fn save(snapshot: Snapshot) -> Result<(), String> {
    tokio::task::spawn_blocking(move || save_now(snapshot)).await.map_err(|e| e.to_string())?
}

/// Writes the snapshot now (on close). A temporary file plus a rename means a
/// crash mid-write never leaves a broken file behind.
pub fn save_now(mut snapshot: Snapshot) -> Result<(), String> {
    snapshot.version = VERSION;
    trim(&mut snapshot.store);
    let path = path().ok_or_else(|| "no cache folder".to_owned())?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, &path).map_err(|e| e.to_string())
}

/// Keeps the newest messages of each conversation, in their original order.
fn trim(store: &mut Store) {
    let mut seen: std::collections::HashMap<u64, usize> = std::collections::HashMap::new();
    let mut keep = vec![false; store.messages.len()];
    for (index, message) in store.messages.iter().enumerate().rev() {
        let count = seen.entry(message.conversation).or_default();
        if *count < MESSAGES_PER_CONVERSATION {
            *count += 1;
            keep[index] = true;
        }
    }
    let mut index = 0;
    store.messages.retain(|_| {
        index += 1;
        keep[index - 1]
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Message;

    #[test]
    fn trimming_keeps_the_newest_messages_of_each_conversation() {
        let mut store = Store::default();
        for id in 0..200u64 {
            store.messages.push(Message {
                id,
                conversation: id % 2,
                author: "a".into(),
                body: id.to_string(),
                time: String::new(),
                mine: false,
            });
        }
        trim(&mut store);
        assert_eq!(store.messages.len(), 2 * MESSAGES_PER_CONVERSATION);
        assert_eq!(store.messages.last().map(|m| m.id), Some(199));
        assert_eq!(store.messages.first().map(|m| m.id), Some(200 - 2 * MESSAGES_PER_CONVERSATION as u64));
        assert!(store.messages.windows(2).all(|pair| pair[0].id < pair[1].id));
    }

    #[test]
    fn a_snapshot_round_trips_through_json() {
        let snapshot = Snapshot { version: VERSION, selected: Some(3), screen: "home".into(), ..Snapshot::default() };
        let text = serde_json::to_string(&snapshot).unwrap();
        let back: Snapshot = serde_json::from_str(&text).unwrap();
        assert_eq!(back.selected, Some(3));
        assert_eq!(back.screen, "home");
    }
}
