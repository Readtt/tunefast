//! Every icon the interface draws, embedded as white-stroked SVG (Lucide,
//! ISC licence in assets/icons) and tinted at draw time. None of the
//! original's own icon files are used.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Settings,
    Advanced,
    Close,
    Minimize,
    Maximize,
    Restore,
    ChevronDown,
    ChevronUp,
    Info,
    Leaf,
    Fan,
    Gauge,
    Bars,
    Warning,
    Laptop,
    Help,
    Stop,
    Drive,
    Menu,
    Cog,
    App,
    Layers,
}

impl Icon {
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Icon::Settings => include_bytes!("../assets/icons/settings.svg"),
            Icon::Advanced => include_bytes!("../assets/icons/settings-2.svg"),
            Icon::Close => include_bytes!("../assets/icons/x.svg"),
            Icon::Minimize => include_bytes!("../assets/icons/minus.svg"),
            Icon::Maximize => include_bytes!("../assets/icons/square.svg"),
            Icon::Restore => include_bytes!("../assets/icons/copy.svg"),
            Icon::ChevronDown => include_bytes!("../assets/icons/chevron-down.svg"),
            Icon::ChevronUp => include_bytes!("../assets/icons/chevron-up.svg"),
            Icon::Info => include_bytes!("../assets/icons/info.svg"),
            Icon::Leaf => include_bytes!("../assets/icons/leaf.svg"),
            Icon::Fan => include_bytes!("../assets/icons/fan.svg"),
            Icon::Gauge => include_bytes!("../assets/icons/gauge.svg"),
            Icon::Bars => include_bytes!("../assets/icons/chart-column.svg"),
            Icon::Warning => include_bytes!("../assets/icons/triangle-alert.svg"),
            Icon::Laptop => include_bytes!("../assets/icons/laptop.svg"),
            Icon::Help => include_bytes!("../assets/icons/circle-help.svg"),
            Icon::Stop => include_bytes!("../assets/icons/circle-slash.svg"),
            Icon::Drive => include_bytes!("../assets/icons/hard-drive.svg"),
            Icon::Menu => include_bytes!("../assets/icons/menu.svg"),
            Icon::Cog => include_bytes!("../assets/icons/cog.svg"),
            Icon::App => include_bytes!("../assets/icons/app-window.svg"),
            Icon::Layers => include_bytes!("../assets/icons/layers-2.svg"),
        }
    }
}
