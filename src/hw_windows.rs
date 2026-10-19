//! Windows sensors, system info and the helper client. Nothing here needs
//! admin rights: firmware requests go to the helper service; sensors come
//! from PDH counters, the ACPI thermal zone (WMI), NVML and the process list.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::time::{Duration, Instant};

use tunefast_core::bios;
use tunefast_core::protocol::{PIPE_NAME, Request, Response};
use tunefast_core::sensors::TemperatureSmoother;
use windows::Win32::System::Performance::{
    PDH_FMT_COUNTERVALUE, PDH_FMT_COUNTERVALUE_ITEM_W, PDH_FMT_DOUBLE, PDH_HCOUNTER, PDH_HQUERY, PDH_MORE_DATA, PdhAddEnglishCounterW,
    PdhCollectQueryData,
    PdhGetFormattedCounterArrayW, PdhGetFormattedCounterValue, PdhOpenQueryW,
};
use windows::core::{HSTRING, PCWSTR};
use wmi::WMIConnection;

use super::Reading;
use crate::model::{Advanced, Drive, Gpu, GpuDetail, MemoryModule, Process, SysInfo, Vitals, push_history};

// ------------------------------------------------------------ helper client

/// One request to the helper service over its named pipe. A refused or absent
/// pipe becomes a plain failure; the caller shows the "turn on hardware
/// control" state rather than an error.
pub fn ask_helper(request: Request) -> Response {
    match talk(&request) {
        Ok(response) => response,
        Err(error) => Response::failure(error),
    }
}

fn talk(request: &Request) -> Result<Response, String> {
    use std::fs::OpenOptions;
    let mut pipe = OpenOptions::new()
        .read(true)
        .write(true)
        .open(PIPE_NAME)
        .map_err(|e| format!("helper not reachable: {e}"))?;
    pipe.write_all(request.encode().as_bytes()).map_err(|e| e.to_string())?;
    pipe.flush().ok();
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 512];
    loop {
        let read = pipe.read(&mut chunk).map_err(|e| e.to_string())?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..read]);
        if buffer.contains(&b'\n') {
            break;
        }
    }
    let line = String::from_utf8_lossy(&buffer);
    Response::decode(line.lines().next().unwrap_or_default())
}

/// Installs the helper service with the user's approval (Windows shows its
/// elevation prompt for tunefast-helper.exe next to this program), then checks it answers.
pub fn install_helper() -> bool {
    use windows::Win32::System::Threading::{INFINITE, WaitForSingleObject};
    use windows::Win32::UI::Shell::{SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW, ShellExecuteExW};
    let Some(helper) = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join("tunefast-helper.exe"))) else {
        return false;
    };
    if !helper.exists() {
        log::warn!("no helper at {}", helper.display());
        return false;
    }
    let file = HSTRING::from(helper.as_os_str());
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: windows::core::w!("runas"),
        lpFile: PCWSTR(file.as_ptr()),
        lpParameters: windows::core::w!("install"),
        nShow: 0,
        ..Default::default()
    };
    unsafe {
        if ShellExecuteExW(&mut info).is_err() {
            return false; // the user declined the prompt
        }
        if !info.hProcess.is_invalid() {
            WaitForSingleObject(info.hProcess, INFINITE);
        }
    }
    std::thread::sleep(Duration::from_millis(500));
    ask_helper(Request::Hello).ok
}

/// PRINT: the system information as a text file handed to the default printer
/// through the shell's "print" verb (Notepad prints .txt files).
pub fn print_text(text: &str) {
    let path = std::env::temp_dir().join("Tunefast system information.txt");
    if std::fs::write(&path, text).is_ok() {
        shell("print", &path.to_string_lossy());
    }
}

/// STOP in TOP PROCESSES: ends the listed processes, as the original does.
pub fn end_processes(pids: &[u32]) {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_TERMINATE, TerminateProcess};
    for pid in pids {
        unsafe {
            if let Ok(handle) = OpenProcess(PROCESS_TERMINATE, false, *pid) {
                let _ = TerminateProcess(handle, 1);
                let _ = CloseHandle(handle);
            }
        }
    }
}

