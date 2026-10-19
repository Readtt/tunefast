//! The window's frame: a custom title bar (drag, double-click to maximize,
//! caption buttons), resizable edges, the navigation rail, and the area the
//! current screen fills.

use iced::widget::{button, column, container, mouse_area, row, space, stack, text, text_input};
use iced::{Element, Length, mouse, window};

use crate::app::{App, Message};
use crate::icons::Icon;
use crate::screens::Screen;
use crate::theme::{self, Palette, font, radius, size};
use crate::ui::widgets::{self, CENTER, ids};

pub fn view(app: &App, p: Palette) -> Element<'_, Message> {
    let content = match app.screen {
        Screen::Home | Screen::Empty => crate::ui::home::view(app, p),
        Screen::Settings => crate::ui::settings::view(app, p),
    };
    let page = column![titlebar(app, p), row![sidebar(app, p), content].height(Length::Fill)];
    stack![
        container(page).width(Length::Fill).height(Length::Fill).style(theme::fill(p.window)),
        resize_edges(),
    ]
    .into()
}

fn titlebar(app: &App, p: Palette) -> Element<'_, Message> {
    let mark = container(text("T").size(font::SECTION).font(font::BOLD).color(p.on_accent))
        .center(Length::Fixed(18.0))
        .style(theme::card(p.accent, radius::SM + 1.0, None));
    let left = row![mark, text(crate::APP_NAME).size(font::LABEL).font(font::MEDIUM).color(p.text_muted)]
        .spacing(8)
        .align_y(CENTER)
        .padding([0, 12]);
    let caption = |icon: Icon, message: Message, danger: bool| {
        button(container(icon.view(14.0, p.text_muted)).center(Length::Fixed(size::TITLEBAR)))
            .padding(0)
            .width(Length::Fixed(size::CAPTION_BUTTON))
            .height(Length::Fixed(size::TITLEBAR))
            .style(theme::caption_button(p, danger))
            .on_press(message)
    };
    let controls = row![
        caption(Icon::Minimize, Message::Minimize, false),
        caption(Icon::Maximize, Message::ToggleMaximize, false),
        caption(Icon::Close, Message::CloseRequested, true),
    ];
    let search = container(
        row![
            Icon::Search.view(14.0, p.text_faint),
            text_input("Search", &app.search)
                .id(ids::SEARCH)
                .on_input(Message::SearchChanged)
                .size(font::BODY)
                .padding([6, 0])
                .style(theme::bare_input(p)),
        ]
        .spacing(8)
        .align_y(CENTER)
        .padding([0, 12]),
    )
    .width(Length::Fixed(440.0))
    .height(Length::Fixed(size::INPUT - 4.0))
    .center_y(Length::Fixed(size::INPUT - 4.0))
    .style(theme::card(p.surface, radius::MD, None));
    let bar = stack![
        row![left, space().width(Length::Fill), controls].align_y(CENTER).height(Length::Fixed(size::TITLEBAR)),
        container(search).center(Length::Fill),
    ]
    .height(Length::Fixed(size::TITLEBAR));
    mouse_area(container(bar).width(Length::Fill).style(theme::fill(p.titlebar)))
        .on_press(Message::DragWindow)
        .on_double_click(Message::ToggleMaximize)
        .into()
}

fn sidebar<'a>(app: &'a App, p: Palette) -> Element<'a, Message> {
    let workspace = button(
        row![
            widgets::avatar(&app.store.workspace, 0.62, 26.0, None, p),
            text(app.store.workspace.clone()).size(font::BODY).font(font::SEMIBOLD).color(p.text),
            space().width(Length::Fill),
            Icon::ChevronDown.view(14.0, p.text_muted),
        ]
        .spacing(10)
        .align_y(CENTER)
        .padding([0, 6]),
    )
    .width(Length::Fill)
    .height(Length::Fixed(40.0))
    .style(theme::row_button(p, false, radius::MD))
    .on_press(Message::Navigate(Screen::Home));

    let unread: u32 = app.store.conversations.iter().map(|c| c.unread).sum();
    let home = matches!(app.screen, Screen::Home | Screen::Empty);
    let mut nav = column![
        workspace,
        space().height(Length::Fixed(8.0)),
        nav_row(Icon::Inbox, "Inbox", home, Some(unread), Message::Navigate(Screen::Home), p),
        nav_row(Icon::Star, "Starred", false, None, Message::Navigate(Screen::Home), p),
        nav_row(Icon::Bell, "Mentions", false, Some(3), Message::Navigate(Screen::Home), p),
        nav_row(Icon::Archive, "Archive", false, None, Message::Navigate(Screen::Home), p),
        space().height(Length::Fixed(16.0)),
        container(text("CHANNELS").size(font::SECTION).font(font::SEMIBOLD).color(p.text_faint)).padding([4, 10]),
    ]
    .spacing(2);
    for channel in &app.store.channels {
        let color = if channel.unread { p.text } else { p.text_muted };
        let face = move |hovered: bool| -> Element<'a, Message> {
            let mut line = row![
                Icon::Hash.view(14.0, p.text_faint),
                text(channel.name.clone()).size(font::BODY).font(if channel.unread { font::SEMIBOLD } else { font::REGULAR }).color(color),
                space().width(Length::Fill),
            ]
            .spacing(10)
            .align_y(CENTER)
            .padding([0, 10]);
            if channel.unread {
                line = line.push(container("").width(Length::Fixed(7.0)).height(Length::Fixed(7.0)).style(theme::card(p.text, 4.0, None)));
            }
            container(line)
                .width(Length::Fill)
                .height(Length::Fixed(30.0))
                .center_y(Length::Fixed(30.0))
                .style(theme::row_fill(p, false, hovered, radius::MD))
                .into()
        };
        nav = nav.push(widgets::press_row(face, Message::Navigate(Screen::Home), None));
    }
    let settings = nav_row(Icon::Settings, "Settings", app.screen == Screen::Settings, None, Message::Navigate(Screen::Settings), p);
    container(column![nav, space().height(Length::Fill), settings])
        .width(Length::Fixed(size::SIDEBAR))
        .height(Length::Fill)
        .padding([12, 10])
        .style(theme::fill(p.sidebar))
        .into()
}

/// A sidebar row: switches on mouse-down, like the original's navigation.
fn nav_row<'a>(icon: Icon, label: &'a str, selected: bool, count: Option<u32>, message: Message, p: Palette) -> Element<'a, Message> {
    let tint = if selected { p.text } else { p.text_muted };
    let face = move |hovered: bool| -> Element<'a, Message> {
        let mut line = row![icon.view(size::ICON, tint), text(label).size(font::LABEL).font(font::MEDIUM).color(tint), space().width(Length::Fill)]
            .spacing(10)
            .align_y(CENTER)
            .padding([0, 10]);
        if let Some(count) = count.filter(|c| *c > 0) {
            line = line.push(widgets::badge(count, p));
        }
        container(line)
            .width(Length::Fill)
            .height(Length::Fixed(size::NAV_ROW))
            .center_y(Length::Fixed(size::NAV_ROW))
            .style(theme::row_fill(p, selected, hovered, radius::MD))
            .into()
    };
    widgets::press_row(face, message, None)
}

/// Invisible strips on the window's edges that resize the frameless window.
fn resize_edges<'a>() -> Element<'a, Message> {
    use window::Direction::*;
    let edge = |direction, width: Length, height: Length, cursor| -> Element<'a, Message> {
        mouse_area(space().width(width).height(height))
            .interaction(cursor)
            .on_press(Message::ResizeWindow(direction))
            .into()
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
