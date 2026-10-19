//! System Vitals (design/specs/shell-home.md 5-6): the 104-unit top bar with
//! the three view buttons and the temperature unit switch, then the basic
//! view's tiles in a centred wrap panel (GPU, CPU, RAM, STORAGE, YOUR
//! SETTINGS, NETWORK, TOP PROCESSES), or the advanced / system info views.

use iced::widget::{column, container, mouse_area, row, space, stack};
use iced::{Background, Border, Element, Length, mouse};
use tunefast_core::sensors::{format_temp, temp_state};

use crate::app::{App, Message};
use crate::icons::Icon;
use crate::model::Vitals;
use crate::screens::Screen;
use crate::theme::{self, c, font};
use crate::ui::performance::{info, state_color};
use crate::ui::widgets::{icon, press_row, ring, unit_switch};
use crate::ui::{Board, centred, rect, t};

pub const TOP_BAR: f32 = 104.0;
const TILE: f32 = 300.0;
const TILE_H: f32 = 312.0;

pub fn view(app: &App, width: f32, height: f32) -> Element<'_, Message> {
    let area_h = (height - TOP_BAR - 20.0).max(0.0);
    let body: Element<'_, Message> = match app.screen {
        Screen::VitalsAdvanced => crate::ui::advanced::view(app, width, area_h),
        Screen::Sysinfo => crate::ui::sysinfo::view(app, width, area_h),
        _ => basic(app, width, area_h),
    };
    column![top_bar(app, width), body].into()
}

fn top_bar(app: &App, width: f32) -> Element<'_, Message> {
    let mode_button = |screen: Screen, glyph: Icon, gear: bool, x: f32| {
        let selected = app.screen == screen;
        let face = move |_hovered: bool| -> Element<'static, Message> {
            let tint = if selected { c::AREA } else { c::TEXT };
            let mut glyph_layer = Board::new(40.0, 40.0)
                .push(rect(40.0, 40.0, if selected { c::TEXT } else { c::MODE_IDLE }))
                .at(8.0, 8.0, icon(glyph, 24.0, tint));
            if gear {
                glyph_layer = glyph_layer.at(22.0, 22.0, icon(Icon::Cog, 12.0, tint));
            }
            glyph_layer.into()
        };
        (x, press_row(face, Message::Navigate(screen)))
    };
    let buttons = [
        mode_button(Screen::Vitals, Icon::Gauge, false, 50.0),
        mode_button(Screen::VitalsAdvanced, Icon::Gauge, true, 98.0),
        mode_button(Screen::Sysinfo, Icon::Info, false, 146.0),
    ];
    let fahrenheit = app.store.fahrenheit;
    let unit = |label: &'static str, on: bool, value: bool| {
        mouse_area(t(label, 14.0, font::MEDIUM, if on { c::WHITE } else { c::GREY }))
            .on_press(Message::SetFahrenheit(value))
            .interaction(mouse::Interaction::Pointer)
    };
    let temperature = row![
        t("Temperature", 16.0, font::BOOK, c::WHITE),
        space().width(Length::Fixed(16.0)),
        unit("°F", fahrenheit, true),
        space().width(Length::Fixed(8.0)),
        mouse_area(unit_switch(!fahrenheit)).on_press(Message::SetFahrenheit(!fahrenheit)).interaction(mouse::Interaction::Pointer),
        space().width(Length::Fixed(8.0)),
        unit("°C", !fahrenheit, false),
        space().width(Length::Fixed(36.0)),
        info(16.0, TEMPERATURE_TIP),
        space().width(Length::Fixed(64.0)),
    ]
    .align_y(iced::Alignment::Center);
    let mut board = Board::new(width, TOP_BAR)
        .push(rect(width, TOP_BAR, c::BANNER))
        .at(50.0, 18.0, t("Mode", 16.0, font::BOOK, c::WHITE));
    for (x, button) in buttons {
        board = board.at(x, 44.0, button);
    }
    board.push(container(temperature).width(Length::Fill).height(Length::Fill).align_x(iced::alignment::Horizontal::Right).center_y(Length::Fill)).into()
}

