//! The live machine. Sensors are read on a background thread only while the
//! window is visible; firmware requests go to the helper service; system
//! information is read once. Off Windows everything reports "unsupported" and
//! the app shows its demo data.


use crate::model::{Advanced, Vitals};

/// The page on screen, so the sensor thread reads only what it shows (the
/// original's pages each start their own monitors): 0 Performance Control,
/// 1 System Vitals basic, 2 advanced, 3 system information.
pub static PAGE: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn show_page(screen: crate::screens::Screen) {
    use crate::screens::Screen;
    let page = match screen {
        Screen::Performance => 0,
        Screen::Vitals => 1,
        Screen::VitalsAdvanced => 2,
        Screen::Sysinfo => 3,
    };
    PAGE.store(page, std::sync::atomic::Ordering::Relaxed);
}

/// One sample for the screens.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize)]
pub struct Reading {
    pub vitals: Vitals,
    pub advanced: Advanced,
    pub on_ac: bool,
}

#[cfg(not(windows))]
mod platform {
    use super::*;
    use crate::model::SysInfo;
    use tunefast_core::protocol::{Request, Response};
    pub fn ask_helper(_request: Request) -> Response {
        Response::failure("unsupported")
    }
    pub fn read_system_info() -> SysInfo {
        SysInfo::default()
    }
    pub fn sensor_thread(_tx: tokio::sync::mpsc::Sender<Reading>) {}
    pub fn on_ac() -> bool {
        true
    }
    pub fn install_helper() -> bool {
        false
    }
    pub fn print_text(_text: &str) {}
    pub fn open_task_manager() {}
    pub fn end_processes(_pids: &[u32], _exes: &[String]) {}
}

#[cfg(windows)]
#[path = "hw_windows.rs"]
mod platform;

pub use platform::{ask_helper, end_processes, install_helper, on_ac, open_task_manager, print_text, read_system_info, sensor_thread};
