//! The window's frame, as the original's MainWindowV2 lays it out: a 48-unit
//! title bar, the 316-unit left menu (docked from 974.4 points, otherwise
//! behind a menu button), the tab band with its 4-unit rule, and the page.

use iced::widget::{Stack, button, column, container, image, mouse_area, responsive, row, space, stack};
use iced::{Background, Border, Element, Length, Size, mouse, window};

use crate::app::{App, Message};
use crate::icons::Icon;
use crate::model::HelperState;
use crate::screens::Screen;
use crate::theme::{self, Palette, c, font, size};
use crate::ui::widgets::{icon, press_row};
use crate::ui::{Board, rect, t};

/// Height of the tab band above the page (title bar to the rule).
pub const BAND_TOP: f32 = 48.0;
pub const RULE_TOP: f32 = 110.0;
/// Where pages start (below the rule).
pub const PAGE_TOP: f32 = 114.0;

pub fn view(app: &App, p: Palette) -> Element<'_, Message> {
    let docked = app.menu_docked();
    let content = responsive(move |area: Size| page(app, p, area, docked));
    let body: Element<'_, Message> = if docked {
        row![menu(app, p), content].height(Length::Fill).into()
    } else {
        content.into()
    };
    let main = column![titlebar(app, p), body];
    let mut layers = Stack::new().width(Length::Fill).height(Length::Fill);
    layers = layers.push(container(main).width(Length::Fill).height(Length::Fill).style(theme::fill(c::WINDOW)));
    if !docked && app.menu_open {
        // The menu opens over the content, under a dimmed cover (MenuNameCover).
        let overlay = column![
            space().height(Length::Fixed(size::TITLEBAR)),
            row![
                menu(app, p),
                mouse_area(container(space()).width(Length::Fill).height(Length::Fill).style(theme::fill(p.cover)))
                    .on_press(Message::ToggleMenu)
            ]
            .height(Length::Fill)
        ];
        layers = layers.push(overlay);
    }
    if app.settings_open {
        layers = layers.push(settings_popover(app, p, docked));
    }
    layers = layers.push(resize_edges());
    layers.into()
}

/// Everything right of the menu: the tab band, the rule, then the page.
fn page<'a>(app: &'a App, p: Palette, area: Size, docked: bool) -> Element<'a, Message> {
    let page_height = (area.height - PAGE_TOP + size::TITLEBAR).max(0.0);
    let body: Element<'a, Message> = match app.screen {
        Screen::Performance => crate::ui::performance::view(app, area.width, page_height),
        Screen::Vitals | Screen::VitalsAdvanced | Screen::Sysinfo => crate::ui::vitals::view(app, area.width, page_height),
    };
    let band = tab_band(app, p, area.width, docked);
    column![band, body].width(Length::Fill).height(Length::Fill).into()
}

/// The band with the two hardware tabs (the original's third tab, Network
/// Booster, is not a hardware feature and is left out).
fn tab_band<'a>(app: &'a App, p: Palette, width: f32, docked: bool) -> Element<'a, Message> {
    let mut board = Board::new(width, PAGE_TOP - BAND_TOP)
        .at(0.0, 0.0, rect(width, RULE_TOP - BAND_TOP, c::BAND))
        .at(0.0, RULE_TOP - BAND_TOP, rect(width, size::TAB_RULE, c::TAB_RULE));
    let mut x = 32.0;
    if !docked {
        let menu_button = button(container(icon(Icon::Menu, 24.0, c::TEXT)).center(Length::Fixed(48.0)))
            .padding(0)
            .style(theme::hover_fill(c::BANNER))
            .on_press(Message::ToggleMenu);
        board = board
            .at(0.0, 7.0, menu_button)
            .at(56.0, 18.0, t(app.store.device.clone(), 20.0, font::BLACK, p.band_title));
        x = 229.0;
    }
    let vitals_selected = app.screen.is_vitals();
    board = board
        .at(x, 9.0, tab("System Vitals", 145.0, vitals_selected, Message::Navigate(app.vitals_screen)))
        .at(x + 145.0, 9.0, tab("Performance Control", 204.0, !vitals_selected, Message::Navigate(Screen::Performance)));
    board.into()
}

