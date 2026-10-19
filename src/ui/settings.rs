//! Preferences.

use iced::widget::{button, column, container, row, scrollable, space, text, toggler};
use iced::{Element, Length};

use crate::app::{App, Message, Setting, ThemeSetting};
use crate::icons::Icon;
use crate::theme::{self, Palette, font, radius};
use crate::ui::widgets::CENTER;

pub fn view(app: &App, p: Palette) -> Element<'_, Message> {
    let themes = row![
        theme_card(Icon::Monitor, "System", ThemeSetting::System, app.settings.theme, p),
        theme_card(Icon::Moon, "Dark", ThemeSetting::Dark, app.settings.theme, p),
        theme_card(Icon::Sun, "Light", ThemeSetting::Light, app.settings.theme, p),
    ]
    .spacing(10);
    let notifications = column![
        switch_row("Desktop notifications", "Show a notification for new messages", Setting::Notifications, app.settings.notifications, p),
        switch_row("Unread badge", "Count unread messages on the taskbar icon", Setting::Badge, app.settings.badge, p),
        switch_row("Start at login", "Open in the tray when you sign in", Setting::Autostart, app.settings.autostart, p),
    ]
    .spacing(4);
    let about = column![
        text(format!("{} {}", crate::APP_NAME, env!("CARGO_PKG_VERSION"))).size(font::BODY).font(font::SEMIBOLD).color(p.text),
        text("Native and fast. Built with Rust and iced.").size(font::BODY).color(p.text_muted),
    ]
    .spacing(6);
    let page = column![
        text("Settings").size(font::HEADING).font(font::BOLD).color(p.text),
        space().height(Length::Fixed(12.0)),
        group("APPEARANCE", themes.into(), p),
        group("NOTIFICATIONS", notifications.into(), p),
        group("ABOUT", about.into(), p),
    ]
    .spacing(12)
    .max_width(640);
    scrollable(container(page).padding([28, 40]))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(theme::thin_scrollbar(p))
        .into()
}

fn group<'a>(title: &'a str, contents: Element<'a, Message>, p: Palette) -> Element<'a, Message> {
    column![
        text(title).size(font::SECTION).font(font::SEMIBOLD).color(p.text_faint),
        container(contents).width(Length::Fill).padding(16).style(theme::card(p.list, radius::LG, Some(p.border))),
        space().height(Length::Fixed(12.0)),
    ]
    .spacing(8)
    .into()
}

fn switch_row<'a>(title: &'a str, detail: &'a str, setting: Setting, value: bool, p: Palette) -> Element<'a, Message> {
    row![
        column![
            text(title).size(font::BODY).color(p.text),
            text(detail).size(font::CAPTION).color(p.text_muted),
        ]
        .spacing(3),
        space().width(Length::Fill),
        toggler(value).on_toggle(move |on| Message::Toggle(setting, on)).size(20),
    ]
    .align_y(CENTER)
    .height(Length::Fixed(48.0))
    .into()
}

fn theme_card<'a>(icon: Icon, label: &'a str, value: ThemeSetting, current: ThemeSetting, p: Palette) -> Element<'a, Message> {
    let selected = value == current;
    let content = column![icon.view(20.0, if selected { p.text } else { p.text_muted }), text(label).size(font::LABEL).font(font::MEDIUM).color(p.text)]
        .spacing(10)
        .align_x(CENTER);
    button(container(content).center(Length::Fill))
        .width(Length::Fixed(120.0))
        .height(Length::Fixed(76.0))
        .style(move |_, status| {
            let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: Some(iced::Background::Color(if selected {
                    p.selected
                } else if hovered {
                    p.hover
                } else {
                    p.surface
                })),
                text_color: p.text,
                border: iced::Border {
                    radius: theme::corners(radius::MD),
                    width: if selected { 1.5 } else { 0.0 },
                    color: p.accent,
                },
                ..button::Style::default()
            }
        })
        .on_press(Message::SetTheme(value))
        .into()
}
