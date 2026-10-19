//! System information (design/specs/shell-home.md 7): two equal columns of
//! blocks (MODEL, SERIAL NUMBER, OPERATING SYSTEM, CPU, GPU | RAM, AUDIO) and
//! the action bar with COPY TO CLIPBOARD and PRINT.

use iced::widget::{Column, button, column, container, row, scrollable, space};
use iced::{Element, Length};

use crate::app::{App, Message};
use crate::theme::{self, c, font};
use crate::ui::widgets::wheel_scale;
use crate::ui::{Board, rect, t};

const ACTION_BAR: f32 = 95.0;

pub fn view(app: &App, width: f32, height: f32) -> Element<'_, Message> {
    let info = &app.store.sysinfo;
    let none = || vec![Entry { name: "None".to_owned(), details: Vec::new() }];
    let plain = |value: &str| if value.trim().is_empty() { none() } else { vec![Entry { name: value.to_owned(), details: Vec::new() }] };
    let left = column![
        block("MODEL", plain(&info.model)),
        block("SERIAL NUMBER", plain(&info.serial)),
        block("OPERATING SYSTEM", vec![Entry {
            name: info.os_name.clone(),
            details: vec![format!("- Release ID: {}", info.os_release), format!("- Version: {}", info.os_version), format!("- Build: {}", info.os_build)],
        }]),
        block("CPU", vec![Entry {
            name: info.cpu_name.clone(),
            details: vec![format!("- Clock speed: {}MHz", info.cpu_clock_mhz), format!("- Description: {}", info.cpu_description)],
        }]),
        block(
            "GPU",
            info.gpus
                .iter()
                .map(|gpu| Entry {
                    name: gpu.name.clone(),
                    details: gpu
                        .refresh_hz
                        .map(|hz| format!("- Refresh rate: {hz}Hz"))
                        .into_iter()
                        .chain(std::iter::once(format!("- Driver version: {}", gpu.driver)))
                        .collect(),
                })
                .collect(),
        ),
    ];
    let right = column![
        block(
            "RAM",
            info.memory
                .iter()
                .map(|m| Entry {
                    name: m.part.clone(),
                    details: vec![format!("- Capacity: {}GB", m.capacity_gb), format!("- Type: {}", m.kind), format!("- Default clock speed: {}MHz", m.speed_mhz)],
                })
                .collect(),
        ),
        block("AUDIO", if info.audio.is_empty() { none() } else { info.audio.iter().map(|a| Entry { name: a.clone(), details: Vec::new() }).collect() }),
    ];
    let half = (width / 2.0).floor();
    let pad = iced::Padding { top: 30.0, left: 80.0, bottom: 0.0, right: 0.0 };
    let columns = row![
        container(left).padding(pad).width(Length::Fixed(half)),
        container(right).padding(pad).width(Length::Fill),
    ];
    let list_h = (height - ACTION_BAR).max(0.0);
    let backdrop = Board::new(width, list_h)
        .push(rect(width, list_h, theme::over(c::AREA, 0.7, c::WINDOW)))
        .at(half, 0.0, container(space()).width(Length::Fixed(width - half)).height(Length::Fixed(list_h)).style(theme::gradient_fill(theme::stripe_gradient())));
    let scroller = scrollable(columns)
        .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(8).scroller_width(8)))
        .style(theme::scrollbar)
        .width(Length::Fill)
        .height(Length::Fill);
    let action = |label: &'static str, message: Message| {
        button(t(label, 18.0, font::MEDIUM, c::BASE)).padding([10.0, 30.0]).height(Length::Fixed(48.0)).style(theme::action).on_press(message)
    };
    let bar = container(
        row![space().width(Length::Fill), action("COPY TO CLIPBOARD", Message::CopySysinfo), space().width(Length::Fixed(20.0)), action("PRINT", Message::PrintSysinfo), space().width(Length::Fixed(40.0))]
            .align_y(iced::Alignment::Center),
    )
    .width(Length::Fixed(width))
    .height(Length::Fixed(ACTION_BAR))
    .center_y(Length::Fixed(ACTION_BAR))
    .style(theme::fill(theme::over(c::ACTION_BAR, c::ACTION_BAR.a, c::WINDOW)));
    column![Board::new(width, list_h).push(backdrop).push(wheel_scale(scroller)), bar].into()
}

struct Entry {
    name: String,
    details: Vec<String>,
}

/// A block: bold heading, then entries (name, up to three detail lines).
fn block<'a>(heading: &'a str, entries: Vec<Entry>) -> Element<'a, Message> {
    let mut list: Column<'a, Message> = column![t(heading, 20.0, font::BOLD, c::TEXT)];
    for entry in entries {
        let mut item = column![t(entry.name, 14.0, font::MEDIUM, c::TEXT)];
        for detail in entry.details.into_iter().filter(|d| !d.trim_end().ends_with(':')) {
            item = item.push(container(t(detail, 12.0, font::MEDIUM, c::DETAIL_GREY)).padding(iced::Padding { top: 10.0, left: 25.0, ..iced::Padding::ZERO }));
        }
        list = list.push(container(item).padding(iced::Padding { top: 10.0, left: 39.0, bottom: 10.0, right: 0.0 }));
    }
    container(list).padding(iced::Padding { bottom: 32.0, ..iced::Padding::ZERO }).into()
}
