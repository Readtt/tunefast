//! Tunefast: the entry point. Parses the command line and opens the window.
//! Everything else happens in [`app::App`] (state, update, view).
//!
//! Rendering is done on the CPU (tiny-skia): no GPU driver in the process,
//! ~13 MB idle on Windows. Build with `--features gpu` for wgpu instead.

// Release builds open no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod args;
mod backend;
mod cache;
mod demo;
mod icons;
mod model;
mod screens;
mod theme;
mod ui;

use clap::Parser;

/// The name people see: window title, About, logs.
pub const APP_NAME: &str = "Tunefast";
/// Reverse-DNS identity for data folders.
#[allow(dead_code)]
pub const APP_ID: &str = "dev.fastrebuilds.tunefast";

fn main() -> iced::Result {
    let args = args::Args::parse().normalized();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,cosmic_text=warn,iced_winit=warn"))
        .init();

    if args.list_screens {
        for screen in screens::Screen::ALL {
            println!("{}\t{}", screen.id(), screen.title());
        }
        return Ok(());
    }

    // The last session, so the first frame shows real content (live runs only).
    let snapshot = if args.demo { None } else { cache::load() };
    let size = args.size.or_else(|| snapshot.as_ref().and_then(|s| s.window)).unwrap_or(theme::size::DEFAULT_WINDOW);
    let window = iced::window::Settings {
        size: iced::Size::new(size[0], size[1]),
        maximized: args.size.is_none() && snapshot.as_ref().is_some_and(|s| s.maximized),
        min_size: Some(iced::Size::new(theme::size::MIN_WINDOW[0], theme::size::MIN_WINDOW[1])),
        // The app draws its own title bar, like most apps it rebuilds.
        decorations: false,
        resizable: !args.is_tooling(),
        icon: iced::window::icon::from_file_data(include_bytes!("../assets/icon.png"), None).ok(),
        // Closing goes through App::update so the frame count can be logged.
        exit_on_close_request: false,
        ..Default::default()
    };

    let boot_args = args.clone();
    iced::application(move || app::App::boot(boot_args.clone(), snapshot.clone()), app::App::update, app::App::view)
        .title(app::App::title)
        .subscription(app::App::subscription)
        .style(app::App::style)
        .theme(app::App::theme)
        .window(window)
        .font(include_bytes!("../assets/fonts/Inter-400.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Inter-500.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Inter-600.ttf").as_slice())
        .font(include_bytes!("../assets/fonts/Inter-700.ttf").as_slice())
        .default_font(theme::font::REGULAR)
        .run()
}