const TEMPERATURE_TIP: &str = "Green: Low to Moderate CPU/GPU Temperature\nOrange: High CPU/GPU Temperature\nRed: Critical CPU/GPU Temperature.";

/// The basic view: tiles in a wrap panel centred in the area (grid margin 0,22,10,0).
fn basic(app: &App, width: f32, height: f32) -> Element<'_, Message> {
    let v = &app.store.vitals;
    let usable = (width - 10.0).max(TILE);
    let slots = ((usable / TILE).floor() as usize).max(1);
    // (span, tile) in the original's order.
    let tiles: Vec<(usize, Element<'_, Message>)> = vec![
        (1, gauge_tile("GPU", v.gpu_util, Some(v.gpu_temp), "GPU Utilization", "GPU Temperature", app, v.gpu_inactive)),
        (1, gauge_tile("CPU", v.cpu_util, Some(v.cpu_temp), "CPU Utilization", "CPU Temperature", app, false)),
        (1, ram_tile(v)),
        (1, storage_tile(v)),
        (1, settings_tile(app)),
        (1, network_tile(v)),
        (2.min(slots), processes_tile(v)),
    ];
    let mut rows: Vec<Vec<(usize, Element<'_, Message>)>> = vec![Vec::new()];
    let mut used = 0;
    for (span, tile) in tiles {
        if used + span > slots && used > 0 {
            rows.push(Vec::new());
            used = 0;
        }
        used += span;
        rows.last_mut().map(|r| r.push((span, tile)));
    }
    let panel_w = rows.iter().map(|r| r.iter().map(|(s, _)| s).sum::<usize>()).max().unwrap_or(1) as f32 * TILE;
    let left = ((usable - panel_w) / 2.0).max(0.0);
    let mut board = Board::new(width, height).push(rect(width, height, c::AREA));
    for (index, line) in rows.into_iter().enumerate() {
        let mut x = left;
        for (span, tile) in line {
            board = board.at(x + 10.0, 22.0 + index as f32 * TILE_H + 10.0, tile);
            x += span as f32 * TILE;
        }
    }
    board.into()
}

/// A card (HighContrast_Button_BG2: #303030 > #212121 from the top left).
fn card<'a>(width: f32, content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .width(Length::Fixed(width))
        .height(Length::Fixed(TILE_H - 10.0))
        .style(theme::gradient_fill(theme::tile_gradient()))
        .into()
}

fn title<'a>(text: &'a str) -> Element<'a, Message> {
    t(text, 20.0, font::BOLD, c::TEXT).into()
}

fn gauge_tile<'a>(
    name: &'a str,
    util: Option<f64>,
    temp: Option<Option<f64>>,
    caption: &'a str,
    temp_caption: &'a str,
    app: &'a App,
    inactive: bool,
) -> Element<'a, Message> {
    let fahrenheit = app.store.fahrenheit;
    let percent = util.map(|u| format!("{}%", u.round() as i64)).unwrap_or_else(|| "--%".to_owned());
    let mut board = Board::new(290.0, 302.0)
        .at(18.0, 16.0, title(name))
        .at(55.0, 52.0, ring(util.unwrap_or(0.0) as f32, 180.0))
        .at(55.0, 91.6, centred(t(percent, 55.0, font::BLACK, c::TEXT), 180.0))
        .at(78.0, 163.0, centred(t(caption, 14.0, font::BOOK, c::WHITE), 134.0));
    if let Some(temp) = temp {
        board = board
            .at(18.0, 236.0, t(format_temp(temp, fahrenheit), 26.0, font::BOLD, state_color(temp_state(temp))).line_height(iced::widget::text::LineHeight::Absolute(37.0.into())))
            .at(18.0, 268.0, t(temp_caption, 14.0, font::BOOK, c::GREY80));
    }
    let face: Element<'a, Message> = board.into();
    if inactive {
        let cover = container(column![
            t("Inactive", 20.0, font::BOLD, c::WHITE),
            space().height(Length::Fixed(8.0)),
            t("Depending on the type of applications you are running, your discrete GPU may be inactive when not in use.", 14.0, font::BOOK, c::WHITE)
                .width(Length::Fixed(250.0))
                .wrapping(iced::widget::text::Wrapping::Word),
        ])
        .padding(20)
        .width(Length::Fixed(290.0))
        .height(Length::Fixed(302.0))
        .center_y(Length::Fixed(302.0))
        .style(theme::fill(iced::Color { a: 0.85, ..iced::Color::BLACK }));
        return card(290.0, stack![face, cover].into());
    }
    card(290.0, face)
}

