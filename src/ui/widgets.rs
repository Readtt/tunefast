//! Shared widgets: press-to-select rows, the wheel distance of the original,
//! and the original's controls drawn natively (utilisation ring, history
//! chart, pill toggle, AUTO/MAX slider, F/C switch, gradient border).

use std::sync::OnceLock;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget, overlay, renderer};
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, path::Arc};
use iced::widget::{container, hover, mouse_area, stack, svg, text};
use iced::{Color, Element, Event, Length, Point, Radians, Rectangle, Renderer, Size, Theme, Vector, mouse};

use crate::app::Message;
use crate::icons::Icon;
use crate::theme::{self, c, font};

/// A row, tab or menu item that acts the moment the mouse button goes down,
/// as native lists and tabs do (iced's `button` acts on release, 80-120 ms
/// later). `build(hovered)` draws either state.
pub fn press_row<'a>(build: impl Fn(bool) -> Element<'a, Message>, on_press: Message) -> Element<'a, Message> {
    mouse_area(hover(build(false), build(true))).on_press(on_press).interaction(mouse::Interaction::Pointer).into()
}

/// An SVG icon tinted with `color`, `size` design units square.
pub fn icon<'a>(icon: Icon, size: f32, color: Color) -> Element<'a, Message> {
    svg(svg::Handle::from_memory(icon.bytes()))
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .style(move |_, _| svg::Style { color: Some(color) })
        .into()
}

// ------------------------------------------------------------------- wheel

/// Wraps a scrollable so one wheel notch moves as far as in the original: a
/// WPF ScrollViewer scrolls 16 DIPs per line, 3 lines per notch by default
/// (48 design units), at once, without easing.
pub fn wheel_scale<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    Element::new(WheelScale { content: content.into() })
}

pub fn scaled_delta(delta: mouse::ScrollDelta) -> mouse::ScrollDelta {
    match delta {
        mouse::ScrollDelta::Lines { x, y } => {
            let px = wheel_notch_px();
            mouse::ScrollDelta::Pixels { x: x * px, y: y * px }
        }
        pixels => pixels,
    }
}

struct WheelScale<'a> {
    content: Element<'a, Message>,
}

impl Widget<Message, Theme, Renderer> for WheelScale<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::stateless()
    }

    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    fn size_hint(&self) -> Size<Length> {
        self.content.as_widget().size_hint()
    }

    fn layout(&mut self, tree: &mut Tree, renderer: &Renderer, limits: &layout::Limits) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    fn operate(&mut self, tree: &mut Tree, layout: Layout<'_>, renderer: &Renderer, operation: &mut dyn Operation) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        viewport: &Rectangle,
    ) {
        let child = &mut tree.children[0];
        if let Event::Mouse(mouse::Event::WheelScrolled { delta: delta @ mouse::ScrollDelta::Lines { .. } }) = event {
            let scaled = Event::Mouse(mouse::Event::WheelScrolled { delta: scaled_delta(*delta) });
            self.content.as_widget_mut().update(child, &scaled, layout, cursor, renderer, clipboard, shell, viewport);
            return;
        }
        self.content.as_widget_mut().update(child, event, layout, cursor, renderer, clipboard, shell, viewport);
    }

    fn mouse_interaction(&self, tree: &Tree, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle, renderer: &Renderer) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(&self, tree: &Tree, renderer: &mut Renderer, theme: &Theme, style: &renderer::Style, layout: Layout<'_>, cursor: mouse::Cursor, viewport: &Rectangle) {
        self.content.as_widget().draw(&tree.children[0], renderer, theme, style, layout, cursor, viewport);
    }

    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, Theme, Renderer>> {
        self.content.as_widget_mut().overlay(&mut tree.children[0], layout, renderer, viewport, translation)
    }
}

/// Design units one wheel notch scrolls: WPF's 16 per line times the system's
/// lines per notch (3 by default: 48).
pub fn wheel_notch_px() -> f32 {
    static PX: OnceLock<f32> = OnceLock::new();
    *PX.get_or_init(|| 16.0 * wheel_lines())
}