pub fn open_task_manager() {
    shell("open", "taskmgr.exe");
}

fn shell(verb: &str, file: &str) {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let (verb, file) = (HSTRING::from(verb), HSTRING::from(file));
    unsafe {
        ShellExecuteW(None, PCWSTR(verb.as_ptr()), PCWSTR(file.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}

// --------------------------------------------------------------- sensor loop

/// Reads CPU/GPU/RAM/network/processes and (through the helper) fan speed
/// once a second, sending a `Reading` each time, until the receiver is gone
/// (the window was minimized or closed).
pub fn sensor_thread(tx: tokio::sync::mpsc::Sender<Reading>) {
    let mut cpu = CpuSampler::new();
    let mut gpu = GpuSampler::new();
    let mut net = NetSampler::new();
    let mut processes = ProcessSampler::new();
    let mut smoother = TemperatureSmoother::default();
    let mut advanced = Advanced::default();
    advanced.cpu.name = clean_cpu_name(&processor_name());
    let mut tick: u64 = 0;
    loop {
        let started = Instant::now();
        if tx.is_closed() {
            return;
        }
        let c = cpu.sample();
        let temp = cpu_temperature().map(|t| smoother.push(t));
        let g = gpu.sample();
        let (up, down) = net.sample();
        let (used, total, load) = memory();
        // The original refreshes the process list every 2.5 s and storage less often.
        let top = if tick % 3 == 0 { Some(processes.sample(&gpu_by_process())) } else { None };
        let fans = fan_speeds();

        let average_clock = if c.clocks.is_empty() { 0.0 } else { c.clocks.iter().sum::<f64>() / c.clocks.len() as f64 };
        let a = &mut advanced;
        a.cpu.clocks_mhz = c.clocks.clone();
        a.cpu.package_temp = temp;
        a.cpu.package_util = c.total;
        a.cpu.utils = c.utils.clone();
        push_history(&mut a.cpu.clock_history, average_clock);
        push_history(&mut a.cpu.temp_history, temp.unwrap_or(0.0));
        push_history(&mut a.cpu.util_history, c.total);
        a.gpu = GpuDetail {
            clock_history: std::mem::take(&mut a.gpu.clock_history),
            temp_history: std::mem::take(&mut a.gpu.temp_history),
            util_history: std::mem::take(&mut a.gpu.util_history),
            ..g.detail.clone()
        };
        push_history(&mut a.gpu.clock_history, g.detail.core_clock_mhz);
        push_history(&mut a.gpu.temp_history, g.detail.temp.unwrap_or(0.0));
        push_history(&mut a.gpu.util_history, g.detail.util_core);
        a.ram.used_gb = used;
        a.ram.total_gb = total;
        push_history(&mut a.ram.util_history, if total > 0.0 { used / total * 100.0 } else { 0.0 });
        a.network.upload_mbps = up;
        a.network.download_mbps = down;
        push_history(&mut a.network.upload_history, up);
        push_history(&mut a.network.download_history, down);

        let reading = Reading {
            vitals: Vitals {
                cpu_temp: temp,
                gpu_temp: g.detail.temp,
                cpu_util: Some(c.total),
                gpu_util: (!g.detail.inactive && !g.detail.absent).then_some(g.detail.util_core),
                gpu_inactive: g.detail.inactive,
                ram_used_gb: used,
                ram_total_gb: total,
                ram_load: Some(load),
                drives: fixed_drives(),
                upload_mbps: up,
                download_mbps: down,
                processes: top.unwrap_or_else(|| processes.last.clone()),
                fans_rpm: fans,
            },
            advanced: advanced.clone(),
            on_ac: on_ac(),
        };
        if tx.blocking_send(reading).is_err() {
            return;
        }
        tick += 1;
        std::thread::sleep(Duration::from_millis(1000).saturating_sub(started.elapsed()));
    }
}

fn fan_speeds() -> Vec<u32> {
    match ask_helper(Request::GetFanLevel) {
        Response { ok: true, data, .. } => bios::fan_rpms(&data),
        _ => Vec::new(),
    }
}

// ------------------------------------------------------------------- PDH

/// A PDH query with wildcard counters, read as instance -> value.
struct Pdh {
    query: PDH_HQUERY,
    counters: Vec<PDH_HCOUNTER>,
}

// SAFETY: PDH handles may be used from any thread; each Pdh is used by one at a time.
unsafe impl Send for Pdh {}

impl Pdh {
    fn new(paths: &[&str]) -> Option<Pdh> {
        let mut query = PDH_HQUERY::default();
        unsafe {
            if PdhOpenQueryW(PCWSTR::null(), 0, &mut query) != 0 {
                return None;
            }
            let mut counters = Vec::new();
            for path in paths {
                let mut counter = PDH_HCOUNTER::default();
                if PdhAddEnglishCounterW(query, &HSTRING::from(*path), 0, &mut counter) != 0 {
                    return None;
                }
                counters.push(counter);
            }
            let _ = PdhCollectQueryData(query);
            Some(Pdh { query, counters })
        }
    }

    fn collect(&self) -> bool {
        unsafe { PdhCollectQueryData(self.query) == 0 }
    }

    fn array(&self, index: usize) -> HashMap<String, f64> {
        let mut values = HashMap::new();
        let counter = self.counters[index];
        unsafe {
            let (mut size, mut count) = (0u32, 0u32);
            let status = PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, None);
            if status != PDH_MORE_DATA || size == 0 {
                return values;
            }
            let mut buffer = vec![0u8; size as usize];
            let items = buffer.as_mut_ptr().cast::<PDH_FMT_COUNTERVALUE_ITEM_W>();
            if PdhGetFormattedCounterArrayW(counter, PDH_FMT_DOUBLE, &mut size, &mut count, Some(items)) != 0 {
                return values;
            }
            for item in std::slice::from_raw_parts(items, count as usize) {
                if let Ok(name) = item.szName.to_string() {
                    values.insert(name, item.FmtValue.Anonymous.doubleValue);
                }
            }
        }
        values
    }

    fn single(&self, index: usize) -> Option<f64> {
        let mut value = PDH_FMT_COUNTERVALUE::default();
        unsafe {
            (PdhGetFormattedCounterValue(self.counters[index], PDH_FMT_DOUBLE, None, &mut value) == 0)
                .then(|| value.Anonymous.doubleValue)
        }
    }
}

struct CpuSample {
    total: f64,
    utils: Vec<f64>,
    clocks: Vec<f64>,
}

/// Utilisation and effective clock per logical processor, as Task Manager
/// computes them: % Processor Utility, and Processor Frequency x % Processor
/// Performance / 100.
struct CpuSampler {
    pdh: Option<Pdh>,
}

impl CpuSampler {
    fn new() -> Self {
        CpuSampler {
            pdh: Pdh::new(&[
                r"\Processor Information(*)\% Processor Utility",
                r"\Processor Information(*)\% Processor Performance",
                r"\Processor Information(*)\Processor Frequency",
                r"\Processor Information(_Total)\% Processor Utility",
            ]),
        }
    }

    fn sample(&mut self) -> CpuSample {
        let mut sample = CpuSample { total: 0.0, utils: Vec::new(), clocks: Vec::new() };
        let Some(pdh) = &self.pdh else { return sample };
        if !pdh.collect() {
            return sample;
        }
        let utility = pdh.array(0);
        let performance = pdh.array(1);
        let frequency = pdh.array(2);
        let mut cores: Vec<(u32, u32, String)> = utility
            .keys()
            .filter_map(|name| {
                let (group, index) = name.split_once(',')?;
                Some((group.parse().ok()?, index.parse().ok()?, name.clone()))
            })
            .collect();
        cores.sort();
        for (_, _, name) in cores {
            sample.utils.push(utility[&name].clamp(0.0, 100.0));
            let mhz = frequency.get(&name).copied().unwrap_or(0.0) * performance.get(&name).copied().unwrap_or(0.0) / 100.0;
            sample.clocks.push(mhz.round());
        }
        sample.total = pdh.single(3).unwrap_or(0.0).clamp(0.0, 100.0);
        sample
    }
}

fn processor_name() -> String {
    wmi_string("Win32_Processor", "Name").unwrap_or_default()
}

/// "12th Gen Intel(R) Core(TM) i5-12500H" -> "12th Gen Intel Core i5-12500H", as the advanced view titles it.
fn clean_cpu_name(name: &str) -> String {
    name.replace("(R)", "").replace("(TM)", "").split_whitespace().collect::<Vec<_>>().join(" ")
}

/// CPU temperature from the ACPI thermal zone (firmware-reported; no driver).
/// `MSAcpi_ThermalZoneTemperature` reports tenths of a kelvin.
fn cpu_temperature() -> Option<f64> {
    #[derive(serde::Deserialize)]
    #[serde(rename = "MSAcpi_ThermalZoneTemperature")]
    struct Zone {
        #[serde(rename = "CurrentTemperature")]
        current_temperature: u32,
    }
    let wmi = WMIConnection::with_namespace_path("ROOT\\WMI").ok()?;
    let zones: Vec<Zone> = wmi.raw_query("SELECT CurrentTemperature FROM MSAcpi_ThermalZoneTemperature").ok()?;
    let max = zones.iter().map(|z| z.current_temperature).max()?;
    (max > 0).then(|| f64::from(max) / 10.0 - 273.15)
}

// ------------------------------------------------------------------- GPU

struct GpuSample {
    detail: GpuDetail,
}

/// The discrete NVIDIA GPU through NVML, asked only while it is powered: a
/// GPU that hybrid graphics has put to sleep (D3) is shown as inactive and
/// left asleep, as the original does.
struct GpuSampler {
    nvml: Option<nvml_wrapper::Nvml>,
    present: Option<bool>,
}

impl GpuSampler {
    fn new() -> Self {
        GpuSampler { nvml: None, present: None }
    }

    fn sample(&mut self) -> GpuSample {
        let mut detail = GpuDetail::default();
        let awake = nvidia_awake();
        if self.present.is_none() {
            self.present = Some(awake.is_some());
        }
        match awake {
            None => {
                detail.absent = true;
                return GpuSample { detail };
            }
            Some(false) => {
                detail.inactive = true;
                detail.name = self.cached_name();
                return GpuSample { detail };
            }
            Some(true) => {}
        }
        if self.nvml.is_none() {
            self.nvml = nvml_wrapper::Nvml::init().ok();
        }
        let Some(nvml) = &self.nvml else {
            detail.inactive = true;
            return GpuSample { detail };
        };
        let Ok(device) = nvml.device_by_index(0) else {
            detail.inactive = true;
            return GpuSample { detail };
        };
        use nvml_wrapper::enum_wrappers::device::{Clock, PcieUtilCounter, TemperatureSensor};
        detail.name = device.name().unwrap_or_default();
        detail.temp = device.temperature(TemperatureSensor::Gpu).ok().map(f64::from);
        if let Ok(util) = device.utilization_rates() {
            detail.util_core = f64::from(util.gpu);
            detail.util_frame_buffer = f64::from(util.memory);
        }
        let encoder = device.encoder_utilization().map(|u| u.utilization).unwrap_or(0);
        let decoder = device.decoder_utilization().map(|u| u.utilization).unwrap_or(0);
        detail.util_video = f64::from(encoder.max(decoder));
        // Bus: PCIe traffic against the link's bandwidth (KB/s against MB/s per lane).
        let traffic = device.pcie_throughput(PcieUtilCounter::Send).unwrap_or(0) + device.pcie_throughput(PcieUtilCounter::Receive).unwrap_or(0);
        let lane_mb = match device.current_pcie_link_gen().unwrap_or(0) {
            1 => 250.0,
            2 => 500.0,
            3 => 985.0,
            4 => 1969.0,
            5 => 3938.0,
            _ => 0.0,
        };
        let width = f64::from(device.current_pcie_link_width().unwrap_or(0));
        if lane_mb > 0.0 && width > 0.0 {
            detail.util_bus = (f64::from(traffic) / 1024.0 / (lane_mb * width) * 100.0).clamp(0.0, 100.0);
        }
        detail.core_clock_mhz = device.clock_info(Clock::Graphics).map(f64::from).unwrap_or(0.0);
        detail.memory_clock_mhz = device.clock_info(Clock::Memory).map(f64::from).unwrap_or(0.0);
        if let Ok(memory) = device.memory_info() {
            detail.memory_used_gb = memory.used as f64 / 1_073_741_824.0;
            detail.memory_total_gb = memory.total as f64 / 1_073_741_824.0;
        }
        // The original treats a GPU reporting neither load nor temperature as inactive.
        detail.inactive = detail.util_core <= 0.0 && detail.temp.unwrap_or(0.0) <= 0.0;
        GpuSample { detail }
    }

    fn cached_name(&self) -> String {
        static NAME: std::sync::OnceLock<String> = std::sync::OnceLock::new();
        NAME.get_or_init(|| video_controllers().into_iter().map(|g| g.name).find(|n| n.to_uppercase().contains("NVIDIA")).unwrap_or_default())
            .clone()
    }
}

/// Whether the NVIDIA display adapter is powered (D0): None without one.
/// Reads the device's power data from the configuration manager, which does
/// not wake the GPU.
fn nvidia_awake() -> Option<bool> {
    use windows::Win32::Devices::DeviceAndDriverInstallation::{
        DIGCF_PRESENT, GUID_DEVCLASS_DISPLAY, SP_DEVINFO_DATA, SPDRP_HARDWAREID, SetupDiDestroyDeviceInfoList, SetupDiEnumDeviceInfo,
        SetupDiGetClassDevsW, SetupDiGetDevicePropertyW, SetupDiGetDeviceRegistryPropertyW,
    };
    use windows::Win32::Devices::Properties::{DEVPKEY_Device_PowerData, DEVPROPTYPE};
    unsafe {
        let set = SetupDiGetClassDevsW(Some(&GUID_DEVCLASS_DISPLAY), PCWSTR::null(), None, DIGCF_PRESENT).ok()?;
        let mut found = None;
        for index in 0.. {
            let mut data = SP_DEVINFO_DATA { cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32, ..Default::default() };
            if SetupDiEnumDeviceInfo(set, index, &mut data).is_err() {
                break;
            }
            let mut ids = [0u8; 1024];
            if SetupDiGetDeviceRegistryPropertyW(set, &data, SPDRP_HARDWAREID, None, Some(&mut ids), None).is_err() {
                continue;
            }
            let wide: Vec<u16> = ids.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect();
            if !String::from_utf16_lossy(&wide).to_uppercase().contains("VEN_10DE") {
                continue;
            }
            let mut kind = DEVPROPTYPE::default();
            let mut power = [0u8; 64];
            let awake = if SetupDiGetDevicePropertyW(set, &data, &DEVPKEY_Device_PowerData, &mut kind, Some(&mut power), None, 0).is_ok() {
                // CM_POWER_DATA: PD_Size, then PD_MostRecentPowerState (PowerDeviceD0 = 1).
                i32::from_le_bytes([power[4], power[5], power[6], power[7]]) == 1
            } else {
                true
            };
            found = Some(awake);
            break;
        }
        let _ = SetupDiDestroyDeviceInfoList(set);
        found
    }
}

/// GPU load per process id: the busiest engine of each process (Task
/// Manager's GPU column), from the WDDM counters (no driver call).
fn gpu_by_process() -> HashMap<u32, f64> {
    static PDH: std::sync::Mutex<Option<Pdh>> = std::sync::Mutex::new(None);
    let mut guard = PDH.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_none() {
        *guard = Pdh::new(&[r"\GPU Engine(*)\Utilization Percentage"]);
    }
    let mut per_process = HashMap::new();
    if let Some(pdh) = guard.as_ref() {
        if pdh.collect() {
            for (instance, value) in pdh.array(0) {
                let Some(pid) = instance.strip_prefix("pid_").and_then(|rest| rest.split('_').next()).and_then(|p| p.parse::<u32>().ok()) else {
                    continue;
                };
                let entry = per_process.entry(pid).or_insert(0.0f64);
                *entry = entry.max(value);
            }
        }
    }
    per_process
}

// ------------------------------------------------------------- RAM, net, disks

/// (used GB, total GB, memory load %).
fn memory() -> (f64, f64, f64) {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    unsafe {
        if GlobalMemoryStatusEx(&mut status).is_ok() {
            let total = status.ullTotalPhys as f64 / 1_073_741_824.0;
            let used = (status.ullTotalPhys - status.ullAvailPhys) as f64 / 1_073_741_824.0;
            return (used, total, f64::from(status.dwMemoryLoad));
        }
    }
    (0.0, 0.0, 0.0)
}

struct NetSampler {
    pdh: Option<Pdh>,
}

impl NetSampler {
    fn new() -> Self {
        NetSampler { pdh: Pdh::new(&[r"\Network Interface(*)\Bytes Sent/sec", r"\Network Interface(*)\Bytes Received/sec"]) }
    }

    /// (upload, download) in Mbps across all interfaces.
    fn sample(&mut self) -> (f64, f64) {
        let Some(pdh) = &self.pdh else { return (0.0, 0.0) };
        if !pdh.collect() {
            return (0.0, 0.0);
        }
        let sum = |index| pdh.array(index).values().sum::<f64>() * 8.0 / 1_000_000.0;
        (sum(0), sum(1))
    }
}

fn fixed_drives() -> Vec<Drive> {
    use windows::Win32::Storage::FileSystem::{GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDrives};
    const DRIVE_FIXED: u32 = 3;
    let mut drives = Vec::new();
    let mask = unsafe { GetLogicalDrives() };
    for i in 0..26u32 {
        if mask & (1 << i) == 0 {
            continue;
        }
        let letter = (b'A' + i as u8) as char;
        let root = HSTRING::from(format!("{letter}:\\"));
        if unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) } != DRIVE_FIXED {
            continue;
        }
        let (mut free, mut total) = (0u64, 0u64);
        let ok = unsafe { GetDiskFreeSpaceExW(PCWSTR(root.as_ptr()), None, Some(&mut total), Some(&mut free)).is_ok() };
        if ok && total > 0 {
            drives.push(Drive { name: format!("{letter}:\\"), free_gb: free as f64 / 1_073_741_824.0, total_gb: total as f64 / 1_073_741_824.0 });
        }
    }
    drives
}

