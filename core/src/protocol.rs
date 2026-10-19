//! The window <-> helper protocol: one JSON object per line over the named
//! pipe [`PIPE_NAME`]. A request names an operation; the helper maps it to one
//! fixed [`BiosCall`]. Nothing else can be expressed.

use serde::{Deserialize, Serialize};

use crate::bios::{self, BiosCall, Mode};

pub const PIPE_NAME: &str = r"\\.\pipe\tunefast-helper";
pub const SERVICE_NAME: &str = "TunefastHelper";
pub const SERVICE_DISPLAY: &str = "Tunefast hardware helper";
pub const PROTOCOL_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Hello,
    SetMode { mode: Mode },
    SetMaxFan { on: bool },
    GetMaxFan,
    GetFanLevel,
    GetAdapterStatus,
    GetSystemDesignData,
}

impl Request {
    /// The one firmware call this request stands for (none for `Hello`).
    pub fn bios_call(&self) -> Option<BiosCall> {
        match self {
            Request::Hello => None,
            Request::SetMode { mode } => Some(bios::set_mode(*mode)),
            Request::SetMaxFan { on } => Some(bios::set_max_fan(*on)),
            Request::GetMaxFan => Some(bios::get_max_fan()),
            Request::GetFanLevel => Some(bios::get_fan_level()),
            Request::GetAdapterStatus => Some(bios::get_adapter_status()),
            Request::GetSystemDesignData => Some(bios::get_system_design_data()),
        }
    }

    pub fn encode(&self) -> String {
        let mut line = serde_json::to_string(self).unwrap_or_default();
        line.push('\n');
        line
    }

    pub fn decode(line: &str) -> Result<Request, String> {
        serde_json::from_str(line.trim()).map_err(|e| format!("bad request: {e}"))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    /// The firmware's return code (0 = success) when a call was made.
    #[serde(default)]
    pub return_code: Option<u32>,
    #[serde(default)]
    pub data: Vec<u8>,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub version: u32,
}

impl Response {
    pub fn encode(&self) -> String {
        let mut line = serde_json::to_string(self).unwrap_or_default();
        line.push('\n');
        line
    }

    pub fn decode(line: &str) -> Result<Response, String> {
        serde_json::from_str(line.trim()).map_err(|e| format!("bad response: {e}"))
    }

    pub fn failure(error: impl Into<String>) -> Response {
        Response { ok: false, error: Some(error.into()), version: PROTOCOL_VERSION, ..Response::default() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_round_trip_and_map_to_fixed_calls() {
        for request in [
            Request::Hello,
            Request::SetMode { mode: Mode::Quiet },
            Request::SetMaxFan { on: true },
            Request::GetMaxFan,
            Request::GetFanLevel,
            Request::GetAdapterStatus,
            Request::GetSystemDesignData,
        ] {
            assert_eq!(Request::decode(&request.encode()).unwrap(), request);
        }
        assert_eq!(Request::SetMode { mode: Mode::Quiet }.bios_call().unwrap().input, vec![0xFF, 3, 0, 0]);
        assert!(Request::Hello.bios_call().is_none());
    }

    #[test]
    fn anything_outside_the_list_is_rejected() {
        assert!(Request::decode(r#"{"op":"raw","command":131080,"command_type":46}"#).is_err());
        assert!(Request::decode(r#"{"op":"set_mode","mode":"Turbo"}"#).is_err());
        assert!(Request::decode("not json").is_err());
    }
}
