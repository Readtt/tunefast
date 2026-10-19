//! The Windows service: a named pipe that accepts `Request` lines, the
//! firmware calls behind them, and re-applying the user's choice after
//! power events (the same calls the maker's background process makes then).

#![allow(non_snake_case)]

use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tunefast_core::bios::{BiosCall, Mode};
use tunefast_core::protocol::{PIPE_NAME, PROTOCOL_VERSION, Request, Response, SERVICE_DISPLAY, SERVICE_NAME};
use windows::Win32::Foundation::{CloseHandle, HANDLE, HLOCAL, INVALID_HANDLE_VALUE, LocalFree};
use windows::Win32::Security::Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1};
use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
use windows::Win32::Storage::FileSystem::{FlushFileBuffers, PIPE_ACCESS_DUPLEX, ReadFile, WriteFile};
use windows::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES,
    PIPE_WAIT,
};
use windows::core::{HSTRING, w};
use windows_service::service::{
    ServiceAccess, ServiceControl, ServiceControlAccept, ServiceErrorControl, ServiceExitCode, ServiceInfo,
    ServiceStartType, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_manager::{ServiceManager, ServiceManagerAccess};
use windows_service::{define_windows_service, service_dispatcher};

pub fn main() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("install") => report(install()),
        Some("uninstall") => report(uninstall()),
        Some("serve") => {
            serve_forever();
            0
        }
        Some("selftest") => selftest(),
        Some(other) => {
            eprintln!("unknown command {other:?}; use install, uninstall, selftest or serve");
            2
        }
        None => match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
            Ok(()) => 0,
            Err(error) => {
                eprintln!("not started by the service manager ({error}); use install, uninstall, selftest or serve");
                2
            }
        },
    }
}

fn report(result: Result<String, String>) -> i32 {
    match result {
        Ok(message) => {
            println!("{message}");
            0
        }
        Err(error) => {
            eprintln!("{error}");
            1
        }
    }
}

// ------------------------------------------------------------------ firmware

#[derive(Serialize)]
#[serde(rename = "hpqBDataIn")]
struct BDataIn {
    Sign: Vec<u8>,
    Command: u32,
    CommandType: u32,
    Size: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    hpqBData: Option<Vec<u8>>,
}

#[derive(Serialize)]
struct MethodIn {
    InData: BDataIn,
}

#[derive(Deserialize)]
#[serde(rename = "hpqBIntM")]
struct BIntM {}

#[derive(Deserialize)]
#[serde(rename = "hpqBIntM")]
struct BIntMInstance {
    InstanceName: String,
}

#[derive(Deserialize)]
struct MethodOut {
    OutData: BDataOut,
}

#[derive(Deserialize)]
struct BDataOut {
    #[serde(default)]
    rwReturnCode: Option<u32>,
    #[serde(default)]
    Data: Option<Vec<u8>>,
}

/// One call into `hpqBIntM`, exactly as described by the `BiosCall`.
fn call_firmware(call: &BiosCall) -> Result<(u32, Vec<u8>), String> {
    let wmi = wmi::WMIConnection::with_namespace_path("ROOT\\WMI").map_err(|e| format!("WMI: {e}"))?;
    let instances: Vec<BIntMInstance> =
        wmi.raw_query("SELECT InstanceName FROM hpqBIntM").map_err(|e| format!("hpqBIntM: {e}"))?;
    let instance = instances.first().ok_or("hpqBIntM: no instance (not an HP BIOS?)")?;
    let path = format!("hpqBIntM.InstanceName=\"{}\"", instance.InstanceName.replace('\\', "\\\\"));
    let input = MethodIn {
        InData: BDataIn {
            Sign: b"SECU".to_vec(),
            Command: call.command,
            CommandType: call.command_type,
            Size: call.input.len() as u32,
            hpqBData: (!call.input.is_empty()).then(|| call.input.clone()),
        },
    };
    let out: MethodOut =
        wmi.exec_instance_method::<BIntM, MethodOut>(&path, call.method(), input).map_err(|e| format!("{}: {e}", call.method()))?;
    let mut data = out.OutData.Data.unwrap_or_default();
    data.truncate(call.return_size as usize);
    Ok((out.OutData.rwReturnCode.unwrap_or(0), data))
}

