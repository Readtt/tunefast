//! What the screens show. The same structure is the `--fixtures` format, so a
//! fixture can reproduce a reference screenshot's exact content.

use serde::{Deserialize, Serialize};
use tunefast_core::bios::Mode;

/// Whether the hardware helper (the elevated service) can be reached.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelperState {
    /// Connected: modes and fan controls work.
    Ready,
    /// Not installed yet: controls show their disabled look and an "Enable" action.
    #[default]
    Missing,
    /// Not this kind of laptop (or not Windows): controls stay disabled.
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Controls {
    pub mode: Mode,
    pub max_fan: bool,
    pub auto_eco: bool,
    /// ECO is on because auto switch kicked in (battery or weak adapter).
    pub auto_eco_active: bool,
    pub helper: HelperState,
}

impl Default for Controls {
    fn default() -> Self {
        Controls { mode: Mode::Default, max_fan: false, auto_eco: false, auto_eco_active: false, helper: HelperState::Ready }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Drive {
    pub name: String,
    pub free_gb: f64,
    pub total_gb: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Process {
    pub name: String,
    pub cpu: u32,
    pub gpu: u32,
    pub ram_mb: u32,
    /// The processes grouped under this name (STOP ends them all).
    pub pids: Vec<u32>,
}

/// The live numbers (System Vitals and the Performance Control readouts).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Vitals {
    pub cpu_temp: Option<f64>,
    pub gpu_temp: Option<f64>,
    pub cpu_util: Option<f64>,
    pub gpu_util: Option<f64>,
    /// The discrete GPU is powered down (hybrid graphics): shown as "Inactive".
    pub gpu_inactive: bool,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    /// Memory load in percent as Windows reports it (whole number).
    pub ram_load: Option<f64>,
    pub drives: Vec<Drive>,
    pub upload_mbps: f64,
    pub download_mbps: f64,
    pub processes: Vec<Process>,
    /// Fan speeds in RPM from the firmware (empty without the helper).
    pub fans_rpm: Vec<u32>,
}

impl Vitals {
    pub fn ram_util(&self) -> Option<f64> {
        self.ram_load.or_else(|| (self.ram_total_gb > 0.0).then(|| (self.ram_used_gb / self.ram_total_gb * 100.0).floor()))
    }
}

/// One chart line: up to `HISTORY` samples, oldest first, one a second.
pub type History = Vec<f64>;

/// Samples kept per chart line (the original's 31 points, x = 8 * i).
pub const HISTORY: usize = 31;

pub fn push_history(history: &mut History, value: f64) {
    history.push(value);
    if history.len() > HISTORY {
        history.remove(0);
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CpuDetail {
    pub name: String,
    /// Per logical processor, MHz.
    pub clocks_mhz: Vec<f64>,
    pub package_temp: Option<f64>,
    pub package_util: f64,
    /// Per logical processor, percent.
    pub utils: Vec<f64>,
    /// Average clock (MHz), temperature, utilisation.
    pub clock_history: History,
    pub temp_history: History,
    pub util_history: History,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GpuDetail {
    /// No discrete GPU: the section is hidden.
    pub absent: bool,
    pub name: String,
    /// Powered down by hybrid graphics: shown with the "Inactive" overlay.
    pub inactive: bool,
    pub core_clock_mhz: f64,
    pub memory_clock_mhz: f64,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub temp: Option<f64>,
    pub util_core: f64,
    pub util_frame_buffer: f64,
    pub util_video: f64,
    pub util_bus: f64,
    pub clock_history: History,
    pub temp_history: History,
    pub util_history: History,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct RamDetail {
    pub used_gb: f64,
    pub total_gb: f64,
    pub util_history: History,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetDetail {
    pub upload_mbps: f64,
    pub download_mbps: f64,
    pub upload_history: History,
    pub download_history: History,
}

/// The advanced view: CPU, GPU, RAM and network sections.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Advanced {
    pub cpu: CpuDetail,
    pub gpu: GpuDetail,
    pub ram: RamDetail,
    pub network: NetDetail,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemoryModule {
    pub part: String,
    pub capacity_gb: u32,
    pub kind: String,
    pub speed_mhz: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Gpu {
    pub name: String,
    pub refresh_hz: Option<u32>,
    pub driver: String,
}

/// System information (read once).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SysInfo {
    pub model: String,
    pub serial: String,
    pub os_name: String,
    pub os_release: String,
    pub os_version: String,
    pub os_build: String,
    pub cpu_name: String,
    pub cpu_clock_mhz: u32,
    pub cpu_description: String,
    pub gpus: Vec<Gpu>,
    pub memory: Vec<MemoryModule>,
    pub audio: Vec<String>,
}

/// Everything a screen can show. `--fixtures <file>` loads one of these.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    /// The laptop's name in the menu (from its model string).
    pub device: String,
    pub controls: Controls,
    pub vitals: Vitals,
    pub advanced: Advanced,
    pub sysinfo: SysInfo,
    pub fahrenheit: bool,
    pub version: String,
}

/// "VICTUS Laptop" style group name from the model string, or a plain one.
pub fn device_label(model: &str) -> String {
    let lower = model.to_lowercase();
    match lower.split_whitespace().next() {
        Some(first) if lower.contains("laptop") && first.chars().all(char::is_alphanumeric) => {
            format!("{} Laptop", first.to_uppercase())
        }
        _ => "This Laptop".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_label_from_model() {
        assert_eq!(device_label("Victus by HP Laptop 16-d1xxx"), "VICTUS Laptop");
        assert_eq!(device_label(""), "This Laptop");
    }

    #[test]
    fn fixtures_parse_with_missing_fields() {
        let store: Store = serde_json::from_str(r#"{"vitals":{"cpu_temp":61.0},"controls":{"mode":"Quiet"}}"#).unwrap();
        assert_eq!(store.vitals.cpu_temp, Some(61.0));
        assert_eq!(store.controls.mode, Mode::Quiet);
        assert!(store.sysinfo.audio.is_empty());
    }
}