/// One header tab (SubMenuButtonStyle): a 5-unit gradient line over the
/// title box when selected, #484848 on hover; acts on mouse-down.
fn tab<'a>(label: &'a str, width: f32, selected: bool, on_press: Message) -> Element<'a, Message> {
    let face = move |hovered: bool| -> Element<'a, Message> {
        let line: Element<'a, Message> = if selected {
            container(space())
                .width(Length::Fixed(width - 2.5))
                .height(Length::Fixed(5.0))
                .style(|_| container::Style {
                    background: Some(Background::Gradient(theme::tab_gradient())),
                    border: Border { radius: iced::border::Radius { top_left: 4.4, top_right: 4.4, bottom_right: 0.0, bottom_left: 0.0 }, ..Border::default() },
                    ..container::Style::default()
                })
                .into()
        } else {
            rect(width - 2.5, 5.0, if hovered { c::TAB_RULE } else { c::TAB_IDLE })
        };
        let fill = if selected || hovered { c::TAB_RULE } else { c::TAB_IDLE };
        let title = container(t(label, 16.0, font::MEDIUM, c::WHITE).line_height(iced::widget::text::LineHeight::Absolute(26.0.into())))
            .padding([0.0, 16.0])
            .padding(iced::Padding { top: 14.0, right: 16.0, bottom: 8.0, left: 16.0 })
            .width(Length::Fixed(width - 2.5))
            .height(Length::Fixed(48.0))
            .style(theme::fill(fill));
        column![line, title].into()
    };
    press_row(face, on_press)
}

fn titlebar(app: &App, p: Palette) -> Element<'_, Message> {
    // One handle for the life of the app: a new handle per view would decode the PNG every frame.
    static MARK: std::sync::OnceLock<image::Handle> = std::sync::OnceLock::new();
    let mark = image(MARK.get_or_init(|| image::Handle::from_bytes(include_bytes!("../../assets/icon.png").as_slice())).clone())
        .width(Length::Fixed(22.0))
        .height(Length::Fixed(22.0));
    let caption = |glyph: Icon, message: Message, close: bool| {
        button(container(icon(glyph, 16.0, p.title_text)).center(Length::Fixed(size::TITLEBAR)))
            .padding(0)
            .width(Length::Fixed(size::CAPTION))
            .height(Length::Fixed(size::TITLEBAR))
            .style(theme::caption(p, close))
            .on_press(message)
    };
    let gear = button(container(icon(Icon::Settings, 20.0, p.title_text)).center(Length::Fixed(size::TITLEBAR)))
        .padding(0)
        .width(Length::Fixed(44.0))
        .height(Length::Fixed(size::TITLEBAR))
        .style(theme::caption(p, false))
        .on_press(Message::ToggleSettings);
    let controls = row![
        gear,
        space().width(Length::Fixed(48.0)),
        caption(Icon::Minimize, Message::Minimize, false),
        caption(if app.maximized { Icon::Restore } else { Icon::Maximize }, Message::ToggleMaximize, false),
        caption(Icon::Close, Message::CloseRequested, true),
    ];
    let bar = row![
        space().width(Length::Fixed(12.0)),
        mark,
        space().width(Length::Fixed(10.0)),
        t(crate::APP_NAME, 15.0, font::MEDIUM, p.title_text),
        space().width(Length::Fill),
        controls,
    ]
    .align_y(iced::Alignment::Center)
    .height(Length::Fixed(size::TITLEBAR));
    mouse_area(container(bar).width(Length::Fill).style(theme::fill(p.title_bg)))
        .on_press(Message::DragWindow)
        .on_double_click(Message::ToggleMaximize)
        .into()
}