fn handle(request: &Request) -> Response {
    let Some(call) = request.bios_call() else {
        return Response { ok: true, version: PROTOCOL_VERSION, ..Response::default() };
    };
    match call_firmware(&call) {
        Ok((code, data)) => {
            if code == 0 {
                remember(request);
            }
            Response { ok: code == 0, return_code: Some(code), data, error: None, version: PROTOCOL_VERSION }
        }
        Err(error) => Response::failure(error),
    }
}

// ------------------------------------------------- the user's choice, re-applied

#[derive(Default, Serialize, Deserialize)]
struct Choice {
    mode: Option<Mode>,
    max_fan: Option<bool>,
}

fn choice_path() -> PathBuf {
    let base = std::env::var_os("ProgramData").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    base.join("Tunefast").join("helper-choice.json")
}

fn load_choice() -> Choice {
    std::fs::read_to_string(choice_path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn remember(request: &Request) {
    let mut choice = load_choice();
    match request {
        Request::SetMode { mode } => choice.mode = Some(*mode),
        Request::SetMaxFan { on } => choice.max_fan = Some(*on),
        _ => return,
    }
    let path = choice_path();
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Ok(text) = serde_json::to_string(&choice) {
        let tmp = path.with_extension("tmp");
        if std::fs::write(&tmp, text).is_ok() {
            let _ = std::fs::rename(tmp, path);
        }
    }
}

/// After resume or a power-source change the maker's background process
/// re-applies its stored mode and max fan; re-apply the user's Tunefast choice
/// after it (only what the user chose in Tunefast).
fn reapply_choice() {
    std::thread::sleep(Duration::from_secs(4));
    let choice = load_choice();
    if let Some(mode) = choice.mode {
        let _ = call_firmware(&tunefast_core::bios::set_mode(mode));
    }
    if let Some(on) = choice.max_fan {
        let _ = call_firmware(&tunefast_core::bios::set_max_fan(on));
    }
}

// -------------------------------------------------------------------- the pipe

/// SYSTEM and administrators: full; interactive users: read/write (connect and talk).
const PIPE_SDDL: windows::core::PCWSTR = w!("D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;IU)");

fn serve_forever() {
    loop {
        if let Err(error) = serve_one() {
            eprintln!("pipe: {error}");
            std::thread::sleep(Duration::from_millis(500));
        }
    }
}

fn serve_one() -> Result<(), String> {
    unsafe {
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        ConvertStringSecurityDescriptorToSecurityDescriptorW(PIPE_SDDL, SDDL_REVISION_1, &mut descriptor, None)
            .map_err(|e| format!("security descriptor: {e}"))?;
        let attributes = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: descriptor.0,
            bInheritHandle: false.into(),
        };
        let pipe = CreateNamedPipeW(
            &HSTRING::from(PIPE_NAME),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            4096,
            4096,
            0,
            Some(&attributes),
        );
        let _ = LocalFree(Some(HLOCAL(descriptor.0)));
        if pipe == INVALID_HANDLE_VALUE {
            return Err("CreateNamedPipe failed".to_owned());
        }
        let connected = ConnectNamedPipe(pipe, None);
        if connected.is_ok() || windows::Win32::Foundation::GetLastError() == windows::Win32::Foundation::ERROR_PIPE_CONNECTED {
            // One short conversation per connection, on its own thread.
            let raw = pipe.0 as usize;
            std::thread::spawn(move || {
                let pipe = HANDLE(raw as *mut core::ffi::c_void);
                converse(pipe);
                let _ = FlushFileBuffers(pipe);
                let _ = DisconnectNamedPipe(pipe);
                let _ = CloseHandle(pipe);
            });
        } else {
            let _ = CloseHandle(pipe);
        }
    }
    Ok(())
}

unsafe fn converse(pipe: HANDLE) {
    let mut line = Vec::new();
    let mut buffer = [0u8; 1024];
    loop {
        let mut read = 0u32;
        let ok = unsafe { ReadFile(pipe, Some(&mut buffer), Some(&mut read), None) };
        if ok.is_err() || read == 0 {
            return;
        }
        line.extend_from_slice(&buffer[..read as usize]);
        if line.len() > 8192 {
            return;
        }
        while let Some(end) = line.iter().position(|b| *b == b'\n') {
            let text = String::from_utf8_lossy(&line[..end]).into_owned();
            line.drain(..=end);
            let response = match Request::decode(&text) {
                Ok(request) => handle(&request),
                Err(error) => Response::failure(error),
            };
            let bytes = response.encode().into_bytes();
            let mut written = 0u32;
            if unsafe { WriteFile(pipe, Some(&bytes), Some(&mut written), None) }.is_err() {
                return;
            }
        }
    }
}

// ----------------------------------------------------------------- the service

define_windows_service!(ffi_service_main, service_main);

fn service_main(_arguments: Vec<OsString>) {
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let handler = move |control| match control {
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = stop_tx.send(());
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::PowerEvent(_) => {
            std::thread::spawn(reapply_choice);
            ServiceControlHandlerResult::NoError
        }
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        _ => ServiceControlHandlerResult::NotImplemented,
    };
    let Ok(status) = service_control_handler::register(SERVICE_NAME, handler) else {
        return;
    };
    let running = ServiceStatus {
        service_type: ServiceType::OWN_PROCESS,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN | ServiceControlAccept::POWER_EVENT,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    };
    let _ = status.set_service_status(running.clone());
    std::thread::spawn(serve_forever);
    let _ = stop_rx.recv();
    let _ = status.set_service_status(ServiceStatus {
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        ..running
    });
}

fn install() -> Result<String, String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE)
        .map_err(|e| format!("needs an elevated prompt: {e}"))?;
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let info = ServiceInfo {
        name: OsString::from(SERVICE_NAME),
        display_name: OsString::from(SERVICE_DISPLAY),
        service_type: ServiceType::OWN_PROCESS,
        start_type: ServiceStartType::AutoStart,
        error_control: ServiceErrorControl::Normal,
        executable_path: exe.clone(),
        launch_arguments: vec![],
        dependencies: vec![],
        account_name: None,
        account_password: None,
    };
    let service = match manager.create_service(&info, ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS) {
        Ok(service) => service,
        Err(_) => manager
            .open_service(SERVICE_NAME, ServiceAccess::CHANGE_CONFIG | ServiceAccess::START | ServiceAccess::QUERY_STATUS)
            .map_err(|e| format!("open service: {e}"))?,
    };
    let _ = service.change_config(&info);
    let _ = service.set_description("Makes the few laptop firmware calls Tunefast's window asks for (performance mode, max fan, fan speed).");
    if service.query_status().map(|s| s.current_state != ServiceState::Running).unwrap_or(true) {
        service.start::<&str>(&[]).map_err(|e| format!("start: {e}"))?;
    }
    Ok(format!("INSTALLED {SERVICE_NAME} -> {}", exe.display()))
}

