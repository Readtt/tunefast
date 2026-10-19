//! The user's choices, kept in `%APPDATA%\Tunefast\settings.json`: what the
//! original keeps in its profile storage and registry for these pages (mode,
//! max fan, auto switch to Eco, temperature unit, theme) plus what ECO changed,
//! so leaving ECO puts the display and the NVIDIA frame cap back as they were.
//! Demo and tooling runs never read or write it.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tunefast_core::bios::Mode;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// The mode the user last chose; re-applied after power events (the
    /// firmware has no read-back for it).
    pub mode: Mode,
    pub max_fan: bool,
    pub auto_eco: bool,
    pub fahrenheit: bool,
    pub light: bool,
    /// What entering ECO changed, cleared once it is put back.
    pub eco_saved: Option<EcoSaved>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { mode: Mode::Default, max_fan: false, auto_eco: false, fahrenheit: false, light: false, eco_saved: None }
    }
}

/// The state ECO replaced (the original's SaveNormalRefreshRate and
/// NvidiaMaxFrameRate).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EcoSaved {
    /// The internal display's refresh rate before ECO (None: not changed).
    pub refresh_hz: Option<u32>,
    /// NVIDIA "Max Frame Rate" before ECO: None when ECO did not touch it,
    /// Some(None) when it was not set (restoring deletes it), Some(Some(n)).
    pub frame_cap: Option<Option<u32>>,
}

pub fn path() -> Option<PathBuf> {
    let base = std::env::var_os(if cfg!(windows) { "APPDATA" } else { "HOME" }).map(PathBuf::from)?;
    Some(base.join(crate::APP_ID).join("settings.json"))
}

pub fn load() -> Settings {
    path()
        .and_then(|path| std::fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

/// Writes atomically (temporary file + rename): a crash never leaves half a file.
pub fn save_now(settings: &Settings) -> Result<(), String> {
    let path = path().ok_or_else(|| "no settings folder".to_owned())?;
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(settings).map_err(|e| e.to_string())?;
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&temporary, &path).map_err(|e| e.to_string())
}

/// The same, off the UI thread.
pub async fn save(settings: Settings) -> Result<(), String> {
    tokio::task::spawn_blocking(move || save_now(&settings)).await.map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip_and_tolerate_missing_fields() {
        let settings = Settings {
            mode: Mode::Quiet,
            max_fan: true,
            eco_saved: Some(EcoSaved { refresh_hz: Some(144), frame_cap: Some(None) }),
            ..Settings::default()
        };
        let json = serde_json::to_string(&settings).unwrap();
        assert_eq!(serde_json::from_str::<Settings>(&json).unwrap(), settings);
        let partial: Settings = serde_json::from_str(r#"{"fahrenheit": true}"#).unwrap();
        assert!(partial.fahrenheit && partial.mode == Mode::Default && partial.eco_saved.is_none());
    }
}
