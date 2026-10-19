//! Small shared widgets. Each takes its colours and sizes from `theme`, so
//! restyling the app is a token change.

use std::sync::OnceLock;

use std::time::Duration;

use iced::advanced::layout::{self, Layout};
use iced::advanced::widget::{Operation, Tree, tree};
use iced::advanced::{Clipboard, Shell, Widget, overlay, renderer};
use iced::time::Instant;
use iced::widget::{button, container, hover, mouse_area, pin, rule, stack, text};
use iced::{Alignment, Color, Element, Event, Length, Rectangle, Renderer, Size, Theme, Vector, mouse, window};

use crate::app::Message;
use crate::icons::Icon;
use crate::theme::{self, Palette, font, radius, size};

/// Widget ids for focus and scroll operations (`iced::widget::operation`).
pub mod ids {
    pub const SEARCH: &str = "search";
    pub const COMPOSER: &str = "composer";
    pub const LIST: &str = "list";
    pub const LOG: &str = "log";
}

/// A row that acts the moment the mouse button goes down, as native lists,
/// tabs and sidebars do. iced's `button` acts on release, which in a normal
/// click comes 80-120 ms later; keep `button` for actions a person may want to
/// abandon by dragging off (Send, Delete, Leave). `build(hovered)` draws the
/// row in either state. `on_enter` runs when the pointer arrives: use it to
/// start loading what the row opens, so it is ready when the click lands.
pub fn press_row<'a>(build: impl Fn(bool) -> Element<'a, Message>, on_press: Message, on_enter: Option<Message>) -> Element<'a, Message> {
    let area = mouse_area(hover(build(false), build(true))).on_press(on_press).interaction(mouse::Interaction::Pointer);
    match on_enter {
        Some(message) => area.on_enter(message).into(),
        None => area.into(),
    }
}

/// Wraps a scrollable so one mouse-wheel notch scrolls `wheel_notch_px()`
/// (Chromium's 100 px) instead of iced's fixed 60, and eases it the way
/// Chromium does: part of the distance lands in the frame the wheel moved,
/// the rest settles in about 150 ms, so fast scrolling moves every frame
/// instead of jumping once per notch. It feeds pixel deltas to the scrollable,
/// so iced's own scrolling stays in charge: no message and no view rebuild per
/// notch, and `on_scroll` keeps firing for virtualized lists. Touchpad (pixel)
/// deltas pass through untouched.
pub fn wheel_scale<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    Element::new(WheelScale { content: content.into() })
}

/// Ease each notch (Chromium, Edge, Firefox and WinUI do). Set to false when
/// the original jumps a whole notch at once.
pub const SMOOTH_WHEEL: bool = true;
/// Time constant of the ease: 95% of a notch has landed after 3x this.
const WHEEL_EASE: Duration = Duration::from_millis(30);
/// Share of a notch applied in the frame the wheel moved.
const WHEEL_FIRST_STEP: f32 = 0.3;
/// Pace of the animation frames (about one 144 Hz frame), so it never spins.
const WHEEL_FRAME: Duration = Duration::from_millis(7);

/// The delta a scrollable should see for one wheel event, without easing.
pub fn scaled_delta(delta: mouse::ScrollDelta) -> mouse::ScrollDelta {
    match delta {
        mouse::ScrollDelta::Lines { x, y } => {
            let px = wheel_notch_px();
            mouse::ScrollDelta::Pixels { x: x * px, y: y * px }
        }
        pixels => pixels,
    }
}

fn wheel_event(pixels: Vector) -> Event {
    Event::Mouse(mouse::Event::WheelScrolled { delta: mouse::ScrollDelta::Pixels { x: pixels.x, y: pixels.y } })
}

/// Distance still to scroll and when the last step was applied.
#[derive(Default)]
struct WheelState {
    pending: Vector,
    last: Option<Instant>,
}

struct WheelScale<'a> {
    content: Element<'a, Message>,
}

impl Widget<Message, Theme, Renderer> for WheelScale<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<WheelState>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(WheelState::default())
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
        let state = tree.state.downcast_mut::<WheelState>();
        let child = &mut tree.children[0];
        match event {
            Event::Mouse(mouse::Event::WheelScrolled { delta: delta @ mouse::ScrollDelta::Lines { .. } }) => {
                if !SMOOTH_WHEEL {
                    let scaled = Event::Mouse(mouse::Event::WheelScrolled { delta: scaled_delta(*delta) });
                    self.content.as_widget_mut().update(child, &scaled, layout, cursor, renderer, clipboard, shell, viewport);
                    return;
                }
                if let mouse::ScrollDelta::Pixels { x, y } = scaled_delta(*delta) {
                    state.pending = state.pending + Vector::new(x, y);
                }
                // Move in this very frame; the rest eases in on the frames that follow.
                let first = state.pending * WHEEL_FIRST_STEP;
                state.pending = state.pending - first;
                state.last = Some(Instant::now());
                self.content.as_widget_mut().update(child, &wheel_event(first), layout, cursor, renderer, clipboard, shell, viewport);
                shell.request_redraw();
            }
            Event::Window(window::Event::RedrawRequested(now)) if state.last.is_some() => {
                self.content.as_widget_mut().update(child, event, layout, cursor, renderer, clipboard, shell, viewport);
                let elapsed = now.saturating_duration_since(state.last.unwrap_or(*now)).as_secs_f32();
                state.last = Some(*now);
                let share = 1.0 - (-elapsed / WHEEL_EASE.as_secs_f32()).exp();
                let step = if state.pending.x.abs() + state.pending.y.abs() < 0.5 { state.pending } else { state.pending * share };
                state.pending = state.pending - step;
                // The step goes through its own shell, so the redraw event is not
                // marked captured for the widgets after this one.
                let mut messages = Vec::new();
                let mut inner = Shell::new(&mut messages);
                self.content.as_widget_mut().update(child, &wheel_event(step), layout, cursor, renderer, clipboard, &mut inner, viewport);
                let (redraw, layout_invalid, widgets_invalid) =
                    (inner.redraw_request(), inner.is_layout_invalid(), inner.are_widgets_invalid());
                for message in messages {
                    shell.publish(message);
                }
                shell.request_redraw_at(redraw);
                if layout_invalid {
                    shell.invalidate_layout();
                }
                if widgets_invalid {
                    shell.invalidate_widgets();
                }
                if state.pending == Vector::ZERO {
                    state.last = None;
                } else {
                    shell.request_redraw_at(window::RedrawRequest::At(*now + WHEEL_FRAME));
                }
            }
            _ => self.content.as_widget_mut().update(child, event, layout, cursor, renderer, clipboard, shell, viewport),
        }
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &Renderer,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(&tree.children[0], layout, cursor, viewport, renderer)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut Renderer,
        theme: &Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
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