/// The left menu. This laptop's group, its two pages and Settings sit where
/// the original has its device group and HELP & FEEDBACK; the original's other
/// entries (home, store, games, gallery, offers...) are left out, not moved up.
fn menu<'a>(app: &'a App, p: Palette) -> Element<'a, Message> {
    let group = row![
        space().width(Length::Fixed(20.0)),
        icon(Icon::Laptop, 20.0, p.menu_text),
        space().width(Length::Fixed(12.0)),
        t(app.store.device.clone(), 14.0, font::BOLD, p.menu_text),
        space().width(Length::Fill),
        icon(Icon::ChevronUp, 14.0, p.menu_text),
        space().width(Length::Fixed(24.0)),
    ]
    .align_y(iced::Alignment::Center)
    .width(Length::Fixed(size::MENU))
    .height(Length::Fixed(32.0));
    let vitals = sub_row("System Vitals", app.screen.is_vitals(), Message::Navigate(app.vitals_screen), p);
    let performance = sub_row("Performance Control", app.screen == Screen::Performance, Message::Navigate(Screen::Performance), p);
    let settings = press_row(
        move |hovered| {
            let line = row![
                space().width(Length::Fixed(20.0)),
                icon(Icon::Settings, 20.0, p.menu_text),
                space().width(Length::Fixed(12.0)),
                t("SETTINGS", 14.0, font::BOLD, p.menu_text),
            ]
            .align_y(iced::Alignment::Center)
            .height(Length::Fixed(32.0));
            menu_fill(line.into(), hovered || app.settings_open, p)
        },
        Message::ToggleSettings,
    );
    let footer = container(t(app.store.version.clone(), 12.0, font::BOOK, p.footer_text))
        .width(Length::Fixed(size::MENU - 16.0))
        .align_x(iced::alignment::Horizontal::Right);
    let items = Board::new(Length::Fixed(size::MENU), Length::Fill)
        .at(0.0, 383.0, divider(p))
        .at(0.0, 395.6, group)
        .at(0.0, 430.0, vitals)
        .at(0.0, 465.6, performance)
        .at(0.0, 549.0, divider(p))
        .at(0.0, 561.0, settings)
        .push(container(footer).height(Length::Fill).align_y(iced::alignment::Vertical::Bottom).padding(iced::Padding { bottom: 9.0, ..iced::Padding::ZERO }));
    container(items)
        .width(Length::Fixed(size::MENU))
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(p.menu_bg)),
            border: Border { color: p.menu_border, width: 1.0, radius: 0.0.into() },
            ..container::Style::default()
        })
        .into()
}

/// A second-level menu row (System Vitals, Performance Control): the
/// LeftMenuHighlight gradient and indicator when selected or hovered.
fn sub_row<'a>(label: &'a str, selected: bool, on_press: Message, p: Palette) -> Element<'a, Message> {
    press_row(
        move |hovered| {
            let indicator: Element<'a, Message> = if selected {
                container(space())
                    .width(Length::Fixed(6.0))
                    .height(Length::Fixed(24.0))
                    .style(move |_| container::Style {
                        background: Some(if p.light { Background::Color(c::TAB_RULE) } else { Background::Gradient(theme::indicator_gradient()) }),
                        border: Border { radius: iced::border::Radius { top_left: 0.0, top_right: 2.0, bottom_right: 2.0, bottom_left: 0.0 }, ..Border::default() },
                        ..container::Style::default()
                    })
                    .into()
            } else {
                space().width(Length::Fixed(6.0)).into()
            };
            let line = row![indicator, space().width(Length::Fixed(58.0)), t(label, 14.0, font::BOOK, p.menu_text)]
                .align_y(iced::Alignment::Center)
                .height(Length::Fixed(32.0));
            menu_fill(line.into(), selected || hovered, p)
        },
        on_press,
    )
}

fn menu_fill<'a>(content: Element<'a, Message>, lit: bool, p: Palette) -> Element<'a, Message> {
    container(content)
        .width(Length::Fixed(size::MENU - 4.0))
        .style(move |_| container::Style {
            background: lit.then(|| Background::Gradient(theme::menu_highlight(p.light))),
            ..container::Style::default()
        })
        .into()
}

fn divider<'a>(p: Palette) -> Element<'a, Message> {
    container(space())
        .width(Length::Fixed(size::MENU))
        .height(Length::Fixed(1.0))
        .style(move |_| container::Style { background: Some(Background::Gradient(theme::divider_gradient(p.light))), ..container::Style::default() })
        .into()
}

