//! The main view: the conversation list and the open conversation.

use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, scrollable, space, text, text_input};
use iced::{Element, Length};

use crate::app::{App, Message};
use crate::icons::Icon;
use crate::model::{Conversation, Message as ChatMessage};
use crate::screens::Screen;
use crate::theme::{self, Palette, font, radius, size};
use crate::ui::widgets::{self, CENTER, ids};

pub fn view(app: &App, p: Palette) -> Element<'_, Message> {
    let main = match (app.screen, app.selected.and_then(|id| app.store.conversation(id))) {
        (Screen::Home, Some(c)) => conversation(app, c, p),
        _ => empty(p),
    };
    row![list(app, p), main].height(Length::Fill).into()
}

fn list(app: &App, p: Palette) -> Element<'_, Message> {
    let header = row![
        text("Inbox").size(font::TITLE).font(font::SEMIBOLD).color(p.text),
        space().width(Length::Fill),
        widgets::icon_button(Icon::Plus, Message::Navigate(Screen::Empty), p),
    ]
    .align_y(CENTER)
    .padding([0, 12])
    .height(Length::Fixed(size::HEADER));

    // Only the rows in view are built; spacers stand in for the rest, so a
    // list of 100,000 rows costs the same as one of 15. Every row is exactly
    // LIST_ROW tall, which is what makes the spacer arithmetic exact.
    let rows: Vec<&Conversation> = app.rows();
    let first = (app.list_offset / size::LIST_ROW).floor().max(0.0) as usize;
    let visible = (app.list_height / size::LIST_ROW).ceil() as usize + 2;
    let last = (first + visible).min(rows.len());
    let first = first.min(last);
    let mut body = column![space().height(Length::Fixed(first as f32 * size::LIST_ROW))];
    for c in &rows[first..last] {
        let selected = app.screen == Screen::Home && app.selected == Some(c.id);
        body = body.push(list_row(c, selected, p));
    }
    body = body.push(space().height(Length::Fixed((rows.len() - last) as f32 * size::LIST_ROW)));

    let scroller = scrollable(body.padding([0, 8]))
        .id(ids::LIST)
        .on_scroll(|viewport| Message::ListScrolled { offset: viewport.absolute_offset().y, height: viewport.bounds().height })
        .height(Length::Fill)
        .style(theme::thin_scrollbar(p));
    container(column![header, widgets::hairline(p), widgets::wheel_scale(scroller)])
        .width(Length::Fixed(size::LIST))
        .height(Length::Fill)
        .style(theme::fill(p.list))
        .into()
}

fn list_row<'a>(c: &'a Conversation, selected: bool, p: Palette) -> Element<'a, Message> {
    let strong = c.unread > 0;
    let text_width = size::LIST - 16.0 - 8.0 - size::AVATAR - 12.0 - 12.0;
    // Built once per state (normal and hovered) by `press_row`.
    let content = move || {
        let mut bottom = row![
            text(widgets::ellipsize(&c.preview, text_width - if strong { 32.0 } else { 0.0 }, font::CAPTION))
                .size(font::CAPTION)
                .wrapping(Wrapping::None)
                .color(if strong { p.text_muted } else { p.text_faint }),
            space().width(Length::Fill),
        ]
        .align_y(CENTER);
        if strong {
            bottom = bottom.push(widgets::badge(c.unread, p));
        }
        row![
        widgets::avatar(&c.title, c.hue, size::AVATAR, Some(c.online), p),
        column![
            row![
                text(widgets::ellipsize(&c.title, text_width - 60.0, font::BODY))
                    .size(font::BODY)
                    .font(if strong { font::SEMIBOLD } else { font::REGULAR })
                    .wrapping(Wrapping::None)
                    .color(p.text),
                space().width(Length::Fill),
                text(c.time.clone()).size(font::CAPTION).color(p.text_faint),
            ]
            .align_y(CENTER),
            bottom,
        ]
        .spacing(4),
    ]
    .spacing(12)
    .align_y(CENTER)
    .padding([0, 8])
    };
    // Selects on mouse-down; hovering starts loading the conversation.
    let face = move |hovered: bool| -> Element<'a, Message> {
        container(content())
            .width(Length::Fill)
            .height(Length::Fill)
            .center_y(Length::Fill)
            .style(theme::row_fill(p, selected, hovered, radius::MD))
            .into()
    };
    container(widgets::press_row(face, Message::Select(c.id), Some(Message::Prefetch(c.id))))
        .height(Length::Fixed(size::LIST_ROW))
        .padding([2, 0])
        .into()
}