fn ram_tile(v: &Vitals) -> Element<'_, Message> {
    let util = v.ram_util();
    let percent = util.map(|u| format!("{}%", u as i64)).unwrap_or_else(|| "--%".to_owned());
    let board = Board::new(290.0, 302.0)
        .at(18.0, 16.0, title("RAM"))
        .at(55.0, 52.0, ring(util.unwrap_or(0.0) as f32, 180.0))
        .at(55.0, 91.6, centred(t(percent, 55.0, font::BLACK, c::TEXT), 180.0))
        .at(78.0, 163.0, centred(t("RAM Utilization", 14.0, font::BOOK, c::WHITE), 134.0))
        .at(18.0, 268.0, t(format!("{:.1}GB / {:.1}GB", v.ram_used_gb, v.ram_total_gb), 14.0, font::BOOK, c::TEXT));
    card(290.0, board.into())
}

fn storage_tile(v: &Vitals) -> Element<'_, Message> {
    let mut board = Board::new(290.0, 302.0).at(16.0, 16.0, title("STORAGE"));
    for (index, drive) in v.drives.iter().take(4).enumerate() {
        let used = if drive.total_gb > 0.0 { ((drive.total_gb - drive.free_gb) / drive.total_gb).clamp(0.0, 1.0) } else { 0.0 } as f32;
        let y = 138.0 + index as f32 * 60.0;
        let bar = Board::new(155.0, 16.0)
            .push(rect(155.0, 16.0, c::TRACK))
            .at(0.0, 0.0, container(space()).width(Length::Fixed(155.0 * used)).height(Length::Fixed(16.0)).style(theme::gradient_fill(theme::omen_gradient(70.0))));
        board = board
            .at(32.0, y, t(drive.name.clone(), 14.0, font::MEDIUM, c::TEXT))
            .at(70.0, y + 1.0, bar)
            .at(70.0, y + 20.0, t(free_text(drive.free_gb, drive.total_gb), 12.0, font::BOOK, c::TEXT));
    }
    card(290.0, board.into())
}

/// "20.6 GB free of 475.6 GB" (TB above 1000 GB, as the original).
pub fn free_text(free_gb: f64, total_gb: f64) -> String {
    let part = |gb: f64| if gb >= 1000.0 { format!("{:.1} TB", gb / 1024.0) } else { format!("{gb:.1} GB") };
    format!("{} free of {}", part(free_gb), part(total_gb))
}

fn settings_tile(app: &App) -> Element<'_, Message> {
    let mode = app.store.controls.mode.name().to_uppercase();
    let button = mouse_area(
        container(t(mode, 12.0, font::BOLD, c::WHITE))
            .center_x(Length::Fixed(115.0))
            .center_y(Length::Fixed(30.0))
            .style(|_| container::Style {
                background: Some(Background::Gradient(theme::tile_gradient())),
                border: Border { color: c::RED, width: 1.0, radius: 2.0.into() },
                ..container::Style::default()
            }),
    )
    .on_press(Message::Navigate(Screen::Performance))
    .interaction(mouse::Interaction::Pointer);
    let board = Board::new(290.0, 302.0)
        .at(16.0, 16.0, title("YOUR SETTINGS"))
        .at(27.0, 70.0, t("Performance Control", 14.0, font::MEDIUM, c::TEXT))
        .at(28.0, 98.0, button);
    card(290.0, board.into())
}

