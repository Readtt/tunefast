//! `--screenshot` and `--smoke` without a window: the views are laid out and
//! drawn offscreen by the same CPU renderer (tiny-skia) the window uses, at
//! the display's scale, so nothing appears on screen or takes the focus of
//! whoever is using the PC.

use std::path::Path;

use iced::advanced::graphics::text::font_system;
use iced::advanced::renderer::{Headless, Style};
use iced::{Pixels, Size, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};

use crate::app::App;
use crate::screens::Screen;
use crate::theme;

/// The fonts the window registers (main.rs), for offscreen text.
pub const FONTS: [&[u8]; 6] = [
    include_bytes!("../assets/fonts/Montserrat-300.ttf"),
    include_bytes!("../assets/fonts/Montserrat-400.ttf"),
    include_bytes!("../assets/fonts/Montserrat-500.ttf"),
    include_bytes!("../assets/fonts/Montserrat-700.ttf"),
    include_bytes!("../assets/fonts/Montserrat-900.ttf"),
    include_bytes!("../assets/fonts/ZenDots-400.ttf"),
];

/// RGBA pixels of the app's current screen at `points` (logical window size)
/// times the display scale.
pub fn render(app: &App, points: [f32; 2]) -> Result<(Size<u32>, Vec<u8>), String> {
    {
        let mut fonts = font_system().write().map_err(|_| "font system poisoned".to_owned())?;
        for bytes in FONTS {
            fonts.load_font(std::borrow::Cow::Borrowed(bytes));
        }
    }
    let dpi = display_scale();
    let scale = app.scale * dpi;
    let physical = Size::new((points[0] * dpi).round() as u32, (points[1] * dpi).round() as u32);
    let logical = Size::new(physical.width as f32 / scale, physical.height as f32 / scale);
    let mut renderer = futures_block_on(<iced::Renderer as Headless>::new(theme::font::BOOK, Pixels(16.0), Some("tiny-skia")))
        .ok_or_else(|| "no offscreen renderer".to_owned())?;
    let app_theme = app.theme();
    let mut interface = UserInterface::build(app.view(), logical, Cache::default(), &mut renderer);
    interface.draw(&mut renderer, &app_theme, &Style { text_color: theme::c::TEXT }, mouse::Cursor::Unavailable);
    // tiny-skia's offscreen path takes its damage area in logical units, so
    // below a scale of 1 it would leave the right and bottom edges unpainted:
    // render on a canvas large enough for that area, then crop.
    let canvas = if scale < 1.0 {
        Size::new((physical.width as f32 / scale).ceil() as u32, (physical.height as f32 / scale).ceil() as u32)
    } else {
        physical
    };
    let pixels = renderer.screenshot(canvas, scale, theme::c::WINDOW);
    if canvas == physical {
        return Ok((physical, pixels));
    }
    let mut cropped = Vec::with_capacity(physical.width as usize * physical.height as usize * 4);
    for row in pixels.chunks_exact(canvas.width as usize * 4).take(physical.height as usize) {
        cropped.extend_from_slice(&row[..physical.width as usize * 4]);
    }
    Ok((physical, cropped))
}

/// Renders and writes a PNG.
pub fn write(app: &App, points: [f32; 2], path: &Path) -> Result<Size<u32>, String> {
    let (size, pixels) = render(app, points)?;
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    image::RgbaImage::from_raw(size.width, size.height, pixels)
        .ok_or_else(|| "bad pixel buffer".to_owned())?
        .save(path)
        .map_err(|e| e.to_string())?;
    Ok(size)
}

/// Draws every screen offscreen and checks each shows more than a blank fill.
pub fn smoke(app: &mut App, points: [f32; 2]) -> Result<Vec<&'static str>, String> {
    let mut visited = Vec::new();
    for screen in Screen::ALL {
        app.screen = *screen;
        let (_, pixels) = render(app, points)?;
        let first = &pixels[..4];
        let varied = pixels.chunks_exact(4).filter(|p| *p != first).count();
        if varied < 1000 {
            return Err(format!("{} drew almost nothing ({varied} pixels differ)", screen.id()));
        }
        visited.push(screen.id());
    }
    Ok(visited)
}

/// The primary display's scale (1.25 at 120 dpi); 1 elsewhere.
fn display_scale() -> f32 {
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn GetDpiForSystem() -> u32;
            fn SetProcessDpiAwarenessContext(value: isize) -> i32;
        }
        // SAFETY: plain Win32 calls without pointers; -4 = per-monitor aware v2.
        let dpi = unsafe {
            SetProcessDpiAwarenessContext(-4);
            GetDpiForSystem()
        };
        if dpi >= 72 {
            return dpi as f32 / 96.0;
        }
    }
    1.0
}

