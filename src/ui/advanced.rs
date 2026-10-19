//! System Vitals, advanced view (design/specs/shell-home.md 8): CPU, GPU, RAM
//! and NETWORK sections over striped columns (401 / 272 / 261 / *), each with
//! up to three data columns and a 256 x 144 history chart. Per-core CPU
//! temperatures and CPU power are not shown: the original reads them through
//! its kernel driver.

use iced::widget::{Column, column, container, mouse_area, row, scrollable, space};
use iced::{Color, Element, Length, mouse};

use crate::app::{App, Message, Section};
use crate::model::Advanced;
use crate::theme::{self, c, font};
use crate::ui::widgets::{chart, wheel_scale};
use crate::ui::{Board, rect, t};
use tunefast_core::sensors::c_to_f;

const COLUMNS: [f32; 3] = [401.0, 272.0, 261.0];
const DATA_LEFT: f32 = 152.0;
const DATA_COLUMNS: [f32; 3] = [249.0, 272.0, 261.0];

pub fn view(app: &App, width: f32, height: f32) -> Element<'_, Message> {
    let a = &app.store.advanced;
    let mut sections: Column<'_, Message> = column![].spacing(32);
    sections = sections.push(section(app, Section::Cpu, "CPU", cpu(a, app.store.fahrenheit)));
    if !a.gpu.absent {
        sections = sections.push(section(app, Section::Gpu, "GPU", gpu(a, app.store.fahrenheit)));
    }
    sections = sections.push(section(app, Section::Ram, "RAM", ram(a)));
    sections = sections.push(section(app, Section::Network, "NETWORK", network(a)));
    let content = container(sections).padding(iced::Padding { top: 30.0, left: 56.0, bottom: 40.0, right: 0.0 }).width(Length::Fill);
    // Layer 1: #121212 at 0.7 over the window; layer 2: stripes in columns 2 and 4.
    let stripe_x = COLUMNS[0] + COLUMNS[1] + COLUMNS[2];
    let backdrop = Board::new(width, height)
        .push(rect(width, height, theme::over(c::AREA, 0.7, c::WINDOW)))
        .at(COLUMNS[0], 0.0, stripe(COLUMNS[1], height))
        .at(stripe_x, 0.0, stripe((width - stripe_x).max(0.0), height));
    let scroller = scrollable(content)
        .direction(scrollable::Direction::Vertical(scrollable::Scrollbar::new().width(8).scroller_width(8)))
        .style(theme::scrollbar)
        .width(Length::Fill)
        .height(Length::Fill);
    container(Board::new(width, height).push(backdrop).push(wheel_scale(scroller))).width(Length::Fixed(width)).height(Length::Fixed(height)).into()
}

fn stripe<'a>(width: f32, height: f32) -> Element<'a, Message> {
    container(space()).width(Length::Fixed(width)).height(Length::Fixed(height)).style(theme::gradient_fill(theme::stripe_gradient())).into()
}

/// A section: the expander header (triangle + bold title), then its body.
fn section<'a>(app: &'a App, which: Section, title: &'a str, body: Element<'a, Message>) -> Element<'a, Message> {
    let open = !app.collapsed.contains(&which);
    let header = mouse_area(
        row![
            crate::ui::widgets::triangle(open),
            space().width(Length::Fixed(16.0)),
            t(title, 20.0, font::BOLD, c::TEXT).line_height(iced::widget::text::LineHeight::Absolute(24.0.into())),
        ]
        .align_y(iced::Alignment::Center)
        .height(Length::Fixed(24.0)),
    )
    .on_press(Message::ToggleSection(which))
    .interaction(mouse::Interaction::Pointer);
    if open { column![header, body].into() } else { header.into() }
}

