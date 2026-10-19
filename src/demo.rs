//! Built-in sample data for `--demo` (and CI, screenshots of the README):
//! invented but realistic, the same structure as live data. Never the user's
//! own machine details.

use tunefast_core::bios::Mode;

use crate::model::*;

pub fn store() -> Store {
    let clocks = vec![2817.0, 2746.0, 3034.0, 2931.0, 2620.0, 2443.0, 2535.0, 2295.0, 1451.0, 1502.0, 1622.0, 1396.0, 1455.0, 1502.0, 1531.0, 1609.0];
    let utils = vec![52.0, 11.1, 6.5, 53.4, 44.5, 6.5, 46.7, 21.1, 36.9, 11.3, 8.3, 51.7, 60.5, 29.4, 21.0, 13.5];
    Store {
        device: "VICTUS Laptop".to_owned(),
        controls: Controls { mode: Mode::Default, max_fan: false, auto_eco: false, auto_eco_active: false, helper: HelperState::Ready },
        vitals: Vitals {
            cpu_temp: Some(62.0),
            gpu_temp: Some(53.0),
            cpu_util: Some(17.0),
            gpu_util: Some(0.0),
            gpu_inactive: false,
            ram_used_gb: 9.6,
            ram_total_gb: 15.7,
            ram_load: Some(61.0),
            drives: vec![Drive { name: "C:\\".to_owned(), free_gb: 182.4, total_gb: 475.6 }],
            upload_mbps: 0.4,
            download_mbps: 2.7,
            processes: vec![
                Process { name: "Photo Editor".to_owned(), cpu: 9, gpu: 4, ram_mb: 612 },
                Process { name: "Music Player".to_owned(), cpu: 3, gpu: 0, ram_mb: 188 },
                Process { name: "Web Browser".to_owned(), cpu: 2, gpu: 1, ram_mb: 940 },
                Process { name: "Chat".to_owned(), cpu: 1, gpu: 0, ram_mb: 176 },
            ],
            fans_rpm: vec![3300, 3500],
        },
        advanced: Advanced {
            cpu: CpuDetail {
                name: "12th Gen Intel Core i5-12500H".to_owned(),
                clocks_mhz: clocks,
                package_temp: Some(62.0),
                package_util: 26.0,
                utils,
                clock_history: vec![2100.0, 2400.0, 2800.0, 2500.0, 2300.0, 2600.0, 2700.0, 2500.0, 2400.0, 2450.0],
                temp_history: vec![60.0, 62.0, 65.0, 63.0, 61.0, 62.0, 63.0, 62.0, 61.0, 62.0],
                util_history: vec![20.0, 24.0, 31.0, 22.0, 18.0, 25.0, 27.0, 24.0, 22.0, 26.0],
            },
            gpu: GpuDetail {
                absent: false,
                name: "NVIDIA GeForce RTX 3050 Ti Laptop GPU".to_owned(),
                inactive: false,
                core_clock_mhz: 210.0,
                memory_clock_mhz: 405.0,
                memory_used_gb: 0.3,
                memory_total_gb: 4.0,
                temp: Some(53.0),
                util_core: 0.0,
                util_frame_buffer: 0.0,
                util_video: 0.0,
                util_bus: 0.0,
                clock_history: vec![210.0; 10],
                temp_history: vec![53.0; 10],
                util_history: vec![0.0; 10],
            },
            ram: RamDetail { used_gb: 9.6, total_gb: 15.7, util_history: vec![61.0; 10] },
            network: NetDetail { upload_mbps: 0.4, download_mbps: 2.7, upload_history: vec![0.4; 10], download_history: vec![2.7; 10] },
        },
        sysinfo: SysInfo {
            model: "Gaming Laptop 16".to_owned(),
            serial: "DEMO000000".to_owned(),
            os_name: "Microsoft Windows 11 Home".to_owned(),
            os_release: "2009".to_owned(),
            os_version: "10.0.26200".to_owned(),
            os_build: "26200".to_owned(),
            cpu_name: "12th Gen Intel(R) Core(TM) i5-12500H".to_owned(),
            cpu_clock_mhz: 2500,
            cpu_description: "Intel64 Family 6 Model 154 Stepping 3".to_owned(),
            gpus: vec![
                Gpu { name: "NVIDIA GeForce RTX 3050 Ti Laptop GPU".to_owned(), refresh_hz: None, driver: "32.0.15.1714".to_owned() },
                Gpu { name: "Intel(R) Iris(R) Xe Graphics".to_owned(), refresh_hz: Some(144), driver: "31.0.101.4502".to_owned() },
            ],
            memory: vec![
                MemoryModule { part: "DDR5 module A".to_owned(), capacity_gb: 8, kind: "DDR5".to_owned(), speed_mhz: 4800 },
                MemoryModule { part: "DDR5 module B".to_owned(), capacity_gb: 8, kind: "DDR5".to_owned(), speed_mhz: 4800 },
            ],
            audio: vec!["High Definition Audio".to_owned(), "Bluetooth Audio".to_owned()],
        },
        fahrenheit: false,
        version: env!("CARGO_PKG_VERSION").to_owned(),
    }
}
