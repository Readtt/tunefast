//! Colours, fonts and sizes, taken from the original's theme dictionaries
//! (projects/tunefast/design/tokens.json and design/specs/*.md). Sizes are the
//! original's design units (WPF DIPs on its 1624x940 canvas); the whole window
//! is scaled to fit, as the original does (see `App::scale`).

use iced::border::Radius;
use iced::font::{Family, Weight};
use iced::gradient::{ColorStop, Linear};
use iced::widget::{button, container, scrollable};
use iced::{Background, Border, Color, Font, Gradient, Radians, Shadow, Theme};

pub const fn hex(rgb: u32) -> Color {
    Color::from_rgb8(((rgb >> 16) & 0xFF) as u8, ((rgb >> 8) & 0xFF) as u8, (rgb & 0xFF) as u8)
}

pub const fn hexa(rgb: u32, alpha: f32) -> Color {
    let c = hex(rgb);
    Color { a: alpha, ..c }
}

/// The colours a screen uses. The light theme only changes the shell chrome,
/// exactly as the original's LightThemeColorsOverrides do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub light: bool,
    pub title_bg: Color,
    pub title_text: Color,
    pub title_hover: Color,
    pub menu_bg: Color,
    pub menu_text: Color,
    pub menu_border: Color,
    pub divider: Color,
    pub footer_text: Color,
    pub cover: Color,
    /// The device name beside the menu button (narrow windows).
    pub band_title: Color,
}

impl Palette {
    pub const DARK: Palette = Palette {
        light: false,
        title_bg: hex(0x1B1B1B),
        title_text: hex(0xFFFFFF),
        title_hover: hexa(0xEEEEEE, 0.10),
        menu_bg: hex(0x1B1B1B),
        menu_text: hex(0xEEEEEE),
        menu_border: Color::TRANSPARENT,
        divider: hex(0x3A3A3A),
        footer_text: hex(0x9B9B9B),
        cover: hexa(0x0A0A0A, 0.93),
        band_title: hex(0xFFFFFF),
    };

    pub const LIGHT: Palette = Palette {
        light: true,
        title_bg: hex(0xE3E3E3),
        title_text: hex(0x121212),
        title_hover: hexa(0x000000, 0.20),
        menu_bg: hex(0xD8D8D8),
        menu_text: hex(0x121212),
        menu_border: hex(0xD8D8D8),
        divider: hex(0x303030),
        footer_text: hex(0x303030),
        cover: hexa(0xD8D8D8, 0.93),
        band_title: hex(0xFFFFFF),
    };
}

/// Content colours (identical in both themes).
pub mod c {
    use super::{Color, hex, hexa};
    /// The window behind the pages (the original's dark background image).
    pub const WINDOW: Color = hex(0x232323);
    /// The tab band above the pages.
    pub const BAND: Color = hex(0x313131);
    /// Performance Control's panel: #000000cc over the window.
    pub const PANEL: Color = hex(0x070707);
    pub const BASE: Color = hex(0x212121);
    pub const TINT: Color = hexa(0x000000, 0.80);
    pub const TEXT: Color = hex(0xEEEEEE);
    pub const WHITE: Color = hex(0xFFFFFF);
    pub const GREY80: Color = hex(0xD8D8D8);
    pub const GREY: Color = hex(0x757575);
    pub const GREY_DARK: Color = hex(0x424242);
    pub const HYPER_GREY: Color = hex(0x9B9B9B);
    pub const DETAIL_GREY: Color = hex(0x9E9E9E);
    pub const CARD: Color = hex(0x181818);
    pub const BANNER: Color = hex(0x303030);
    pub const AREA: Color = hex(0x121212);
    pub const TAB_RULE: Color = hex(0x484848);
    pub const TAB_IDLE: Color = hex(0x121212);
    pub const MANGO: Color = hex(0xFFA338);
    pub const BLUE: Color = hex(0x4A90E2);
    pub const TEAL: Color = hex(0x48C8AB);
    pub const NORMAL: Color = hex(0x0FFA84);
    pub const WARNING: Color = hex(0xFFA338);
    pub const HOT: Color = hex(0xF9350F);
    pub const RING_MASK: Color = hex(0x363636);
    pub const TRACK: Color = hex(0x5C5C5C);
    pub const CHART_BG: Color = hex(0x303030);
    pub const CLOCK: Color = hex(0x0D7BFC);
    pub const TEMP: Color = hex(0xA044FF);
    pub const UTIL: Color = hex(0xFBB216);
    pub const ACTION_BAR: Color = hexa(0x646464, 0x85 as f32 / 255.0);
    pub const PINK: Color = hex(0xEE5BA0);
    pub const RED: Color = hex(0xEC1F2E);
    pub const GOLD: Color = hex(0xFBB216);
}