fn futures_block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread().build().map(|runtime| runtime.block_on(future)).expect("a runtime")
}

#[cfg(test)]
mod tests {
    use iced::advanced::clipboard;
    use iced::advanced::renderer::{Headless, Style};
    use iced::{Event, Pixels, Point, mouse};
    use iced_runtime::user_interface::{Cache, UserInterface};

    use crate::app::App;
    use crate::args::Args;
    use crate::theme;

    /// One wheel notch on the advanced view moves its content as far as in the
    /// original: WPF's 48 units, scaled with the window (38.4 points measured
    /// on the original at 1280 x 800 by feel.py).
    #[test]
    fn a_wheel_notch_scrolls_as_far_as_the_original() {
        let points = [1280.0, 800.0];
        let args = <Args as clap::Parser>::parse_from(["tunefast", "--demo", "--screen", "vitals-advanced"]).normalized();
        let (app, _) = App::boot(args, None, points);
        {
            let mut fonts = super::font_system().write().unwrap();
            for bytes in super::FONTS {
                fonts.load_font(std::borrow::Cow::Borrowed(bytes));
            }
        }
        let dpi = 1.0;
        let scale = app.scale * dpi;
        let physical = iced::Size::new(1280u32, 800u32);
        let logical = iced::Size::new(1280.0 / scale, 800.0 / scale);
        let mut renderer = super::futures_block_on(<iced::Renderer as Headless>::new(theme::font::BOOK, Pixels(16.0), Some("tiny-skia"))).unwrap();
        let canvas = iced::Size::new((1280.0 / scale).ceil() as u32, (800.0 / scale).ceil() as u32);
        let style = Style { text_color: theme::c::TEXT };
        let app_theme = app.theme();
        // Over the CORE labels of the CPU clock column (design units).
        let at = Point::new(520.0, 610.0);
        let cursor = mouse::Cursor::Available(at);
        let mut ui = UserInterface::build(app.view(), logical, Cache::default(), &mut renderer);
        ui.draw(&mut renderer, &app_theme, &style, cursor);
        let before = renderer.screenshot(canvas, scale, theme::c::WINDOW);
        let mut messages = Vec::new();
        let events = [
            Event::Mouse(mouse::Event::CursorMoved { position: at }),
            Event::Mouse(mouse::Event::WheelScrolled { delta: mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 } }),
        ];
        let _ = ui.update(&events, cursor, &mut renderer, &mut clipboard::Null, &mut messages);
        ui.draw(&mut renderer, &app_theme, &style, cursor);
        let after = renderer.screenshot(canvas, scale, theme::c::WINDOW);
        let shift = vertical_shift(&before, &after, canvas.width as usize, physical, (360, 230, 460, 760));
        let points_per_notch = shift.abs() as f32 / dpi;
        let expected = crate::ui::widgets::wheel_notch_px() / 3.0 * 3.0 * app.scale;
        println!("one notch moved {points_per_notch} pt (design {} x scale {} = {expected})", crate::ui::widgets::wheel_notch_px(), app.scale);
        assert!((points_per_notch - expected).abs() <= 1.5, "moved {points_per_notch} pt, expected {expected}");
        let original = 38.4;
        assert!((0.8..=1.25).contains(&(points_per_notch / original)), "{points_per_notch} pt against the original's {original}");
    }

    /// Rows the content moved between two renders (by matching row profiles in `region`).
    fn vertical_shift(a: &[u8], b: &[u8], stride: usize, size: iced::Size<u32>, region: (usize, usize, usize, usize)) -> i32 {
        let (x0, y0, x1, y1) = region;
        let profile = |pixels: &[u8]| -> Vec<f32> {
            (y0..y1.min(size.height as usize))
                .map(|y| (x0..x1).map(|x| pixels[(y * stride + x) * 4] as f32).sum::<f32>() / (x1 - x0) as f32)
                .collect()
        };
        let (pa, pb) = (profile(a), profile(b));
        let rows = pa.len() as i32;
        let mut best = (f32::MAX, 0);
        for s in -(rows / 2)..=(rows / 2) {
            let pairs: Vec<(f32, f32)> = (0..rows)
                .filter_map(|i| {
                    let j = i - s;
                    (j >= 0 && j < rows).then(|| (pa[i as usize], pb[j as usize]))
                })
                .collect();
            if pairs.len() < rows as usize / 3 {
                continue;
            }
            let error = pairs.iter().map(|(x, y)| (x - y).abs()).sum::<f32>() / pairs.len() as f32;
            if error < best.0 - 1e-6 {
                best = (error, s);
            }
        }
        best.1
    }
}
