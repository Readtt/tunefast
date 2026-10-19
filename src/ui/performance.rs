//! Performance Control (design/specs/performance.md): the 1280 x 800 canvas
//! of the original's regularGridV2 inside the dimmed panel: the heading, the
//! auto switch to ECO, four mode cards, the fan speed switch and the three
//! sensor readouts (VenusRising 40 -> Zen Dots).

use iced::widget::{column, container, mouse_area, row, space, tooltip};
use iced::{Background, Border, Element, Length, mouse};
use tunefast_core::bios::Mode;
use tunefast_core::sensors::{TempState, format_temp, temp_state};

use crate::app::{App, Message};
use crate::icons::Icon;
use crate::model::HelperState;
use crate::theme::{self, c, font};
use crate::ui::widgets::{fan_switch, icon, pill, press_row};
use crate::ui::{Board, centred, rect, t};

const CANVAS: [f32; 2] = [1280.0, 800.0];
const CARD: [f32; 2] = [224.0, 120.0];
const CARD_X: [f32; 4] = [144.0, 400.0, 656.0, 912.0];

pub const DESCRIPTION: &str = "Optimize your performance by selecting the mode that works best for you.";

pub fn view(app: &App, width: f32, height: f32) -> Element<'_, Message> {
    // Outer Grid margin 32,0,32,20 with the #000000cc tint; the canvas sits at
    // the top, centred, or at the left edge when the panel is narrower (WPF
    // aligns an oversized child to the start).
    let panel_w = (width - 64.0).max(0.0);
    let panel_h = (height - 20.0).max(0.0);
    let offset = ((panel_w - CANVAS[0]) / 2.0).max(0.0);
    let canvas = container(Board::new(panel_w.max(CANVAS[0]), CANVAS[1]).at(offset, 0.0, canvas(app)))
        .width(Length::Fixed(panel_w))
        .height(Length::Fixed(panel_h))
        .clip(true)
        .style(theme::fill(c::PANEL));
    Board::new(width, height).at(32.0, 0.0, canvas).into()
}