// --------------------------------------------------------------- processes

/// The busiest apps, grouped by program and named by their file description
/// (as the original's TOP PROCESSES list shows them).
struct ProcessSampler {
    system: sysinfo::System,
    names: HashMap<std::path::PathBuf, String>,
    last: Vec<Process>,
    own: u32,
}

impl ProcessSampler {
    fn new() -> Self {
        ProcessSampler { system: sysinfo::System::new(), names: HashMap::new(), last: Vec::new(), own: std::process::id() }
    }

    fn sample(&mut self, gpu: &HashMap<u32, f64>) -> Vec<Process> {
        use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, UpdateKind};
        self.system.refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing().with_cpu().with_memory().with_exe(UpdateKind::OnlyIfNotSet),
        );
        let cores = self.system.cpus().len().max(1) as f32;
        let cores = if cores <= 1.0 { std::thread::available_parallelism().map(|n| n.get() as f32).unwrap_or(1.0) } else { cores };
        let mut groups: HashMap<String, (f32, f64, u64, Vec<u32>)> = HashMap::new();
        for (pid, process) in self.system.processes() {
            let pid = pid.as_u32();
            if pid == self.own || pid <= 4 {
                continue;
            }
            let Some(exe) = process.exe() else { continue };
            if exe.starts_with(r"C:\Windows") {
                continue; // system processes, not apps
            }
            let name = self.names.entry(exe.to_path_buf()).or_insert_with(|| describe(exe)).clone();
            let entry = groups.entry(name).or_insert((0.0, 0.0, 0, Vec::new()));
            entry.0 += process.cpu_usage() / cores;
            entry.1 = entry.1.max(gpu.get(&pid).copied().unwrap_or(0.0));
            entry.2 += process.memory();
            entry.3.push(pid);
        }
        let mut rows: Vec<Process> = groups
            .into_iter()
            .map(|(name, (cpu, gpu, memory, pids))| Process { name, cpu: cpu.round() as u32, gpu: gpu.round() as u32, ram_mb: (memory / 1_048_576) as u32, pids })
            .collect();
        rows.sort_by(|a, b| b.cpu.cmp(&a.cpu).then(b.gpu.cmp(&a.gpu)).then(b.ram_mb.cmp(&a.ram_mb)));
        rows.truncate(4);
        self.last = rows.clone();
        rows
    }
}

