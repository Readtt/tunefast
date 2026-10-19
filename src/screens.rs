//! Every screen the app can show. The ids are the contract with the
//! fast-rebuild tooling: `--screen <id>` opens one, `--smoke` visits all of
//! them, and `projects/<slug>/screens.md` names the same ids for the visual
//! diff loop. Add a variant here for every core screen of the original app.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    /// The main three-pane view with a conversation open.
    Home,
    /// The main view with nothing selected.
    Empty,
    /// Preferences.
    Settings,
}

impl Screen {
    pub const ALL: &'static [Screen] = &[Screen::Home, Screen::Empty, Screen::Settings];

    pub fn id(self) -> &'static str {
        match self {
            Screen::Home => "home",
            Screen::Empty => "empty",
            Screen::Settings => "settings",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Screen::Home => "Conversation open",
            Screen::Empty => "Nothing selected",
            Screen::Settings => "Settings",
        }
    }

    pub fn from_id(id: &str) -> Option<Screen> {
        Screen::ALL.iter().copied().find(|screen| screen.id() == id)
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
        let mut ids: Vec<_> = Screen::ALL.iter().map(|s| s.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Screen::ALL.len());
    }
}