#[cfg(windows)]
fn wheel_lines() -> f32 {
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SystemParametersInfoW(action: u32, param: u32, value: *mut core::ffi::c_void, win_ini: u32) -> i32;
    }
    const SPI_GETWHEELSCROLLLINES: u32 = 0x0068;
    let mut lines: u32 = 3;
    // SAFETY: the call writes one u32 into `lines`.
    let ok = unsafe { SystemParametersInfoW(SPI_GETWHEELSCROLLLINES, 0, (&raw mut lines).cast(), 0) };
    if ok != 0 && (1..=100).contains(&lines) { lines as f32 } else { 3.0 }
}

#[cfg(not(windows))]
fn wheel_lines() -> f32 {
    3.0
}

// ------------------------------------------------------------------ canvas

/// The utilisation ring: a 3-unit stroke with the purple gradient, masked
/// counter-clockwise from 12 o'clock so the coloured part is `value`%.
pub struct Ring {
    pub value: f32,
}

impl canvas::Program<Message> for Ring {
    type State = ();

    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        let radius = bounds.width.min(bounds.height) / 2.0 - 1.5;
        let circle = Path::circle(center, radius);
        let gradient = canvas::gradient::Linear::new(Point::new(center.x, bounds.height), Point::new(center.x, 0.0))
            .add_stop(0.38, theme::hex(0xFA0FDD))
            .add_stop(1.0, theme::hex(0x8438FF));
        frame.stroke(&circle, Stroke::default().with_width(3.0).with_color(theme::hex(0xFA0FDD)));
        frame.stroke(&circle, Stroke { style: canvas::Style::Gradient(gradient.into()), width: 3.0, ..Stroke::default() });
        let masked = (100.0 - self.value.clamp(0.0, 100.0)) / 100.0;
        if masked > 0.0 {
            let top = -std::f32::consts::FRAC_PI_2;
            let arc = Path::new(|b| {
                b.arc(Arc {
                    center,
                    radius,
                    start_angle: Radians(top - masked * std::f32::consts::TAU),
                    end_angle: Radians(top),
                })
            });
            frame.stroke(&arc, Stroke::default().with_width(3.5).with_color(c::RING_MASK));
        }
        vec![frame.into_geometry()]
    }
}

pub fn ring<'a>(value: f32, size: f32) -> Element<'a, Message> {
    canvas::Canvas::new(Ring { value }).width(Length::Fixed(size)).height(Length::Fixed(size)).into()
}

/// The advanced view's history chart: 256x144, 5 columns, up to three lines.
pub struct Chart {
    pub lines: Vec<(Color, Vec<f64>, f64)>,
}

impl canvas::Program<Message> for Chart {
    type State = ();

    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), c::CHART_BG);
        let plot = Rectangle::new(Point::new(8.0, 8.0), Size::new(bounds.width - 16.0, bounds.height - 16.0));
        let border = Stroke::default().with_width(1.0).with_color(c::TRACK);
        frame.stroke(&Path::rectangle(plot.position(), plot.size()), border);
        for i in 1..5 {
            let x = plot.x + plot.width * i as f32 / 5.0;
            frame.stroke(&Path::line(Point::new(x, plot.y), Point::new(x, plot.y + plot.height)), border);
        }
        for (color, samples, range) in &self.lines {
            if samples.len() < 2 {
                continue;
            }
            let step = plot.width / 30.0;
            let path = Path::new(|b| {
                for (i, value) in samples.iter().enumerate() {
                    let y = plot.y + plot.height - (*value / range).clamp(0.0, 1.0) as f32 * plot.height;
                    let point = Point::new(plot.x + step * i as f32, y);
                    if i == 0 { b.move_to(point) } else { b.line_to(point) }
                }
            });
            frame.stroke(&path, Stroke::default().with_width(2.0).with_color(*color));
        }
        vec![frame.into_geometry()]
    }
}