fn canvas(app: &App) -> Element<'_, Message> {
    let controls = &app.store.controls;
    let ready = controls.helper == HelperState::Ready;
    let eco = controls.mode == Mode::Eco;
    let mut board = Board::new(CANVAS[0], CANVAS[1])
        .at(0.0, 48.0, centred(t(DESCRIPTION, 20.0, font::MEDIUM, c::TEXT).line_height(iced::widget::text::LineHeight::Absolute(24.0.into())), CANVAS[0]))
        .at(149.0, 122.0, t("Auto Switch to Eco Mode", 14.0, font::BOOK, c::WHITE))
        .at(
            339.0,
            121.5,
            mouse_area(pill(controls.auto_eco)).on_press(Message::SetAutoEco(!controls.auto_eco)).interaction(mouse::Interaction::Pointer),
        )
        .at(391.0, 123.0, t(if controls.auto_eco { "ON" } else { "OFF" }, 14.0, font::MEDIUM, c::WHITE))
        .at(437.0, 119.0, info(24.0, AUTO_ECO_TIP));
    for (index, mode) in Mode::ALL.into_iter().enumerate() {
        let selected = controls.mode == mode && (!controls.auto_eco_active || mode == Mode::Eco);
        board = board
            .at(CARD_X[index], 174.0, card(mode, selected, ready))
            .at(CARD_X[index], 308.0, t(description(mode), 16.0, font::BOOK, c::GREY).width(Length::Fixed(CARD[0])).wrapping(iced::widget::text::Wrapping::Word));
    }
    if let Some(note) = &app.notice {
        board = board.at(0.0, 452.0, centred(t(note.clone(), 14.0, font::BOOK, c::MANGO), CANVAS[0]));
    } else if !ready {
        let note = match controls.helper {
            HelperState::Unsupported => "Hardware control is not available on this computer.",
            _ => "Hardware control is off: open Settings in the menu to turn it on.",
        };
        board = board.at(0.0, 452.0, centred(t(note, 14.0, font::BOOK, c::MANGO), CANVAS[0]));
    }
    // Fan speed: separator, title, AUTO [switch] MAX, separator.
    let fan_enabled = ready && !eco;
    let label = |text: &'static str, on: bool| t(text, 14.0, font::MEDIUM, if on && fan_enabled { c::WHITE } else { c::GREY_DARK });
    let switch = mouse_area(fan_switch(controls.max_fan, fan_enabled))
        .on_press(Message::SetMaxFan(!controls.max_fan))
        .interaction(if fan_enabled { mouse::Interaction::Pointer } else { mouse::Interaction::NotAllowed });
    let fan_row = row![
        info(16.0, FAN_AUTO_TIP),
        space().width(Length::Fixed(13.0)),
        mouse_area(label("AUTO", !controls.max_fan)).on_press(Message::SetMaxFan(false)),
        space().width(Length::Fixed(12.5)),
        switch,
        space().width(Length::Fixed(13.0)),
        mouse_area(label("MAX", controls.max_fan)).on_press(Message::SetMaxFan(true)),
        space().width(Length::Fixed(13.0)),
        info(16.0, FAN_MAX_TIP),
    ]
    .align_y(iced::Alignment::Center)
    .height(Length::Fixed(35.5));
    let mut title = row![t("Fan Speed", 18.0, font::BOOK, c::GREY)].align_y(iced::Alignment::Center);
    if eco {
        title = title.push(space().width(Length::Fixed(8.0))).push(tooltip(
            icon(Icon::Warning, 20.0, c::MANGO),
            bubble(ECO_FAN_NOTE),
            tooltip::Position::Bottom,
        ));
    }
    board = board
        .at(0.0, 482.0, separator())
        .at(0.0, 519.0, container(title).center_x(Length::Fixed(CANVAS[0])))
        .at(0.0, 556.0, container(fan_row).center_x(Length::Fixed(CANVAS[0])))
        .at(0.0, 646.0, separator());
    if !app.store.vitals.fans_rpm.is_empty() {
        let speeds = app.store.vitals.fans_rpm.iter().map(|rpm| format!("{rpm} RPM")).collect::<Vec<_>>().join("   ");
        board = board.at(0.0, 604.0, centred(t(format!("Fan speed   {speeds}"), 14.0, font::BOOK, c::GREY), CANVAS[0]));
    }
    // Sensor readouts, centred, 76 apart.
    let v = &app.store.vitals;
    let fahrenheit = app.store.fahrenheit;
    // Each tile is as wide as the original's (its VenusRising value sets it).
    let tile = |value: String, color: iced::Color, name: &'static str, width: f32| -> Element<'_, Message> {
        column![
            t(value, 40.0, font::DISPLAY, color).line_height(iced::widget::text::LineHeight::Absolute(47.0.into())),
            space().height(Length::Fixed(24.0)),
            t(name, 18.0, font::BOOK, c::GREY),
        ]
        .width(Length::Fixed(width))
        .align_x(iced::Alignment::Center)
        .into()
    };
    let util = v.cpu_util.map(|u| format!("{}%", u.round() as i64)).unwrap_or_else(|| "NA".to_owned());
    let sensors = row![
        tile(format_temp(v.cpu_temp, fahrenheit), state_color(temp_state(v.cpu_temp)), "CPU Temperature", 157.0),
        space().width(Length::Fixed(76.0)),
        tile(format_temp(v.gpu_temp, fahrenheit), state_color(temp_state(v.gpu_temp)), "GPU Temperature", 157.0),
        space().width(Length::Fixed(76.0)),
        tile(util, c::TEXT, "CPU Utilization", 136.0),
    ];
    board.at(0.0, 669.0, container(sensors).center_x(Length::Fixed(CANVAS[0]))).into()
}