/// The program's FileDescription (version resource), else its file name.
fn describe(exe: &std::path::Path) -> String {
    use windows::Win32::Storage::FileSystem::{GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW};
    let fallback = || exe.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let path = HSTRING::from(exe.as_os_str());
    unsafe {
        let size = GetFileVersionInfoSizeW(PCWSTR(path.as_ptr()), None);
        if size == 0 {
            return fallback();
        }
        let mut data = vec![0u8; size as usize];
        if GetFileVersionInfoW(PCWSTR(path.as_ptr()), None, size, data.as_mut_ptr().cast()).is_err() {
            return fallback();
        }
        let (mut pointer, mut length) = (std::ptr::null_mut(), 0u32);
        if !VerQueryValueW(data.as_ptr().cast(), windows::core::w!("\\VarFileInfo\\Translation"), &mut pointer, &mut length).as_bool() || length < 4 {
            return fallback();
        }
        let translation = std::slice::from_raw_parts(pointer.cast::<u16>(), 2);
        let key = HSTRING::from(format!("\\StringFileInfo\\{:04x}{:04x}\\FileDescription", translation[0], translation[1]));
        if VerQueryValueW(data.as_ptr().cast(), PCWSTR(key.as_ptr()), &mut pointer, &mut length).as_bool() && length > 1 {
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(pointer.cast::<u16>(), length as usize - 1));
            let text = text.trim().to_owned();
            if !text.is_empty() {
                return text;
            }
        }
        fallback()
    }
}