pub fn chart<'a>(lines: Vec<(Color, Vec<f64>, f64)>) -> Element<'a, Message> {
    canvas::Canvas::new(Chart { lines }).width(Length::Fixed(256.0)).height(Length::Fixed(144.0)).into()
}

/// The AUTO/MAX switch: a 70.5 x 32 pill (blue at AUTO, teal at MAX) and a
/// round thumb with a snowflake.
pub struct FanSwitch {
    pub max: bool,
    pub enabled: bool,
}

impl canvas::Program<Message> for FanSwitch {
    type State = ();

    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let alpha = if self.enabled { 1.0 } else { 0.5 };
        let pill_h = 32.0;
        let top = (bounds.height - pill_h) / 2.0;
        let track = if self.max { c::TEAL } else { c::BLUE };
        let pill = Path::rounded_rectangle(Point::new(0.0, top), Size::new(bounds.width, pill_h), (pill_h / 2.0).into());
        frame.fill(&pill, Color { a: alpha, ..track });
        let cx = if self.max { bounds.width - 15.3 } else { 15.3 };
        let cy = bounds.height / 2.0;
        frame.fill(&Path::circle(Point::new(cx, cy + 1.5), 12.5), Color::from_rgba(0.0, 0.0, 0.0, 0.18 * alpha));
        frame.fill(&Path::circle(Point::new(cx, cy), 11.5), Color { a: alpha, ..c::TEXT });
        let flake = Color { a: alpha, ..theme::hex(0x5495E3) };
        let arm = 6.5;
        for i in 0..3 {
            let a = i as f32 * std::f32::consts::PI / 3.0;
            let (s, co) = a.sin_cos();
            frame.stroke(
                &Path::line(Point::new(cx - co * arm, cy - s * arm), Point::new(cx + co * arm, cy + s * arm)),
                Stroke::default().with_width(1.4).with_color(flake),
            );
        }
        vec![frame.into_geometry()]
    }
}

pub fn fan_switch<'a>(max: bool, enabled: bool) -> Element<'a, Message> {
    canvas::Canvas::new(FanSwitch { max, enabled }).width(Length::Fixed(70.5)).height(Length::Fixed(35.5)).into()
}

/// The original's pill toggle (Auto Switch to Eco Mode): 39 x 19, blue when on.
pub struct Pill {
    pub on: bool,
}

impl canvas::Program<Message> for Pill {
    type State = ();

    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let track = Path::rounded_rectangle(Point::new(0.5, 0.5), Size::new(38.0, 18.0), 9.0.into());
        if self.on {
            frame.fill(&track, c::BLUE);
            frame.stroke(&track, Stroke::default().with_width(1.0).with_color(c::BLUE));
        } else {
            frame.stroke(&track, Stroke::default().with_width(1.0).with_color(c::WHITE));
        }
        let cx = if self.on { 39.0 - 3.0 - 6.0 } else { 3.0 + 6.0 };
        frame.fill(&Path::circle(Point::new(cx, 9.5), 5.5), c::WHITE);
        vec![frame.into_geometry()]
    }
}

pub fn pill<'a>(on: bool) -> Element<'a, Message> {
    canvas::Canvas::new(Pill { on }).width(Length::Fixed(39.0)).height(Length::Fixed(19.0)).into()
}

/// The F/C switch in the System Vitals header: 32 x 13 track, white 16 thumb.
pub struct UnitSwitch {
    pub celsius: bool,
}

impl canvas::Program<Message> for UnitSwitch {
    type State = ();

    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let cy = bounds.height / 2.0;
        frame.fill(&Path::rounded_rectangle(Point::new(0.0, cy - 6.5), Size::new(32.0, 13.0), 4.0.into()), c::BASE);
        let cx = if self.celsius { 32.0 - 8.0 } else { 8.0 };
        frame.fill(&Path::circle(Point::new(cx, cy), 8.0), c::WHITE);
        frame.stroke(&Path::circle(Point::new(cx, cy), 8.0), Stroke::default().with_width(0.5).with_color(Color::BLACK));
        vec![frame.into_geometry()]
    }
}

