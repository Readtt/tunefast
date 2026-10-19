//! Every icon the interface draws, embedded as white-stroked SVG (Lucide,
//! ISC) and tinted at draw time. Fetch more with
//! `uv run tools/get_icons.py <app>/assets/icons <name>...` and add a variant.

use iced::widget::svg;
use iced::{Color, Element, Length};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // The full set rebuilds draw from.
pub enum Icon {
    Search,
    Settings,
    Plus,
    Close,
    ChevronDown,
    Users,
    Moon,
    Sun,
    Monitor,
    More,
    Info,
    Check,
    Minimize,
    Maximize,
    Restore,
    Inbox,
    Hash,
    Bell,
    Star,
    Archive,
    Send,
    Smile,
    Paperclip,
    Phone,
    Video,
    Chat,
}

impl Icon {
    pub fn bytes(self) -> &'static [u8] {
        match self {
            Icon::Search => include_bytes!("../assets/icons/search.svg"),
            Icon::Settings => include_bytes!("../assets/icons/settings.svg"),
            Icon::Plus => include_bytes!("../assets/icons/plus.svg"),
            Icon::Close => include_bytes!("../assets/icons/x.svg"),
            Icon::ChevronDown => include_bytes!("../assets/icons/chevron-down.svg"),
            Icon::Users => include_bytes!("../assets/icons/users.svg"),
            Icon::Moon => include_bytes!("../assets/icons/moon.svg"),
            Icon::Sun => include_bytes!("../assets/icons/sun.svg"),
            Icon::Monitor => include_bytes!("../assets/icons/monitor.svg"),
            Icon::More => include_bytes!("../assets/icons/ellipsis.svg"),
            Icon::Info => include_bytes!("../assets/icons/info.svg"),
            Icon::Check => include_bytes!("../assets/icons/check.svg"),
            Icon::Minimize => include_bytes!("../assets/icons/minus.svg"),
            Icon::Maximize => include_bytes!("../assets/icons/square.svg"),
            Icon::Restore => include_bytes!("../assets/icons/copy.svg"),
            Icon::Inbox => include_bytes!("../assets/icons/inbox.svg"),
            Icon::Hash => include_bytes!("../assets/icons/hash.svg"),
            Icon::Bell => include_bytes!("../assets/icons/bell.svg"),
            Icon::Star => include_bytes!("../assets/icons/star.svg"),
            Icon::Archive => include_bytes!("../assets/icons/archive.svg"),
            Icon::Send => include_bytes!("../assets/icons/send.svg"),
            Icon::Smile => include_bytes!("../assets/icons/smile.svg"),
            Icon::Paperclip => include_bytes!("../assets/icons/paperclip.svg"),
            Icon::Phone => include_bytes!("../assets/icons/phone.svg"),
            Icon::Video => include_bytes!("../assets/icons/video.svg"),
            Icon::Chat => include_bytes!("../assets/icons/message-circle.svg"),
        }
    }

    /// The icon at `size` points, tinted `color`.
    pub fn view<'a, Message: 'a>(self, size: f32, color: Color) -> Element<'a, Message> {
        svg(svg::Handle::from_memory(self.bytes()))
            .width(Length::Fixed(size))
            .height(Length::Fixed(size))
            .style(move |_, _| svg::Style { color: Some(color) })
            .into()
    }
}
