//! ECO mode and "Auto Switch to Eco Mode", ported from the original's power control.

/// Switch to ECO automatically: enabled, on battery or on a low-wattage
/// adapter, and not overridden by a mode the user picked while it was active.
pub fn should_auto_switch(enabled: bool, on_ac: bool, low_wattage: bool, overridden: bool) -> bool {
    enabled && (!on_ac || low_wattage) && !overridden
}

/// ECO's frame cap: 60, or the user's own lower cap if one is set (0 = off).
pub fn eco_frame_cap(current_cap: u32) -> u32 {
    if current_cap != 0 && current_cap < 60 { current_cap } else { 60 }
}

/// ECO's display refresh rate.
pub const ECO_REFRESH_HZ: u32 = 60;

/// The NVIDIA driver setting the original writes for its frame cap
/// (global profile "Max Frame Rate", DWORD fps, 0 = off).
pub const NV_MAX_FRAME_RATE_SETTING: u32 = 0x1083_5002;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_switch_follows_the_original_rule() {
        assert!(should_auto_switch(true, false, false, false), "unplugged");
        assert!(should_auto_switch(true, true, true, false), "weak adapter");
        assert!(!should_auto_switch(true, true, false, false), "full adapter");
        assert!(!should_auto_switch(false, false, false, false), "off");
        assert!(!should_auto_switch(true, false, false, true), "overridden");
    }

    #[test]
    fn eco_cap_keeps_a_lower_user_cap() {
        assert_eq!(eco_frame_cap(0), 60);
        assert_eq!(eco_frame_cap(144), 60);
        assert_eq!(eco_frame_cap(45), 45);
    }
}