pub fn unit_switch<'a>(celsius: bool) -> Element<'a, Message> {
    canvas::Canvas::new(UnitSwitch { celsius }).width(Length::Fixed(32.0)).height(Length::Fixed(20.0)).into()
}

/// The advanced view's expander triangle: points down when open, up when closed.
pub struct Triangle {
    pub open: bool,
}

impl canvas::Program<Message> for Triangle {
    type State = ();

    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let path = if self.open {
            Path::new(|b| {
                b.move_to(Point::new(1.0, h * 0.28));
                b.line_to(Point::new(w - 1.0, h * 0.28));
                b.line_to(Point::new(w / 2.0, h * 0.78));
                b.close();
            })
        } else {
            Path::new(|b| {
                b.move_to(Point::new(1.0, h * 0.72));
                b.line_to(Point::new(w - 1.0, h * 0.72));
                b.line_to(Point::new(w / 2.0, h * 0.22));
                b.close();
            })
        };
        frame.fill(&path, c::TEXT);
        vec![frame.into_geometry()]
    }
}

pub fn triangle<'a>(open: bool) -> Element<'a, Message> {
    canvas::Canvas::new(Triangle { open }).width(Length::Fixed(16.0)).height(Length::Fixed(16.0)).into()
}

/// The STOP glyph (24 x 24): grey square, dark ring and slash, lighter on hover.
pub struct StopGlyph {
    pub hovered: bool,
}

impl canvas::Program<Message> for StopGlyph {
    type State = ();

    fn draw(&self, _state: &(), renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let ink = if self.hovered { c::GREY_DARK } else { c::AREA };
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), c::GREY80);
        frame.stroke(&Path::rectangle(Point::new(0.5, 0.5), Size::new(bounds.width - 1.0, bounds.height - 1.0)), Stroke::default().with_width(1.0).with_color(theme::hex(0x707070)));
        let center = Point::new(bounds.width / 2.0, bounds.height / 2.0);
        frame.fill(&Path::circle(center, 8.0), ink);
        frame.fill(&Path::circle(center, 7.0), c::GREY80);
        let (d, _) = (7.5_f32 / std::f32::consts::SQRT_2, 0);
        frame.stroke(
            &Path::line(Point::new(center.x - d, center.y + d), Point::new(center.x + d, center.y - d)),
            Stroke::default().with_width(2.0).with_color(ink),
        );
        vec![frame.into_geometry()]
    }
}

pub fn stop_glyph<'a>(hovered: bool) -> Element<'a, Message> {
    canvas::Canvas::new(StopGlyph { hovered }).width(Length::Fixed(24.0)).height(Length::Fixed(24.0)).into()
}

/// A box with a `width`-unit gradient border (the original's selected card).
pub fn gradient_border<'a>(inner: Element<'a, Message>, fill: Color, width: f32, w: f32, h: f32) -> Element<'a, Message> {
    stack![
        container(text(""))
            .width(Length::Fixed(w))
            .height(Length::Fixed(h))
            .style(theme::gradient_fill(theme::omen_gradient(45.0))),
        container(container(inner).width(Length::Fill).height(Length::Fill).style(theme::fill(fill)))
            .padding(width)
            .width(Length::Fixed(w))
            .height(Length::Fixed(h)),
    ]
    .into()
}

/// A small label in the original's type (size, font, colour).
pub fn label<'a>(content: impl Into<String>, size: f32, font: iced::Font, color: Color) -> iced::widget::Text<'a> {
    text(content.into()).size(size).font(font).color(color)
}

pub fn book<'a>(content: impl Into<String>, size: f32, color: Color) -> iced::widget::Text<'a> {
    label(content, size, font::BOOK, color)
}

pub fn medium<'a>(content: impl Into<String>, size: f32, color: Color) -> iced::widget::Text<'a> {
    label(content, size, font::MEDIUM, color)
}

pub fn bold<'a>(content: impl Into<String>, size: f32, color: Color) -> iced::widget::Text<'a> {
    label(content, size, font::BOLD, color)
}