// --------------------------------------------------------------- system info

pub fn read_system_info() -> SysInfo {
    SysInfo {
        model: wmi_string("Win32_ComputerSystem", "Model").unwrap_or_default(),
        serial: wmi_string("Win32_BIOS", "SerialNumber").unwrap_or_default(),
        os_name: wmi_string("Win32_OperatingSystem", "Caption").unwrap_or_default(),
        os_release: registry_string(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "ReleaseId").unwrap_or_default(),
        os_version: wmi_string("Win32_OperatingSystem", "Version").unwrap_or_default(),
        os_build: wmi_string("Win32_OperatingSystem", "BuildNumber").unwrap_or_default(),
        cpu_name: processor_name(),
        cpu_clock_mhz: wmi_u32("Win32_Processor", "MaxClockSpeed").unwrap_or(0),
        cpu_description: wmi_string("Win32_Processor", "Description").unwrap_or_default(),
        gpus: video_controllers(),
        memory: memory_modules(),
        audio: audio_devices(),
    }
}

fn registry_string(key: &str, value: &str) -> Option<String> {
    use windows::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    let (key, value) = (HSTRING::from(key), HSTRING::from(value));
    let mut buffer = [0u16; 256];
    let mut size = (buffer.len() * 2) as u32;
    unsafe {
        RegGetValueW(HKEY_LOCAL_MACHINE, PCWSTR(key.as_ptr()), PCWSTR(value.as_ptr()), RRF_RT_REG_SZ, None, Some(buffer.as_mut_ptr().cast()), Some(&mut size))
            .ok()
            .ok()?;
    }
    let len = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buffer[..len]).trim().to_owned())
}

