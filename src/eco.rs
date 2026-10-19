//! ECO's Windows-side effects, ported from the original's background
//! (UpdateEcoModeSettings: SetNvidiaMaxFrameRate, SetDisplayRefreshRate):
//! entering ECO saves the internal display's refresh rate and the NVIDIA
//! global "Max Frame Rate", then sets 60 Hz and a 60 fps cap (or the user's
//! own lower cap); leaving ECO puts both back. The firmware part of ECO is the
//! Default policy byte (tunefast_core::bios). ECO's Intel Turbo Boost switch
//! is not reproduced: the original does it through its kernel driver.

use crate::settings::EcoSaved;

/// Applies ECO and returns what it replaced. `previous` is a record left by
/// an earlier ECO session that was never undone (kept, not overwritten).
pub fn enter(previous: Option<EcoSaved>) -> EcoSaved {
    let mut saved = previous.unwrap_or_default();
    if saved.refresh_hz.is_none() {
        if let Some(current) = platform::refresh_rate() {
            if current != tunefast_core::eco::ECO_REFRESH_HZ && platform::set_refresh_rate(tunefast_core::eco::ECO_REFRESH_HZ) {
                saved.refresh_hz = Some(current);
            }
        }
    }
    if saved.frame_cap.is_none() {
        if let Some(current) = platform::frame_cap() {
            let cap = tunefast_core::eco::eco_frame_cap(current.unwrap_or(0));
            if platform::set_frame_cap(Some(cap)) {
                saved.frame_cap = Some(current);
            }
        }
    }
    saved
}

/// Puts back what `enter` changed. Returns the parts that could not be restored.
pub fn leave(saved: &EcoSaved) -> Option<EcoSaved> {
    let mut left = EcoSaved::default();
    if let Some(hz) = saved.refresh_hz {
        // Only if the display is still at ECO's rate: a rate the user picked
        // since then stays (the original checks IsRefreshRate(60) the same way).
        if platform::refresh_rate() == Some(tunefast_core::eco::ECO_REFRESH_HZ) && !platform::set_refresh_rate(hz) {
            left.refresh_hz = Some(hz);
        }
    }
    if let Some(cap) = saved.frame_cap {
        if !platform::set_frame_cap(cap) {
            left.frame_cap = Some(cap);
        }
    }
    (left != EcoSaved::default()).then_some(left)
}

#[cfg(not(windows))]
mod platform {
    pub fn refresh_rate() -> Option<u32> {
        None
    }
    pub fn set_refresh_rate(_hz: u32) -> bool {
        false
    }
    pub fn frame_cap() -> Option<Option<u32>> {
        None
    }
    pub fn set_frame_cap(_cap: Option<u32>) -> bool {
        false
    }
}

#[cfg(windows)]
mod platform {
    use windows::Win32::Devices::Display::{
        DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_MODE_INFO,
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED, DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL,
        DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
        DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS,
        QueryDisplayConfig,
    };
    use windows::Win32::Graphics::Gdi::{
        CDS_UPDATEREGISTRY, ChangeDisplaySettingsExW, DEVMODEW, DISP_CHANGE_SUCCESSFUL, DM_DISPLAYFREQUENCY,
        ENUM_CURRENT_SETTINGS, ENUM_DISPLAY_SETTINGS_MODE, EnumDisplaySettingsW,
    };
    use windows::core::PCWSTR;