/// Pixels one wheel notch scrolls: Chromium's 100/3 px per line times the
/// system's lines per notch (3 by default, so 100 px), which is what users of
/// the Electron original are used to.
pub fn wheel_notch_px() -> f32 {
    static PX: OnceLock<f32> = OnceLock::new();
    *PX.get_or_init(|| 100.0 / 3.0 * wheel_lines())
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
    // u32::MAX means "one page per notch"; treat it like the default.
    if ok != 0 && (1..=100).contains(&lines) { lines as f32 } else { 3.0 }
}

#[cfg(not(windows))]
fn wheel_lines() -> f32 {
    3.0
}

/// A colour from a hue in 0..1 (avatar backgrounds without a picture).
pub fn hue_color(hue: f32, saturation: f32, value: f32) -> Color {
    let h = (hue.fract() * 6.0).max(0.0);
    let i = h.floor() as i32;
    let f = h - i as f32;
    let (p, q, t) = (value * (1.0 - saturation), value * (1.0 - saturation * f), value * (1.0 - saturation * (1.0 - f)));
    let (r, g, b) = match i % 6 {
        0 => (value, t, p),
        1 => (q, value, p),
        2 => (p, value, t),
        3 => (p, q, value),
        4 => (t, p, value),
        _ => (value, p, q),
    };
    Color::from_rgb(r, g, b)
}

/// A circle with initials and an optional presence dot.
pub fn avatar<'a>(name: &str, hue: f32, diameter: f32, online: Option<bool>, p: Palette) -> Element<'a, Message> {
    let circle = container(text(crate::model::initials(name)).size(diameter * 0.38).font(font::SEMIBOLD).color(Color::WHITE))
        .width(Length::Fixed(diameter))
        .height(Length::Fixed(diameter))
        .center(Length::Fixed(diameter))
        .style(theme::card(hue_color(hue, 0.45, 0.72), diameter / 2.0, None));
    match online {
        None => circle.into(),
        Some(online) => {
            let ring = diameter * 0.32;
            let dot = container(
                container("")
                    .width(Length::Fixed(ring * 0.66))
                    .height(Length::Fixed(ring * 0.66))
                    .style(theme::card(if online { p.positive } else { p.text_faint }, ring, None)),
            )
            .center(Length::Fixed(ring))
            .style(theme::card(p.list, ring, None));
            stack![circle, pin(dot).x(diameter - ring * 0.85).y(diameter - ring * 0.85)]
                .width(Length::Fixed(diameter))
                .height(Length::Fixed(diameter))
                .into()
        }
    }
}

/// A pill with a count.
pub fn badge<'a>(count: u32, p: Palette) -> Element<'a, Message> {
    let label = if count > 99 { "99+".to_owned() } else { count.to_string() };
    container(text(label).size(font::SECTION).font(font::SEMIBOLD).color(p.on_accent))
        .padding([2, 7])
        .style(theme::card(p.accent, radius::PILL, None))
        .into()
}

/// A square button with one icon and a hover fill.
pub fn icon_button<'a>(icon: Icon, message: Message, p: Palette) -> Element<'a, Message> {
    button(container(icon.view(size::ICON, p.text_muted)).center(Length::Fixed(size::ICON_BUTTON)))
        .padding(0)
        .width(Length::Fixed(size::ICON_BUTTON))
        .height(Length::Fixed(size::ICON_BUTTON))
        .style(theme::row_button(p, false, radius::MD))
        .on_press(message)
        .into()
}

/// A 1 px line in the border colour.
pub fn hairline<'a>(p: Palette) -> Element<'a, Message> {
    rule::horizontal(1).style(move |_| rule::Style {
        color: p.border,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: true,
    })
    .into()
}

/// Cuts `text` to roughly fit `width` at `size` points, with an ellipsis.
/// For exact ellipsis, measure with a custom widget (iced::advanced).
pub fn ellipsize(text: &str, width: f32, size: f32) -> String {
    let budget = (width / (size * 0.56)).floor().max(4.0) as usize;
    if text.chars().count() <= budget {
        return text.to_owned();
    }
    let cut: String = text.chars().take(budget.saturating_sub(1)).collect();
    format!("{}…", cut.trim_end())
}

/// Centre-aligned row spacing helper.
pub const CENTER: Alignment = Alignment::Center;
