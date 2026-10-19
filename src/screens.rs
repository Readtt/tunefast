//! Every screen the app can show. The ids are the contract with the
//! fast-rebuild tooling: `--screen <id>` opens one, `--smoke` visits all of
//! them, and `projects/tunefast/screens.md` names the same ids.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// Performance modes, fan speed, auto switch to ECO, sensor readouts.
    Performance,
    /// System Vitals, basic view (tiles).
    Vitals,
    /// System Vitals, advanced view (per-core figures, chart).
    VitalsAdvanced,
    /// System Vitals, system information.
    Sysinfo,
}

impl Screen {
    pub const ALL: &'static [Screen] = &[Screen::Performance, Screen::Vitals, Screen::VitalsAdvanced, Screen::Sysinfo];

    pub fn id(self) -> &'static str {
        match self {
            Screen::Performance => "performance",
            Screen::Vitals => "vitals",
            Screen::VitalsAdvanced => "vitals-advanced",
            Screen::Sysinfo => "sysinfo",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Screen::Performance => "Performance Control",
            Screen::Vitals => "System Vitals",
            Screen::VitalsAdvanced => "System Vitals, advanced view",
            Screen::Sysinfo => "System information",
        }
    }

    /// `-light` ids are the same screens in the light theme (snap.py passes --theme light).
    pub fn from_id(id: &str) -> Option<Screen> {
        let id = id.strip_suffix("-light").unwrap_or(id);
        Screen::ALL.iter().copied().find(|screen| screen.id() == id)
    }

    pub fn is_vitals(self) -> bool {
        matches!(self, Screen::Vitals | Screen::VitalsAdvanced | Screen::Sysinfo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip_and_are_unique() {
        for screen in Screen::ALL {
            assert_eq!(Screen::from_id(screen.id()), Some(*screen));
        }
        assert_eq!(Screen::from_id("performance-light"), Some(Screen::Performance));
        let mut ids: Vec<_> = Screen::ALL.iter().map(|s| s.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Screen::ALL.len());
    }
}