/// A mode card (FanModeButtonV2Style): #181818, square, the gradient border
/// and white glyph when selected or hovered; selects on mouse-down.
fn card(mode: Mode, selected: bool, ready: bool) -> Element<'static, Message> {
    let face = move |hovered: bool| -> Element<'static, Message> {
        let lit = selected || (hovered && ready);
        let glyph = match mode {
            Mode::Eco => Icon::Leaf,
            Mode::Quiet => Icon::Fan,
            Mode::Default => Icon::Layers,
            Mode::Performance => Icon::Bars,
        };
        let tint = if lit { c::TEXT } else { c::GREY };
        let name = t(mode.name(), 18.0, if lit { font::MEDIUM } else { font::BOOK }, tint);
        let inner = Board::new(CARD[0], CARD[1])
            .push(rect(CARD[0], CARD[1], c::CARD))
            .at(92.0, 20.0, icon(glyph, 40.0, tint))
            .at(0.0, 70.0, centred(name, CARD[0]));
        if lit {
            container(container(inner).padding(2.0).style(theme::gradient_fill(theme::omen_gradient(45.0))))
                .width(Length::Fixed(CARD[0]))
                .height(Length::Fixed(CARD[1]))
                .into()
        } else {
            container(inner).width(Length::Fixed(CARD[0])).height(Length::Fixed(CARD[1])).into()
        }
    };
    press_row(face, Message::SetMode(mode))
}

fn description(mode: Mode) -> &'static str {
    match mode {
        Mode::Eco => {
            "Limits system performance and power consumption to reduce heat and noise level. When switching to ECO mode, your screen might flicker briefly."
        }
        Mode::Quiet => "Keep fan noise to a minimum by reducing performance.",
        Mode::Default => "Suitable for all types of tasks.",
        Mode::Performance => "Suitable for gaming. May increase heat and noise levels.",
    }
}

pub fn state_color(state: TempState) -> iced::Color {
    match state {
        TempState::Unknown => c::TEXT,
        TempState::Normal => c::NORMAL,
        TempState::Warning => c::WARNING,
        TempState::Overheating => c::HOT,
    }
}

const AUTO_ECO_TIP: &str =
    "Enable Auto-switch to automatically activate ECO mode when AC is unplugged or when using a low-wattage adapter.";
const FAN_AUTO_TIP: &str = "Auto mode controls the fan speed based on the selected performance.";
const FAN_MAX_TIP: &str = "Max fan speed overrides the auto mode and enables maximum cooling for the selected performance.";
const ECO_FAN_NOTE: &str = "Fan speed cannot be changed in ECO mode. Switch to another mode to use MAX.";

/// The round grey "i" with its bubble (#eeeeee, 240 wide, GothamBook 14).
pub fn info<'a>(size: f32, text: &'static str) -> Element<'a, Message> {
    tooltip(info_glyph(size), bubble(text), tooltip::Position::Bottom).into()
}

pub fn info_glyph<'a>(size: f32) -> Element<'a, Message> {
    container(t("i", size * 0.72, font::BOLD, c::AREA))
        .center(Length::Fixed(size))
        .style(move |_| container::Style {
            background: Some(Background::Color(c::GREY80)),
            border: Border { radius: (size / 2.0).into(), ..Border::default() },
            ..container::Style::default()
        })
        .into()
}

fn bubble<'a>(text: &'static str) -> Element<'a, Message> {
    container(t(text, 14.0, font::BOOK, c::AREA).width(Length::Fixed(240.0)).wrapping(iced::widget::text::Wrapping::Word))
        .padding([8.0, 12.0])
        .style(theme::fill_border(c::TEXT, c::TEXT, 0.0, 4.0))
        .into()
}

fn separator<'a>() -> Element<'a, Message> {
    container(space())
        .width(Length::Fixed(CANVAS[0]))
        .height(Length::Fixed(1.0))
        .style(theme::gradient_fill(theme::separator_gradient()))
        .into()
}