pub mod font {
    use super::{Family, Font, Weight};
    const fn mont(weight: Weight) -> Font {
        Font { family: Family::Name("Montserrat"), weight, ..Font::DEFAULT }
    }
    /// GothamLight / Book / Medium / Bold / Black -> Montserrat (font-match.md).
    pub const LIGHT: Font = mont(Weight::Light);
    pub const BOOK: Font = mont(Weight::Normal);
    pub const MEDIUM: Font = mont(Weight::Medium);
    pub const BOLD: Font = mont(Weight::Bold);
    pub const BLACK: Font = mont(Weight::Black);
    /// VenusRising (sensor values) -> Zen Dots.
    pub const DISPLAY: Font = Font::with_name("Zen Dots");
}

/// Design units (the original's DIPs).
pub mod size {
    pub const DESIGN: [f32; 2] = [1624.0, 940.0];
    pub const MIN_SCALE: f32 = 0.6;
    pub const DEFAULT_WINDOW: [f32; 2] = [1280.0, 800.0];
    pub const MIN_WINDOW: [f32; 2] = [768.0, 564.0];
    /// The left menu docks from this window width (1624 x 0.6).
    pub const MENU_BREAKPOINT: f32 = 974.4;
    pub const TITLEBAR: f32 = 48.0;
    pub const MENU: f32 = 316.0;
    pub const CAPTION: f32 = 40.0;
    pub const TABS: f32 = 58.0;
    pub const TAB_RULE: f32 = 4.0;
}

/// The original's signature gradient (pink -> red -> gold), bottom-left to top-right.
pub fn omen_gradient(angle_degrees: f32) -> Gradient {
    Gradient::Linear(
        Linear::new(Radians(angle_degrees.to_radians()))
            .add_stops([
                ColorStop { offset: 0.0, color: c::PINK },
                ColorStop { offset: 0.5, color: c::RED },
                ColorStop { offset: 1.0, color: c::GOLD },
            ]),
    )
}

/// The selected tab's line: #ff55e1 > #ff0000 (0.52) > #ffb400, left to right.
pub fn tab_gradient() -> Gradient {
    Gradient::Linear(Linear::new(Radians(std::f32::consts::FRAC_PI_2)).add_stops([
        ColorStop { offset: 0.0, color: hex(0xFF55E1) },
        ColorStop { offset: 0.52, color: hex(0xFF0000) },
        ColorStop { offset: 1.0, color: hex(0xFFB400) },
    ]))
}

/// The selected menu row's indicator: gold > red > pink, top to bottom.
pub fn indicator_gradient() -> Gradient {
    Gradient::Linear(Linear::new(Radians(std::f32::consts::PI)).add_stops([
        ColorStop { offset: 0.0, color: hex(0xFFB400) },
        ColorStop { offset: 0.5, color: hex(0xFF0000) },
        ColorStop { offset: 1.0, color: hex(0xFF55E1) },
    ]))
}

/// LeftMenuHighlight: (0,0) -> (1,1) transparent > #ff55e1 10% > #ff0000 22%
/// (light: transparent > #9b9b9b).
pub fn menu_highlight(light: bool) -> Gradient {
    let angle = Radians(std::f32::consts::FRAC_PI_2 + 0.2);
    if light {
        Gradient::Linear(Linear::new(angle).add_stops([
            ColorStop { offset: 0.0, color: hexa(0xE3E3E3, 0.0) },
            ColorStop { offset: 1.0, color: hex(0x9B9B9B) },
        ]))
    } else {
        Gradient::Linear(Linear::new(angle).add_stops([
            ColorStop { offset: 0.0, color: hexa(0x000000, 0.0) },
            ColorStop { offset: 0.5, color: hexa(0xFF55E1, 0x19 as f32 / 255.0) },
            ColorStop { offset: 1.0, color: hexa(0xFF0000, 0x38 as f32 / 255.0) },
        ]))
    }
}

/// The menu divider: transparent > #eeeeee > transparent (light #303030).
pub fn divider_gradient(light: bool) -> Gradient {
    let mid = if light { hex(0x303030) } else { hex(0xEEEEEE) };
    Gradient::Linear(Linear::new(Radians(std::f32::consts::FRAC_PI_2)).add_stops([
        ColorStop { offset: 0.0, color: Color { a: 0.0, ..mid } },
        ColorStop { offset: 0.5, color: Color { a: 0.35, ..mid } },
        ColorStop { offset: 1.0, color: Color { a: 0.0, ..mid } },
    ]))
}

