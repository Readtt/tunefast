//! Design tokens and widget styles: every colour, size, radius and font the
//! interface uses. Views never write a literal colour or pixel value. For a
//! rebuild, replace the values with the original's tokens
//! (`projects/<slug>/design/tokens.json` via `tools/tokens_to_rust.py`).

use iced::border::{self, Border};
use iced::font::{Family, Weight};
use iced::widget::{button, container, scrollable, text_input};
use iced::{Background, Color, Font, Shadow, Theme, Vector};

/// `0xRRGGBB` to a colour, usable in `const` items.
pub const fn hex(rgb: u32) -> Color {
    Color::from_rgb8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8)
}

/// `0xRRGGBB` plus alpha (0.0-1.0), as CSS `rgba()` writes it.
#[allow(dead_code)]
pub const fn hexa(rgb: u32, alpha: f32) -> Color {
    Color::from_rgba8((rgb >> 16) as u8, (rgb >> 8) as u8, rgb as u8, alpha)
}

/// The colours of one theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub window: Color,
    pub titlebar: Color,
    pub sidebar: Color,
    pub list: Color,
    pub surface: Color,
    pub hover: Color,
    pub selected: Color,
    pub border: Color,
    pub text: Color,
    pub text_muted: Color,
    pub text_faint: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub positive: Color,
    pub danger: Color,
}

impl Palette {
    pub const DARK: Palette = Palette {
        window: hex(0x16171B),
        titlebar: hex(0x111215),
        sidebar: hex(0x111215),
        list: hex(0x1B1C21),
        surface: hex(0x24262C),
        hover: hex(0x23252B),
        selected: hex(0x2C2F37),
        border: hex(0x2A2C33),
        text: hex(0xECEDEF),
        text_muted: hex(0x9A9DA6),
        text_faint: hex(0x666974),
        accent: hex(0x6E7BFF),
        on_accent: hex(0xFFFFFF),
        positive: hex(0x3DD68C),
        danger: hex(0xF2555A),
    };

    pub const LIGHT: Palette = Palette {
        window: hex(0xFFFFFF),
        titlebar: hex(0xF3F4F6),
        sidebar: hex(0xF3F4F6),
        list: hex(0xF9FAFB),
        surface: hex(0xFFFFFF),
        hover: hex(0xECEEF2),
        selected: hex(0xE3E6FF),
        border: hex(0xE2E4E9),
        text: hex(0x17181C),
        text_muted: hex(0x5F6370),
        text_faint: hex(0x9A9DA6),
        accent: hex(0x4F5DFF),
        on_accent: hex(0xFFFFFF),
        positive: hex(0x16A36A),
        danger: hex(0xD93036),
    };
}

/// Fixed sizes in logical points (CSS px at zoom 1).
pub mod size {
    pub const DEFAULT_WINDOW: [f32; 2] = [1280.0, 800.0];
    pub const MIN_WINDOW: [f32; 2] = [760.0, 480.0];
    pub const TITLEBAR: f32 = 40.0;
    pub const SIDEBAR: f32 = 232.0;
    pub const LIST: f32 = 320.0;
    pub const NAV_ROW: f32 = 34.0;
    pub const LIST_ROW: f32 = 68.0;
    pub const AVATAR: f32 = 40.0;
    pub const AVATAR_SMALL: f32 = 32.0;
    pub const ICON: f32 = 16.0;
    pub const ICON_BUTTON: f32 = 30.0;
    pub const INPUT: f32 = 36.0;
    pub const COMPOSER: f32 = 48.0;
    pub const HEADER: f32 = 56.0;
    pub const CAPTION_BUTTON: f32 = 46.0;
}

/// Corner radii.
pub mod radius {
    pub const SM: f32 = 4.0;
    pub const MD: f32 = 8.0;
    pub const LG: f32 = 12.0;
    pub const PILL: f32 = 999.0;
}

/// Spacing steps.
#[allow(dead_code)]
pub mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

/// The type scale. The bundled Inter (OFL) is registered in main.rs; swap it
/// for the original's closest open match (tools/font_match.py).
pub mod font {
    use super::*;