fn conversation<'a>(app: &'a App, c: &'a Conversation, p: Palette) -> Element<'a, Message> {
    let header = row![
        widgets::avatar(&c.title, c.hue, size::AVATAR_SMALL, None, p),
        column![
            text(c.title.clone()).size(font::BODY).font(font::SEMIBOLD).color(p.text),
            text(if c.online { "Active now" } else { "Away" }).size(font::CAPTION).color(if c.online { p.positive } else { p.text_faint }),
        ]
        .spacing(2),
        space().width(Length::Fill),
        widgets::icon_button(Icon::Phone, Message::Navigate(Screen::Home), p),
        widgets::icon_button(Icon::Video, Message::Navigate(Screen::Home), p),
        widgets::icon_button(Icon::More, Message::Navigate(Screen::Home), p),
    ]
    .spacing(10)
    .align_y(CENTER)
    .padding([0, 16])
    .height(Length::Fixed(size::HEADER));

    let messages: Vec<&ChatMessage> = app.store.messages_of(c.id).collect();
    let mut log = column![].padding([12, 0]);
    let mut previous: Option<&str> = None;
    for m in messages {
        let first = previous != Some(m.author.as_str());
        log = log.push(message_row(m, first, p));
        previous = Some(m.author.as_str());
    }
    if messages_empty_and_slow(app, c.id) {
        // Shown only after SLOW_AFTER (300 ms): quick loads never flash it.
        log = log.push(container(text("Loading messages…").size(font::CAPTION).color(p.text_faint)).center_x(Length::Fill).padding(24));
    }
    let log = scrollable(log).id(ids::LOG).anchor_bottom().height(Length::Fill).style(theme::thin_scrollbar(p));
    let log = widgets::wheel_scale(log);

    let ready = !app.draft.trim().is_empty();
    let send = button(container(Icon::Send.view(15.0, if ready { p.on_accent } else { p.text_faint })).center(Length::Fixed(30.0)))
        .padding(0)
        .style(move |_, _| button::Style {
            background: Some(iced::Background::Color(if ready { p.accent } else { p.hover })),
            border: iced::Border { radius: theme::corners(radius::MD), ..iced::Border::default() },
            ..button::Style::default()
        })
        .on_press_maybe(ready.then_some(Message::Send));
    let composer = container(
        row![
            Icon::Plus.view(size::ICON, p.text_muted),
            text_input(&format!("Message {}", c.title), &app.draft)
                .id(ids::COMPOSER)
                .on_input(Message::DraftChanged)
                .on_submit(Message::Send)
                .size(font::BODY)
                .padding([8, 0])
                .style(theme::bare_input(p)),
            Icon::Smile.view(size::ICON, p.text_muted),
            Icon::Paperclip.view(size::ICON, p.text_muted),
            send,
        ]
        .spacing(12)
        .align_y(CENTER)
        .padding([0, 10]),
    )
    .height(Length::Fixed(size::COMPOSER))
    .center_y(Length::Fixed(size::COMPOSER))
    .style(theme::card(p.surface, radius::LG, Some(p.border)));

    column![header, widgets::hairline(p), log, container(composer).padding(iced::Padding { top: 0.0, right: 16.0, bottom: 16.0, left: 16.0 })]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn messages_empty_and_slow(app: &App, conversation: u64) -> bool {
    app.slow == Some(conversation) && app.store.messages_of(conversation).next().is_none()
}

fn message_row<'a>(m: &'a ChatMessage, first: bool, p: Palette) -> Element<'a, Message> {
    let body = text(m.body.clone()).size(font::BODY).color(p.text);
    let indent = 16.0 + size::AVATAR_SMALL + 12.0;
    if first {
        let hue = if m.mine { 0.62 } else { 0.08 };
        container(
            row![
                widgets::avatar(&m.author, hue, size::AVATAR_SMALL, None, p),
                column![
                    row![
                        text(m.author.clone()).size(font::BODY).font(font::SEMIBOLD).color(p.text),
                        text(m.time.clone()).size(font::CAPTION).color(p.text_faint),
                    ]
                    .spacing(8)
                    .align_y(iced::Alignment::End),
                    body,
                ]
                .spacing(4),
            ]
            .spacing(12),
        )
        .padding(iced::Padding { top: 10.0, right: 24.0, bottom: 2.0, left: 16.0 })
        .into()
    } else {
        container(body).padding(iced::Padding { top: 3.0, right: 24.0, bottom: 2.0, left: indent }).into()
    }
}

fn empty<'a>(p: Palette) -> Element<'a, Message> {
    let badge = container(Icon::Chat.view(30.0, p.text_muted)).center(Length::Fixed(72.0)).style(theme::card(p.surface, 36.0, None));
    container(
        column![
            badge,
            text("No conversation selected").size(font::TITLE).font(font::SEMIBOLD).color(p.text),
            text("Pick one from the list, or start a new one.").size(font::BODY).color(p.text_muted),
        ]
        .spacing(12)
        .align_x(CENTER),
    )
    .center(Length::Fill)
    .into()
}
