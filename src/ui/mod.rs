//! Views: functions from `&App` to `Element<Message>`. They read state and
//! emit messages; `App::update` changes state. Nothing here blocks.
//!
//! Every coordinate is in the original's design units (WPF DIPs on its
//! 1624 x 940 canvas, design/spec.md); iced scales the whole window.

pub mod advanced;
pub mod performance;
pub mod shell;
pub mod sysinfo;
pub mod vitals;
pub mod widgets;

use iced::widget::text::{LineHeight, Wrapping};
use iced::widget::{Stack, Text, container, pin, space, text};
use iced::{Color, Element, Font, Length};

use crate::app::Message;
use crate::theme;

/// Text in the original's metrics: Gotham lines are 1.2 x the font size.
pub fn t<'a>(content: impl text::IntoFragment<'a>, size: f32, font: Font, color: Color) -> Text<'a> {
    text(content).size(size).font(font).color(color).line_height(LineHeight::Relative(1.2)).wrapping(Wrapping::None)
}

/// A board of fixed (or filling) size whose children sit at design coordinates.
pub struct Board<'a> {
    stack: Stack<'a, Message>,
}

impl<'a> Board<'a> {
    pub fn new(width: impl Into<Length>, height: impl Into<Length>) -> Self {
        let (width, height) = (width.into(), height.into());
        Board { stack: Stack::new().width(width).height(height).push(space().width(width).height(height)) }
    }

    pub fn at(mut self, x: f32, y: f32, content: impl Into<Element<'a, Message>>) -> Self {
        self.stack = self.stack.push(pin(content).x(x).y(y));
        self
    }

    pub fn push(mut self, content: impl Into<Element<'a, Message>>) -> Self {
        self.stack = self.stack.push(content);
        self
    }
}

impl<'a> From<Board<'a>> for Element<'a, Message> {
    fn from(board: Board<'a>) -> Self {
        board.stack.into()
    }
}

/// A plain filled rectangle.
pub fn rect<'a>(width: f32, height: f32, color: Color) -> Element<'a, Message> {
    container(space()).width(Length::Fixed(width)).height(Length::Fixed(height)).style(theme::fill(color)).into()
}

/// A text box of fixed width, its text centred.
pub fn centred<'a>(content: Text<'a>, width: f32) -> Element<'a, Message> {
    content.width(Length::Fixed(width)).align_x(iced::alignment::Horizontal::Center).into()
}