/// Settings: theme and the hardware helper's state.
fn settings_popover<'a>(app: &'a App, _p: Palette, docked: bool) -> Element<'a, Message> {
    let choice = |label: &'a str, on: bool, message: Message| -> Element<'a, Message> {
        button(container(t(label, 14.0, font::MEDIUM, if on { c::BASE } else { c::TEXT })).center_x(Length::Fixed(120.0)).padding([8.0, 0.0]))
            .padding(0)
            .style(move |_, status| button::Style {
                background: Some(Background::Color(if on { c::TEXT } else if matches!(status, button::Status::Hovered) { c::GREY_DARK } else { c::BANNER })),
                text_color: c::TEXT,
                border: Border { radius: 3.0.into(), ..Border::default() },
                ..button::Style::default()
            })
            .on_press(message)
            .into()
    };
    let helper: Element<'a, Message> = match app.store.controls.helper {
        HelperState::Ready => t("Hardware control is on.", 14.0, font::BOOK, c::GREY80).into(),
        HelperState::Unsupported => t("Hardware control needs Windows on this laptop.", 14.0, font::BOOK, c::GREY80).into(),
        HelperState::Missing => column![
            t("Hardware control is off. Turning it on installs a small", 14.0, font::BOOK, c::GREY80),
            t("helper service (Windows asks for your approval once).", 14.0, font::BOOK, c::GREY80),
            space().height(Length::Fixed(10.0)),
            button(t("TURN ON HARDWARE CONTROL", 14.0, font::BOLD, c::BASE)).padding([10.0, 20.0]).style(theme::action).on_press(Message::EnableHelper),
        ]
        .into(),
    };
    let panel = container(
        column![
            t("THEME", 16.0, font::BOLD, c::TEXT),
            space().height(Length::Fixed(12.0)),
            row![choice("Dark", !app.settings.light, Message::SetLight(false)), space().width(Length::Fixed(12.0)), choice("Light", app.settings.light, Message::SetLight(true))],
            space().height(Length::Fixed(24.0)),
            t("HARDWARE CONTROL", 16.0, font::BOLD, c::TEXT),
            space().height(Length::Fixed(12.0)),
            helper,
        ]
        .padding(24),
    )
    .width(Length::Fixed(440.0))
    .style(theme::fill_border(c::CARD, c::GREY_DARK, 1.0, 4.0));
    let left = if docked { size::MENU + 8.0 } else { 8.0 };
    let cover = mouse_area(container(space()).width(Length::Fill).height(Length::Fill)).on_press(Message::ToggleSettings);
    stack![cover, Board::new(Length::Fill, Length::Fill).at(left, 200.0, mouse_area(panel).interaction(mouse::Interaction::Idle))].into()
}

/// Invisible strips on the window's edges that resize the frameless window.
fn resize_edges<'a>() -> Element<'a, Message> {
    use window::Direction::*;
    let edge = |direction, width: Length, height: Length, cursor| -> Element<'a, Message> {
        mouse_area(space().width(width).height(height)).interaction(cursor).on_press(Message::ResizeWindow(direction)).into()
    };
    let (h, v) = (mouse::Interaction::ResizingHorizontally, mouse::Interaction::ResizingVertically);
    let (up, down) = (mouse::Interaction::ResizingDiagonallyUp, mouse::Interaction::ResizingDiagonallyDown);
    column![
        row![edge(NorthWest, Length::Fixed(6.0), Length::Fixed(6.0), down), edge(North, Length::Fill, Length::Fixed(4.0), v), edge(NorthEast, Length::Fixed(6.0), Length::Fixed(6.0), up)],
        row![edge(West, Length::Fixed(4.0), Length::Fill, h), space().width(Length::Fill).height(Length::Fill), edge(East, Length::Fixed(4.0), Length::Fill, h)]
            .height(Length::Fill),
        row![edge(SouthWest, Length::Fixed(6.0), Length::Fixed(6.0), up), edge(South, Length::Fill, Length::Fixed(4.0), v), edge(SouthEast, Length::Fixed(6.0), Length::Fixed(6.0), down)],
    ]
    .into()
}