/// The body: the item name, then the data grid with the chart in column 4.
fn body<'a>(name: String, columns: [Option<Element<'a, Message>>; 3], chart_lines: Vec<(Color, Vec<f64>, f64)>, legend: Vec<(Color, &'a str)>) -> Element<'a, Message> {
    let mut grid = row![].align_y(iced::Alignment::Start);
    for (index, column) in columns.into_iter().enumerate() {
        let cell = container(column.unwrap_or_else(|| space().into())).width(Length::Fixed(DATA_COLUMNS[index]));
        grid = grid.push(cell);
    }
    let mut legend_column = column![].spacing(8);
    for (color, label) in legend {
        legend_column = legend_column.push(row![rect(12.0, 12.0, color), space().width(Length::Fixed(16.0)), t(label, 12.0, font::MEDIUM, c::TEXT)].align_y(iced::Alignment::Center));
    }
    let chart_block = column![chart(chart_lines), space().height(Length::Fixed(16.0)), row![space().width(Length::Fixed(16.0)), legend_column]];
    grid = grid.push(container(chart_block).width(Length::Fill).center_x(Length::Fill));
    column![
        container(t(name, 14.0, font::MEDIUM, c::TEXT)).padding(iced::Padding { top: 16.0, left: 72.0, ..iced::Padding::ZERO }),
        container(grid).padding(iced::Padding { top: 17.0, left: DATA_LEFT - 56.0, ..iced::Padding::ZERO }),
    ]
    .into()
}

/// A data column: header with its colour square, then name/value rows.
fn data<'a>(header: &'a str, color: Option<Color>, rows: Vec<(String, String)>, name_w: f32, value_w: f32, line: f32) -> Column<'a, Message> {
    let mut head = row![t(header, 12.0, font::MEDIUM, c::TEXT)].align_y(iced::Alignment::Center);
    if let Some(color) = color {
        head = head.push(space().width(Length::Fixed(8.0))).push(rect(12.0, 12.0, color));
    }
    let mut list = column![head, space().height(Length::Fixed(5.0))];
    for (name, value) in rows {
        list = list.push(
            row![
                t(name, 12.0, font::MEDIUM, c::HYPER_GREY).width(Length::Fixed(name_w)).line_height(iced::widget::text::LineHeight::Absolute(line.into())),
                space().width(Length::Fixed(10.0)),
                t(value, 12.0, font::MEDIUM, c::HYPER_GREY).width(Length::Fixed(value_w)).line_height(iced::widget::text::LineHeight::Absolute(line.into())),
            ]
            .padding(iced::Padding { top: 6.0, ..iced::Padding::ZERO }),
        );
    }
    list
}

fn temp_text(celsius: Option<f64>, fahrenheit: bool) -> String {
    match celsius {
        Some(c) if c > 0.0 => {
            if fahrenheit { format!("{} °F", c_to_f(c).round() as i64) } else { format!("{} °C", c.round() as i64) }
        }
        _ => "--".to_owned(),
    }
}

fn temp_range(fahrenheit: bool) -> f64 {
    if fahrenheit { 212.0 } else { 100.0 }
}

fn history_in_unit(values: &[f64], fahrenheit: bool) -> Vec<f64> {
    values.iter().map(|v| if fahrenheit { c_to_f(*v) } else { *v }).collect()
}

fn cpu(a: &Advanced, fahrenheit: bool) -> Element<'_, Message> {
    let clocks: Vec<(String, String)> = a.cpu.clocks_mhz.iter().enumerate().map(|(i, mhz)| (format!("Core {}", i + 1), format!("{mhz:.1} MHz"))).collect();
    let mut utils = vec![("Package".to_owned(), format!("{:.1} %", a.cpu.package_util))];
    utils.extend(a.cpu.utils.iter().enumerate().map(|(i, u)| (format!("Core {}", i + 1), format!("{u:.1} %"))));
    let temps = vec![("Package".to_owned(), temp_text(a.cpu.package_temp, fahrenheit))];
    body(
        a.cpu.name.clone(),
        [
            Some(data("Clock", Some(c::CLOCK), clocks, 117.0, 122.0, 18.0).into()),
            Some(container(data("Temperature", Some(c::TEMP), temps, 121.0, 93.0, 14.4)).padding(iced::Padding { left: 48.0, ..iced::Padding::ZERO }).into()),
            Some(container(data("Utilization", Some(c::UTIL), utils, 117.0, 86.0, 14.4)).padding(iced::Padding { left: 48.0, ..iced::Padding::ZERO }).into()),
        ],
        vec![
            (c::CLOCK, a.cpu.clock_history.clone(), 10000.0),
            (c::TEMP, history_in_unit(&a.cpu.temp_history, fahrenheit), temp_range(fahrenheit)),
            (c::UTIL, a.cpu.util_history.clone(), 100.0),
        ],
        vec![(c::CLOCK, "Clock"), (c::TEMP, "Temperature"), (c::UTIL, "Utilization")],
    )
}