    pub const REGULAR: Font = Font::with_name("Inter");
    pub const MEDIUM: Font = Font { family: Family::Name("Inter"), weight: Weight::Medium, ..Font::DEFAULT };
    pub const SEMIBOLD: Font = Font { family: Family::Name("Inter"), weight: Weight::Semibold, ..Font::DEFAULT };
    pub const BOLD: Font = Font { family: Family::Name("Inter"), weight: Weight::Bold, ..Font::DEFAULT };

    pub const CAPTION: f32 = 12.0;
    pub const BODY: f32 = 14.0;
    pub const LABEL: f32 = 13.0;
    pub const SECTION: f32 = 11.0;
    pub const TITLE: f32 = 16.0;
    pub const HEADING: f32 = 22.0;
}

pub fn corners(radius: f32) -> border::Radius {
    border::Radius::from(radius)
}

/// A filled block: panel backgrounds, cards.
pub fn fill(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(color)),
        ..container::Style::default()
    }
}

/// A filled block with rounded corners and an optional hairline.
pub fn card(color: Color, radius: f32, border_color: Option<Color>) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(color)),
        border: Border {
            radius: corners(radius),
            width: if border_color.is_some() { 1.0 } else { 0.0 },
            color: border_color.unwrap_or(Color::TRANSPARENT),
        },
        ..container::Style::default()
    }
}

/// A popover or menu surface with the original's shadow.
#[allow(dead_code)]
pub fn popover(palette: Palette) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(palette.surface)),
        border: Border { radius: corners(radius::MD), width: 1.0, color: palette.border },
        shadow: Shadow { color: Color::from_rgba8(0, 0, 0, 0.35), offset: Vector::new(0.0, 8.0), blur_radius: 24.0 },
        ..container::Style::default()
    }
}

/// A transparent button that fills on hover and when selected: list rows,
/// navigation rows, icon buttons.
pub fn row_button(palette: Palette, selected: bool, radius: f32) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let background = match (selected, status) {
            (true, _) => Some(palette.selected),
            (false, button::Status::Hovered | button::Status::Pressed) => Some(palette.hover),
            _ => None,
        };
        button::Style {
            background: background.map(Background::Color),
            text_color: palette.text,
            border: Border { radius: corners(radius), ..Border::default() },
            ..button::Style::default()
        }
    }
}

/// The background of a `press_row`: filled when selected or hovered.
pub fn row_fill(palette: Palette, selected: bool, hovered: bool, radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: if selected {
            Some(Background::Color(palette.selected))
        } else if hovered {
            Some(Background::Color(palette.hover))
        } else {
            None
        },
        text_color: Some(palette.text),
        border: Border { radius: corners(radius), ..Border::default() },
        ..container::Style::default()
    }
}

/// Window caption buttons (minimize, maximize, close).
pub fn caption_button(palette: Palette, danger: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: hovered.then(|| Background::Color(if danger { hex(0xE81123) } else { palette.hover })),
            text_color: palette.text_muted,
            ..button::Style::default()
        }
    }
}

/// A frameless text input: the surrounding container draws the field.
pub fn bare_input(palette: Palette) -> impl Fn(&Theme, text_input::Status) -> text_input::Style {
    move |_, _| text_input::Style {
        background: Background::Color(Color::TRANSPARENT),
        border: Border::default(),
        icon: palette.text_faint,
        placeholder: palette.text_faint,
        value: palette.text,
        selection: Color { a: 0.35, ..palette.accent },
    }
}

/// Thin scrollbars that only show their thumb.
pub fn thin_scrollbar(palette: Palette) -> impl Fn(&Theme, scrollable::Status) -> scrollable::Style {
    move |theme, status| {
        let mut style = scrollable::default(theme, status);
        let rail = scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: Background::Color(palette.border),
                border: Border { radius: corners(radius::PILL), ..Border::default() },
            },
        };
        style.vertical_rail = rail;
        style.horizontal_rail = rail;
        style.container = container::Style::default();
        style
    }
}