    /// The laptop panel's GDI name (`\\.\DISPLAYn`), as the original's
    /// GetInternalDisplayInfo finds it: the active path whose target is internal.
    fn internal_display() -> Option<Vec<u16>> {
        unsafe {
            let (mut paths_len, mut modes_len) = (0u32, 0u32);
            GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut paths_len, &mut modes_len).ok().ok()?;
            let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); paths_len as usize];
            let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); modes_len as usize];
            QueryDisplayConfig(QDC_ONLY_ACTIVE_PATHS, &mut paths_len, paths.as_mut_ptr(), &mut modes_len, modes.as_mut_ptr(), None)
                .ok()
                .ok()?;
            for path in &paths[..paths_len as usize] {
                let mut target = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
                target.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
                target.header.size = std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
                target.header.adapterId = path.targetInfo.adapterId;
                target.header.id = path.targetInfo.id;
                if DisplayConfigGetDeviceInfo(&mut target.header) != 0 {
                    continue;
                }
                let tech = target.outputTechnology;
                if tech != DISPLAYCONFIG_OUTPUT_TECHNOLOGY_INTERNAL
                    && tech != DISPLAYCONFIG_OUTPUT_TECHNOLOGY_DISPLAYPORT_EMBEDDED
                    && tech != DISPLAYCONFIG_OUTPUT_TECHNOLOGY_UDI_EMBEDDED
                {
                    continue;
                }
                let mut source = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
                source.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
                source.header.size = std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
                source.header.adapterId = path.sourceInfo.adapterId;
                source.header.id = path.sourceInfo.id;
                if DisplayConfigGetDeviceInfo(&mut source.header) == 0 {
                    let name = source.viewGdiDeviceName;
                    let end = name.iter().position(|c| *c == 0).unwrap_or(name.len());
                    let mut wide = name[..end].to_vec();
                    wide.push(0);
                    return Some(wide);
                }
            }
            None
        }
    }

    fn current_mode(name: &[u16]) -> Option<DEVMODEW> {
        let mut mode = DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() };
        unsafe { EnumDisplaySettingsW(PCWSTR(name.as_ptr()), ENUM_CURRENT_SETTINGS, &mut mode).as_bool().then_some(mode) }
    }

    pub fn refresh_rate() -> Option<u32> {
        let name = internal_display()?;
        current_mode(&name).map(|mode| mode.dmDisplayFrequency)
    }

    /// Switches the panel to `hz` at its current resolution, if it offers that rate.
    pub fn set_refresh_rate(hz: u32) -> bool {
        let Some(name) = internal_display() else { return false };
        let Some(current) = current_mode(&name) else { return false };
        let offered = (0..).map_while(|i| {
            let mut mode = DEVMODEW { dmSize: std::mem::size_of::<DEVMODEW>() as u16, ..Default::default() };
            unsafe { EnumDisplaySettingsW(PCWSTR(name.as_ptr()), ENUM_DISPLAY_SETTINGS_MODE(i), &mut mode).as_bool().then_some(mode) }
        });
        let fits = offered.into_iter().any(|m| {
            m.dmPelsWidth == current.dmPelsWidth && m.dmPelsHeight == current.dmPelsHeight && m.dmDisplayFrequency == hz
        });
        if !fits {
            return false;
        }
        let mut mode = current;
        mode.dmDisplayFrequency = hz;
        mode.dmFields = DM_DISPLAYFREQUENCY;
        unsafe {
            ChangeDisplaySettingsExW(PCWSTR(name.as_ptr()), Some(&mode), None, CDS_UPDATEREGISTRY, None) == DISP_CHANGE_SUCCESSFUL
        }
    }

    // ------------------------------------------------------- NVIDIA (NVAPI DRS)

    const SETTING: u32 = tunefast_core::eco::NV_MAX_FRAME_RATE_SETTING;
    const SETTING_SIZE: usize = 0x3020; // NVDRS_SETTING_V1
    const NOT_FOUND: i32 = -160; // NVAPI_SETTING_NOT_FOUND

    type QueryInterface = unsafe extern "C" fn(u32) -> *const core::ffi::c_void;

    struct Drs {
        query: QueryInterface,
        session: *mut core::ffi::c_void,
        profile: *mut core::ffi::c_void,
    }

    impl Drs {
        /// Opens the driver settings and its global (base) profile.
        fn open() -> Option<Drs> {
            use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
            unsafe {
                let module = LoadLibraryW(windows::core::w!("nvapi64.dll")).ok()?;
                let query: QueryInterface = std::mem::transmute(GetProcAddress(module, windows::core::s!("nvapi_QueryInterface"))?);
                let mut drs = Drs { query, session: std::ptr::null_mut(), profile: std::ptr::null_mut() };
                let initialize: unsafe extern "C" fn() -> i32 = drs.function(0x0150_E828)?;
                let create: unsafe extern "C" fn(*mut *mut core::ffi::c_void) -> i32 = drs.function(0x0694_D52E)?;
                let load: unsafe extern "C" fn(*mut core::ffi::c_void) -> i32 = drs.function(0x375D_BD6B)?;
                let base: unsafe extern "C" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> i32 = drs.function(0xDA84_66A0)?;
                if initialize() != 0 || create(&mut drs.session) != 0 {
                    return None;
                }
                if load(drs.session) != 0 || base(drs.session, &mut drs.profile) != 0 {
                    return None; // Drop destroys the session
                }
                Some(drs)
            }
        }

        unsafe fn function<F: Copy>(&self, id: u32) -> Option<F> {
            let pointer = unsafe { (self.query)(id) };
            (!pointer.is_null()).then(|| unsafe { std::mem::transmute_copy::<*const core::ffi::c_void, F>(&pointer) })
        }

        /// The cap in fps; None when the global profile does not set it.
        fn get(&self) -> Result<Option<u32>, i32> {
            let mut buffer = vec![0u8; SETTING_SIZE];
            buffer[..4].copy_from_slice(&(SETTING_SIZE as u32 | (1 << 16)).to_le_bytes());
            unsafe {
                let get: unsafe extern "C" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, u32, *mut u8) -> i32 =
                    self.function(0x73BF_8338).ok_or(-1)?;
                match get(self.session, self.profile, SETTING, buffer.as_mut_ptr()) {
                    0 => Ok(Some(u32::from_le_bytes(buffer[8220..8224].try_into().unwrap_or_default()))),
                    NOT_FOUND => Ok(None),
                    error => Err(error),
                }
            }
        }

        fn set(&self, cap: Option<u32>) -> bool {
            unsafe {
                let status = match cap {
                    Some(fps) => {
                        let mut buffer = vec![0u8; SETTING_SIZE];
                        buffer[..4].copy_from_slice(&(SETTING_SIZE as u32 | (1 << 16)).to_le_bytes());
                        buffer[4100..4104].copy_from_slice(&SETTING.to_le_bytes()); // settingId
                        buffer[4104..4108].copy_from_slice(&0u32.to_le_bytes()); // NVDRS_DWORD_TYPE
                        buffer[8220..8224].copy_from_slice(&fps.to_le_bytes()); // u32CurrentValue
                        let Some(set) = self.function::<unsafe extern "C" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut u8) -> i32>(0x577D_D202) else {
                            return false;
                        };
                        set(self.session, self.profile, buffer.as_mut_ptr())
                    }
                    None => {
                        let Some(delete) = self.function::<unsafe extern "C" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, u32) -> i32>(0xE4A2_6362) else {
                            return false;
                        };
                        match delete(self.session, self.profile, SETTING) {
                            NOT_FOUND => return true,
                            status => status,
                        }
                    }
                };
                let Some(save) = self.function::<unsafe extern "C" fn(*mut core::ffi::c_void) -> i32>(0xFCBC_7E14) else {
                    return false;
                };
                status == 0 && save(self.session) == 0
            }
        }
    }

    impl Drop for Drs {
        fn drop(&mut self) {
            unsafe {
                if !self.session.is_null() {
                    if let Some(destroy) = self.function::<unsafe extern "C" fn(*mut core::ffi::c_void) -> i32>(0xDAD9_CFF8) {
                        destroy(self.session);
                    }
                }
            }
        }
    }

    /// None: no NVIDIA driver; Some(None): no cap set; Some(Some(fps)).
    pub fn frame_cap() -> Option<Option<u32>> {
        Drs::open()?.get().ok()
    }

    pub fn set_frame_cap(cap: Option<u32>) -> bool {
        Drs::open().is_some_and(|drs| drs.set(cap))
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::platform;

    /// Read-only: the laptop panel and its refresh rate are found.
    #[test]
    #[ignore = "reads this PC's display; run by hand"]
    fn internal_panel_refresh_rate_is_readable() {
        let hz = platform::refresh_rate().expect("an internal panel");
        assert!(hz >= 60, "{hz} Hz");
    }

    /// Changes the NVIDIA global "Max Frame Rate" to 60 and puts back exactly
    /// what was there (deleting it when it was not set).
    #[test]
    #[ignore = "changes the NVIDIA global frame cap and restores it; run by hand"]
    fn frame_cap_round_trip_restores_the_found_state() {
        let before = platform::frame_cap().expect("an NVIDIA driver");
        assert!(platform::set_frame_cap(Some(60)));
        assert_eq!(platform::frame_cap(), Some(Some(60)));
        assert!(platform::set_frame_cap(before));
        assert_eq!(platform::frame_cap(), Some(before), "restored");
    }
}