/// The fan section's separators: #696969 > #ffffff > #696969.
pub fn separator_gradient() -> Gradient {
    Gradient::Linear(Linear::new(Radians(std::f32::consts::FRAC_PI_2)).add_stops([
        ColorStop { offset: 0.0, color: hexa(0x696969, 0.0) },
        ColorStop { offset: 0.25, color: hex(0x696969) },
        ColorStop { offset: 0.5, color: hex(0xFFFFFF) },
        ColorStop { offset: 0.75, color: hex(0x696969) },
        ColorStop { offset: 1.0, color: hexa(0x696969, 0.0) },
    ]))
}

/// Tiles and buttons: #303030 > #212121 from the top-left corner.
pub fn tile_gradient() -> Gradient {
    Gradient::Linear(Linear::new(Radians(std::f32::consts::FRAC_PI_4 * 3.0)).add_stops([
        ColorStop { offset: 0.0, color: hex(0x303030) },
        ColorStop { offset: 1.0, color: hex(0x212121) },
    ]))
}

/// The advanced view's stripes: #212121 > #181818, top to bottom.
pub fn stripe_gradient() -> Gradient {
    Gradient::Linear(Linear::new(Radians(std::f32::consts::PI)).add_stops([
        ColorStop { offset: 0.0, color: hex(0x212121) },
        ColorStop { offset: 1.0, color: hex(0x181818) },
    ]))
}

/// A button that only shows a fill on hover.
pub fn hover_fill(color: Color) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| button::Style {
        background: matches!(status, button::Status::Hovered | button::Status::Pressed).then_some(Background::Color(color)),
        text_color: c::TEXT,
        border: Border::default(),
        shadow: Shadow::default(),
        snap: true,
    }
}

/// `color` at `alpha` over `base`, as an opaque colour.
pub fn over(color: Color, alpha: f32, base: Color) -> Color {
    let mix = |a: f32, b: f32| a * alpha + b * (1.0 - alpha);
    Color::from_rgb(mix(color.r, base.r), mix(color.g, base.g), mix(color.b, base.b))
}

pub fn fill(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(Background::Color(color)), ..container::Style::default() }
}

pub fn fill_border(color: Color, border: Color, width: f32, radius: f32) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style {
        background: Some(Background::Color(color)),
        border: Border { color: border, width, radius: Radius::from(radius) },
        ..container::Style::default()
    }
}

pub fn gradient_fill(gradient: Gradient) -> impl Fn(&Theme) -> container::Style {
    move |_| container::Style { background: Some(Background::Gradient(gradient)), ..container::Style::default() }
}

/// A flat, chrome-free button whose look comes from its content.
pub fn bare(_theme: &Theme, _status: button::Status) -> button::Style {
    button::Style { background: None, text_color: c::TEXT, border: Border::default(), shadow: Shadow::default(), snap: true }
}

/// Window caption buttons: hover fill, red for close.
pub fn caption(p: Palette, close: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let background = match (hovered, close) {
            (true, true) => Some(Background::Color(hex(0xFF0000))),
            (true, false) => Some(Background::Color(p.title_hover)),
            _ => None,
        };
        button::Style { background, text_color: p.title_text, border: Border::default(), shadow: Shadow::default(), snap: true }
    }
}

/// The light-grey action button (COPY TO CLIPBOARD, PRINT, OK).
pub fn action(_theme: &Theme, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if hovered { c::WHITE } else { c::TEXT })),
        text_color: if hovered { hex(0x000000) } else { c::BASE },
        border: Border { color: Color::TRANSPARENT, width: 1.0, radius: Radius::from(4.0) },
        shadow: Shadow::default(),
        snap: true,
    }
}

/// The narrow scrollbar of the original's lists.
pub fn scrollbar(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    let active = matches!(status, scrollable::Status::Hovered { .. } | scrollable::Status::Dragged { .. });
    let mut style = scrollable::default(theme, status);
    let rail = scrollable::Rail {
        background: Some(Background::Color(if active { c::GREY } else { c::TAB_RULE })),
        border: Border::default(),
        scroller: scrollable::Scroller {
            background: Background::Color(hexa(0xFFFFFF, if active { 0.8 } else { 0.6 })),
            border: Border::default(),
        },
    };
    style.vertical_rail = rail;
    style.horizontal_rail = rail;
    style.container = container::Style::default();
    style
}

/// Window scale, as the original computes it: the 1624x940 design fitted into
/// the window, never above 1 and never below 0.6.
pub fn window_scale(width: f32, height: f32) -> f32 {
    (width / size::DESIGN[0]).min(height / size::DESIGN[1]).clamp(size::MIN_SCALE, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_matches_the_original_rule() {
        assert!((window_scale(1280.0, 800.0) - 1280.0 / 1624.0).abs() < 1e-6);
        assert!((window_scale(1536.0, 820.0) - 820.0 / 940.0).abs() < 1e-6);
        assert_eq!(window_scale(960.0, 640.0), 0.6);
        assert_eq!(window_scale(3000.0, 2000.0), 1.0);
    }
}