fn gpu(a: &Advanced, fahrenheit: bool) -> Element<'_, Message> {
    let g = &a.gpu;
    let available = (g.memory_total_gb - g.memory_used_gb).max(0.0);
    let memory_util = if g.memory_total_gb > 0.0 { g.memory_used_gb / g.memory_total_gb * 100.0 } else { 0.0 };
    let clock = data(
        "Clock",
        Some(c::CLOCK),
        vec![("Core".to_owned(), format!("{:.1} MHz", g.core_clock_mhz)), ("Memory".to_owned(), format!("{:.1} MHz", g.memory_clock_mhz))],
        117.0,
        122.0,
        18.0,
    );
    let memory = data(
        "Memory",
        None,
        vec![
            ("In use".to_owned(), format!("{:.1} GB", g.memory_used_gb)),
            ("Available".to_owned(), format!("{available:.1} GB")),
            ("Total".to_owned(), format!("{:.1} GB", g.memory_total_gb)),
            ("Utilization".to_owned(), format!("{memory_util:.1} %")),
        ],
        117.0,
        122.0,
        18.0,
    );
    body(
        g.name.clone(),
        [
            Some(column![clock, space().height(Length::Fixed(16.0)), memory].into()),
            Some(container(data("Temperature", Some(c::TEMP), vec![("Core".to_owned(), temp_text(g.temp, fahrenheit))], 121.0, 93.0, 14.4)).padding(iced::Padding { left: 48.0, ..iced::Padding::ZERO }).into()),
            Some(
                container(data(
                    "Utilization",
                    Some(c::UTIL),
                    vec![
                        ("Core".to_owned(), format!("{:.1} %", g.util_core)),
                        ("Frame buffer".to_owned(), format!("{:.1} %", g.util_frame_buffer)),
                        ("Video Engine".to_owned(), format!("{:.1} %", g.util_video)),
                        ("Bus".to_owned(), format!("{:.1} %", g.util_bus)),
                    ],
                    117.0,
                    86.0,
                    14.4,
                ))
                .padding(iced::Padding { left: 48.0, ..iced::Padding::ZERO })
                .into(),
            ),
        ],
        vec![
            (c::CLOCK, g.clock_history.clone(), 2000.0),
            (c::TEMP, history_in_unit(&g.temp_history, fahrenheit), temp_range(fahrenheit)),
            (c::UTIL, g.util_history.clone(), 100.0),
        ],
        vec![(c::CLOCK, "Clock"), (c::TEMP, "Temperature"), (c::UTIL, "Utilization")],
    )
}

fn ram(a: &Advanced) -> Element<'_, Message> {
    let r = &a.ram;
    let util = if r.total_gb > 0.0 { r.used_gb / r.total_gb * 100.0 } else { 0.0 };
    body(
        "Total".to_owned(),
        [
            Some(
                data(
                    "Utilization",
                    Some(c::CLOCK),
                    vec![
                        ("In use".to_owned(), format!("{:.1} GB", r.used_gb)),
                        ("Available".to_owned(), format!("{:.1} GB", (r.total_gb - r.used_gb).max(0.0))),
                        ("Total".to_owned(), format!("{:.1} GB", r.total_gb)),
                        ("Utilization".to_owned(), format!("{util:.1} %")),
                    ],
                    117.0,
                    122.0,
                    18.0,
                )
                .into(),
            ),
            None,
            None,
        ],
        vec![(c::CLOCK, r.util_history.clone(), 100.0)],
        vec![(c::CLOCK, "Utilization")],
    )
}

fn network(a: &Advanced) -> Element<'_, Message> {
    let n = &a.network;
    body(
        "Total ".to_owned(),
        [
            Some(data("Upload Speed", Some(c::CLOCK), vec![("Upload Speed".to_owned(), format!("{:.1} Mbps", n.upload_mbps))], 117.0, 122.0, 18.0).into()),
            Some(
                container(data("Download Speed", Some(c::TEMP), vec![("Download Speed".to_owned(), format!("{:.1} Mbps", n.download_mbps))], 121.0, 93.0, 14.4))
                    .padding(iced::Padding { left: 48.0, ..iced::Padding::ZERO })
                    .into(),
            ),
            None,
        ],
        vec![(c::CLOCK, n.upload_history.clone(), 100.0), (c::TEMP, n.download_history.clone(), 100.0)],
        vec![(c::CLOCK, "Upload Speed"), (c::TEMP, "Download Speed")],
    )
}