fn uninstall() -> Result<String, String> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|e| format!("needs an elevated prompt: {e}"))?;
    let service = manager
        .open_service(SERVICE_NAME, ServiceAccess::STOP | ServiceAccess::DELETE | ServiceAccess::QUERY_STATUS)
        .map_err(|e| format!("open service: {e}"))?;
    let _ = service.stop();
    service.delete().map_err(|e| format!("delete: {e}"))?;
    Ok(format!("REMOVED {SERVICE_NAME}"))
}

/// Elevated, read-only: the reads the window will use, nothing written.
fn selftest() -> i32 {
    let mut failures = 0;
    for (name, request) in [
        ("system design data", Request::GetSystemDesignData),
        ("max fan", Request::GetMaxFan),
        ("fan level", Request::GetFanLevel),
        ("adapter", Request::GetAdapterStatus),
    ] {
        let response = handle(&request);
        if response.ok {
            println!("ok   {name}: code {:?} data {:02X?}", response.return_code, &response.data[..response.data.len().min(8)]);
        } else {
            failures += 1;
            println!("FAIL {name}: code {:?} {}", response.return_code, response.error.unwrap_or_default());
        }
    }
    println!("SELFTEST {}", if failures == 0 { "OK" } else { "FAIL" });
    i32::from(failures > 0)
}
