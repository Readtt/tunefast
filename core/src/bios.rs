//! The firmware calls (WMI `root\wmi:hpqBIntM`, signature "SECU") the maker's
//! app makes on this laptop, with the same command, type and bytes.

use serde::{Deserialize, Serialize};

/// The command family used for the gaming features.
pub const CMD_GAMING: u32 = 0x20008;
/// The generic read command (used for the adapter status).
pub const CMD_READ: u32 = 0x01;

/// The performance modes offered on this model, in the original's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    Eco,
    Quiet,
    Default,
    Performance,
}

impl Mode {
    pub const ALL: [Mode; 4] = [Mode::Eco, Mode::Quiet, Mode::Default, Mode::Performance];

    /// The thermal-policy byte the firmware receives. The original keeps ECO
    /// as its own value (256), which truncates to 0 (Default) in the payload,
    /// and maps ECO to Default on thermal policy V0 as well. ECO's other
    /// effects (60 Hz, a frame cap) happen in Windows, not in the firmware.
    pub fn policy_byte(self) -> u8 {
        match self {
            Mode::Eco | Mode::Default => 0,
            Mode::Performance => 1,
            Mode::Quiet => 3,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Mode::Eco => "ECO",
            Mode::Quiet => "Quiet",
            Mode::Default => "Default",
            Mode::Performance => "Performance",
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Mode::Eco => "eco",
            Mode::Quiet => "quiet",
            Mode::Default => "default",
            Mode::Performance => "performance",
        }
    }

    pub fn from_id(id: &str) -> Option<Mode> {
        Mode::ALL.into_iter().find(|m| m.id() == id)
    }
}

/// One firmware call: what `hpqBIOSInt{0,4,128,1024,4096}` receives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BiosCall {
    pub command: u32,
    pub command_type: u32,
    pub input: Vec<u8>,
    /// Bytes expected back: picks the WMI method.
    pub return_size: u32,
}

impl BiosCall {
    /// The WMI method the original picks for a return size.
    pub fn method(&self) -> &'static str {
        match self.return_size {
            0 => "hpqBIOSInt0",
            1..=4 => "hpqBIOSInt4",
            5..=128 => "hpqBIOSInt128",
            129..=1024 => "hpqBIOSInt1024",
            _ => "hpqBIOSInt4096",
        }
    }
}

/// Performance mode: 0x20008 / 0x1A, `[0xFF, mode, 0, 0]` (fan control by BIOS = 0).
pub fn set_mode(mode: Mode) -> BiosCall {
    BiosCall { command: CMD_GAMING, command_type: 0x1A, input: vec![0xFF, mode.policy_byte(), 0, 0], return_size: 4 }
}

/// Max fan: 0x20008 / 0x27, `[0 | 1]`.
pub fn set_max_fan(on: bool) -> BiosCall {
    BiosCall { command: CMD_GAMING, command_type: 0x27, input: vec![u8::from(on)], return_size: 4 }
}

/// Max fan state: 0x20008 / 0x26, four zero bytes; byte 0 of the answer.
pub fn get_max_fan() -> BiosCall {
    BiosCall { command: CMD_GAMING, command_type: 0x26, input: vec![0; 4], return_size: 4 }
}

/// Fan level: 0x20008 / 0x2D, four zero bytes; 128 bytes back.
pub fn get_fan_level() -> BiosCall {
    BiosCall { command: CMD_GAMING, command_type: 0x2D, input: vec![0; 4], return_size: 128 }
}

/// System design data: 0x20008 / 0x28, four zero bytes; byte 3 = thermal policy version.
pub fn get_system_design_data() -> BiosCall {
    BiosCall { command: CMD_GAMING, command_type: 0x28, input: vec![0; 4], return_size: 128 }
}

/// Smart adapter status: command 1, type 0x0F, no input, 4 bytes back.
pub fn get_adapter_status() -> BiosCall {
    BiosCall { command: CMD_READ, command_type: 0x0F, input: Vec::new(), return_size: 4 }
}

/// RPM from the fan-level answer, as the original's monitor reads it: the
/// first byte is the level in hundreds of RPM; 0 and 128 mean stopped; 128+n
/// means -n.
pub fn fan_rpm(answer: &[u8]) -> i32 {
    match answer.first().copied().unwrap_or(0) {
        0 | 128 => 0,
        b if b > 128 => -i32::from(b - 128) * 100,
        b => i32::from(b) * 100,
    }
}

/// Every fan level reported (non-zero bytes up to the first zero), in RPM.
pub fn fan_rpms(answer: &[u8]) -> Vec<u32> {
    answer.iter().take_while(|b| **b != 0).map(|b| u32::from(*b) * 100).collect()
}

/// Max fan on? (byte 0 of the 0x26 answer)
pub fn max_fan_on(answer: &[u8]) -> bool {
    answer.first().copied().unwrap_or(0) == 1
}