fn video_controllers() -> Vec<Gpu> {
    #[derive(serde::Deserialize)]
    #[serde(rename = "Win32_VideoController")]
    struct Controller {
        #[serde(rename = "Name")]
        name: Option<String>,
        #[serde(rename = "DriverVersion")]
        driver_version: Option<String>,
        #[serde(rename = "CurrentRefreshRate")]
        current_refresh_rate: Option<u32>,
    }
    let Ok(wmi) = WMIConnection::new() else {
        return Vec::new();
    };
    let items: Vec<Controller> = wmi.query().unwrap_or_default();
    items
        .into_iter()
        .map(|c| Gpu {
            name: c.name.unwrap_or_default(),
            refresh_hz: c.current_refresh_rate.filter(|r| *r > 0),
            driver: c.driver_version.unwrap_or_default(),
        })
        .collect()
}

fn memory_modules() -> Vec<MemoryModule> {
    #[derive(serde::Deserialize)]
    #[serde(rename = "Win32_PhysicalMemory")]
    struct Module {
        #[serde(rename = "PartNumber")]
        part_number: Option<String>,
        #[serde(rename = "Capacity")]
        capacity: Option<String>,
        #[serde(rename = "Speed")]
        speed: Option<u32>,
        #[serde(rename = "SMBIOSMemoryType")]
        smbios_memory_type: Option<u32>,
    }
    let Ok(wmi) = WMIConnection::new() else {
        return Vec::new();
    };
    let items: Vec<Module> = wmi.query().unwrap_or_default();
    items
        .into_iter()
        .map(|m| MemoryModule {
            part: m.part_number.unwrap_or_default().trim().to_owned(),
            capacity_gb: m.capacity.and_then(|c| c.parse::<u64>().ok()).map(|b| (b / 1_073_741_824) as u32).unwrap_or(0),
            kind: match m.smbios_memory_type {
                Some(34) => "DDR5".to_owned(),
                Some(26) => "DDR4".to_owned(),
                _ => "DDR".to_owned(),
            },
            speed_mhz: m.speed.unwrap_or(0),
        })
        .collect()
}