fn network_tile(v: &Vitals) -> Element<'_, Message> {
    let speed = |mbps: f64| if mbps >= 100.0 { format!("{}", mbps.round() as i64) } else { format!("{mbps:.1}") };
    let big = |value: String| t(value, 50.0, font::BOLD, c::TEXT);
    let board = Board::new(290.0, 302.0)
        .at(16.0, 16.0, title("NETWORK"))
        .at(23.0, 57.0, big(speed(v.upload_mbps)))
        .at(148.5, 79.0, t("Upload Speed", 12.0, font::MEDIUM, c::TEXT))
        .at(148.5, 93.0, t("Mbps", 12.0, font::BOOK, c::TEXT))
        .at(23.0, 127.0, big(speed(v.download_mbps)))
        .at(148.5, 149.0, t("Download Speed", 12.0, font::MEDIUM, c::TEXT))
        .at(148.5, 163.0, t("Mbps", 12.0, font::BOOK, c::TEXT));
    card(290.0, board.into())
}

fn processes_tile(v: &Vitals) -> Element<'_, Message> {
    // Columns 50/180/75/75/80/100 from x 10; STOP ends the app's processes.
    let header = |text: &'static str, x: f32, width: f32| (x, centred(t(text, 14.0, font::MEDIUM, c::GREY80), width));
    let mut board = Board::new(590.0, 302.0).at(16.0, 16.0, title("TOP PROCESSES"));
    for (x, cell) in [header("APPS", 10.0, 50.0), header("NAME", 60.0, 180.0), header("CPU", 240.0, 75.0), header("GPU", 315.0, 75.0), header("RAM", 390.0, 80.0), header("ACTION", 470.0, 100.0)] {
        board = board.at(x, 68.5, cell);
    }
    for (index, process) in v.processes.iter().take(4).enumerate() {
        let top = 97.5 + index as f32 * 40.0;
        let cell = |text: String, width: f32| container(centred(t(text, 12.0, font::BOOK, c::GREY80), width)).height(Length::Fixed(40.0)).center_y(Length::Fixed(40.0));
        board = board
            .at(10.0, top, container(icon(Icon::App, 22.0, c::GREY)).center_x(Length::Fixed(50.0)).center_y(Length::Fixed(40.0)))
            .at(
                70.0,
                top,
                container(t(process.name.clone(), 12.0, font::BOOK, c::GREY80).width(Length::Fixed(165.0)).wrapping(iced::widget::text::Wrapping::Word))
                    .height(Length::Fixed(40.0))
                    .center_y(Length::Fixed(40.0)),
            )
            .at(240.0, top, cell(format!("{}%", process.cpu), 75.0))
            .at(315.0, top, cell(format!("{}%", process.gpu), 75.0))
            .at(390.0, top, cell(format!("{}MB", process.ram_mb), 80.0))
            .at(508.0, top + 8.0, stop_button(process.pids.clone()));
    }
    let task_manager = mouse_area(t("Open Task Manager", 12.0, font::BOOK, c::HYPER_GREY))
        .on_press(Message::OpenTaskManager)
        .interaction(mouse::Interaction::Pointer);
    card(590.0, board.at(440.5, 275.0, task_manager).into())
}

/// The original's STOP button: a 24-unit #d8d8d8 square with a dark "no" sign.
fn stop_button<'a>(pids: Vec<u32>) -> Element<'a, Message> {
    crate::ui::widgets::press_row(
        |hovered| crate::ui::widgets::stop_glyph(hovered),
        Message::EndProcesses(pids),
    )
}