/// Thermal policy version from the system design data (byte 3; 0 = V0).
pub fn thermal_policy_version(design_data: &[u8]) -> u8 {
    design_data.get(3).copied().unwrap_or(0)
}

/// The adapter answer as the original reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AdapterStatus {
    /// Byte 0 as reported (2 = below requirement, 0x10 = USB-C).
    pub status: u8,
    /// Byte 3 x 5 W (0 when 0xFF).
    pub rating_watts: u32,
    pub usb_c_design_watts: u32,
    pub barrel_supported: bool,
}

pub fn adapter_status(answer: &[u8]) -> AdapterStatus {
    let byte = |i: usize| answer.get(i).copied().unwrap_or(0);
    AdapterStatus {
        status: byte(0),
        rating_watts: if byte(3) == 0xFF { 0 } else { u32::from(byte(3)) * 5 },
        usb_c_design_watts: u32::from(byte(2)) * 5,
        barrel_supported: byte(1) & 0x80 != 0,
    }
}

impl AdapterStatus {
    /// Low-wattage adapter: a USB-C adapter rated under the design rating (or
    /// no design rating on a barrel model), or an adapter the firmware reports
    /// as below the requirement.
    pub fn low_wattage(&self) -> bool {
        const CONNECTED_TYPE_C: u8 = 0x10;
        const BELOW_REQUIREMENT: u8 = 2;
        if self.status == CONNECTED_TYPE_C {
            (self.rating_watts > 0 && self.rating_watts < self.usb_c_design_watts)
                || (self.barrel_supported && self.usb_c_design_watts == 0)
        } else {
            self.status == BELOW_REQUIREMENT
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_payloads_match_the_original() {
        assert_eq!(set_mode(Mode::Default).input, vec![0xFF, 0, 0, 0]);
        assert_eq!(set_mode(Mode::Performance).input, vec![0xFF, 1, 0, 0]);
        assert_eq!(set_mode(Mode::Quiet).input, vec![0xFF, 3, 0, 0]);
        assert_eq!(set_mode(Mode::Eco).input, vec![0xFF, 0, 0, 0], "ECO reaches the firmware as Default");
        let call = set_mode(Mode::Performance);
        assert_eq!((call.command, call.command_type, call.return_size), (0x20008, 0x1A, 4));
        assert_eq!(call.method(), "hpqBIOSInt4");
    }

    #[test]
    fn fan_payloads_match_the_original() {
        assert_eq!(set_max_fan(true), BiosCall { command: 0x20008, command_type: 0x27, input: vec![1], return_size: 4 });
        assert_eq!(set_max_fan(false).input, vec![0]);
        assert_eq!(get_max_fan(), BiosCall { command: 0x20008, command_type: 0x26, input: vec![0; 4], return_size: 4 });
        let level = get_fan_level();
        assert_eq!((level.command_type, level.input.clone(), level.return_size), (0x2D, vec![0; 4], 128));
        assert_eq!(level.method(), "hpqBIOSInt128");
    }

    #[test]
    fn reads_match_the_original() {
        let sdd = get_system_design_data();
        assert_eq!((sdd.command, sdd.command_type, sdd.return_size), (0x20008, 0x28, 128));
        let adapter = get_adapter_status();
        assert_eq!((adapter.command, adapter.command_type, adapter.input.len(), adapter.return_size), (1, 0x0F, 0, 4));
        assert_eq!(adapter.method(), "hpqBIOSInt4");
    }

    #[test]
    fn fan_level_decodes_like_the_original_monitor() {
        assert_eq!(fan_rpm(&[0, 0]), 0);
        assert_eq!(fan_rpm(&[128]), 0);
        assert_eq!(fan_rpm(&[33, 35]), 3300);
        assert_eq!(fan_rpm(&[130]), -200);
        assert_eq!(fan_rpm(&[]), 0);
        assert_eq!(fan_rpms(&[33, 35, 0, 7]), vec![3300, 3500]);
    }

    #[test]
    fn design_data_and_adapter_decode() {
        // The original's cached design data on this laptop starts E6 00 35 00: policy V0.
        assert_eq!(thermal_policy_version(&[0xE6, 0x00, 0x35, 0x00]), 0);
        let a = adapter_status(&[1, 0, 0, 46]);
        assert_eq!(a.rating_watts, 230);
        assert!(!a.low_wattage());
        assert!(adapter_status(&[2, 0, 0, 30]).low_wattage());
        assert!(adapter_status(&[0x10, 0, 20, 13]).low_wattage(), "65 W USB-C under a 100 W design");
        assert_eq!(adapter_status(&[1, 0, 0, 0xFF]).rating_watts, 0);
    }

    #[test]
    fn mode_ids_round_trip() {
        for mode in Mode::ALL {
            assert_eq!(Mode::from_id(mode.id()), Some(mode));
        }
    }
}
