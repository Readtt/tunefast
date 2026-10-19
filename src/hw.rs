//! The live machine. Sensors are read on a background thread only while the
//! window is visible; firmware requests go to the helper service; system
//! information is read once. Off Windows everything reports "unsupported" and
//! the app shows its demo data.


use crate::model::{Advanced, Vitals};

/// One sample for the screens.
#[derive(Clone, Debug, Default)]
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
    pub fn end_processes(_pids: &[u32]) {}
}

#[cfg(windows)]
#[path = "hw_windows.rs"]
mod platform;

pub use platform::{ask_helper, end_processes, install_helper, on_ac, open_task_manager, print_text, read_system_info, sensor_thread};