fn audio_devices() -> Vec<String> {
    #[derive(serde::Deserialize)]
    #[serde(rename = "Win32_SoundDevice")]
    struct Sound {
        #[serde(rename = "Name")]
        name: Option<String>,
    }
    let Ok(wmi) = WMIConnection::new() else {
        return Vec::new();
    };
    let items: Vec<Sound> = wmi.query().unwrap_or_default();
    items.into_iter().filter_map(|s| s.name).collect()
}


pub fn on_ac() -> bool {
    use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    let mut status = SYSTEM_POWER_STATUS::default();
    unsafe {
        if GetSystemPowerStatus(&mut status).is_ok() {
            return status.ACLineStatus == 1;
        }
    }
    true
}

// ----------------------------------------------------------------- WMI helpers

fn wmi_string(class: &str, property: &str) -> Option<String> {
    let wmi = WMIConnection::new().ok()?;
    let rows: Vec<std::collections::HashMap<String, wmi::Variant>> =
        wmi.raw_query(format!("SELECT {property} FROM {class}")).ok()?;
    match rows.into_iter().next()?.remove(property)? {
        wmi::Variant::String(value) => Some(value.trim().to_owned()),
        _ => None,
    }
}

fn wmi_u32(class: &str, property: &str) -> Option<u32> {
    let wmi = WMIConnection::new().ok()?;
    let rows: Vec<std::collections::HashMap<String, wmi::Variant>> =
        wmi.raw_query(format!("SELECT {property} FROM {class}")).ok()?;
    match rows.into_iter().next()?.remove(property)? {
        wmi::Variant::UI4(value) => Some(value),
        wmi::Variant::UI2(value) => Some(u32::from(value)),
        _ => None,
    }
}
