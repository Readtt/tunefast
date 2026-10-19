//! Tunefast: the entry point. Parses the command line and opens the window.
//! Everything else happens in [`app::App`] (state, update, view).
//!
//! Rendering is done on the CPU (tiny-skia): no GPU driver in the process.
//! The window is laid out on the original's 1624 x 940 design canvas and
//! scaled to the window, as the original does (`App::scale_factor`).

// Release builds open no console window.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod args;
mod cache;
mod demo;
mod eco;
mod hw;
mod icons;
mod model;
mod screens;
mod settings;
mod shot;
mod theme;
mod ui;

use clap::Parser;

/// The name people see: window title, About, logs.
pub const APP_NAME: &str = "Tunefast";
/// Data folder name.
pub const APP_ID: &str = "Tunefast";

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

    // Screenshots and smoke runs draw offscreen: no window, no focus taken.
    if args.screenshot.is_some() || args.smoke {
        let points = args.size.unwrap_or(theme::size::DEFAULT_WINDOW);
        let (mut app, _) = app::App::boot(args.clone(), None, points);
        if let Some(path) = &args.screenshot {
            match shot::write(&app, points, path) {
                Ok(size) => {
                    println!("SCREENSHOT {}x{} {}", size.width, size.height, path.display());
                    return Ok(());
                }
                Err(error) => {
                    eprintln!("screenshot failed: {error}");
                    std::process::exit(4);
                }
            }
        }
        let result = shot::smoke(&mut app, points);
        let report = serde_json::json!({
            "ok": result.is_ok(),
            "detail": result.as_ref().err().cloned().unwrap_or_else(|| "ok".to_owned()),
            "visited": result.as_ref().cloned().unwrap_or_default(),
            "expected": screens::Screen::ALL.iter().map(|s| s.id()).collect::<Vec<_>>(),
            "version": env!("CARGO_PKG_VERSION"),
        });
        if let Some(path) = &args.report {
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(path, report.to_string());
        }
        match result {
            Ok(visited) => {
                println!("SMOKE OK {} screens: {}", visited.len(), visited.join(", "));
                return Ok(());
            }
            Err(error) => {
                eprintln!("SMOKE FAILED {error}");
                std::process::exit(1);
            }
        }
    }

    // The last session, so the first frame shows real numbers (live runs only).
    let snapshot = if args.demo { None } else { cache::load() };
    let points = args.size.or_else(|| snapshot.as_ref().and_then(|s| s.window)).unwrap_or(theme::size::DEFAULT_WINDOW);
    // The original opens maximized; later starts reopen as the window was left.
    let maximized = args.size.is_none() && snapshot.as_ref().is_none_or(|s| s.maximized);
    // Window sizes are given to iced in design units (it multiplies them by the
    // scale factor the app reports for its first frame).
    let scale = theme::window_scale(points[0], points[1]);
    let window = iced::window::Settings {
        size: iced::Size::new(points[0] / scale, points[1] / scale),
        maximized,
        min_size: Some(iced::Size::new(theme::size::MIN_WINDOW[0] / scale, theme::size::MIN_WINDOW[1] / scale)),
        // The app draws its own title bar, as the original does.
        decorations: false,
        resizable: !args.is_tooling(),
        icon: iced::window::icon::from_file_data(include_bytes!("../assets/icon.png"), None).ok(),
        // Closing goes through App::update so the frame count can be logged.
        exit_on_close_request: false,
        ..Default::default()
    };

    let boot_args = args.clone();
    iced::application(
        move || app::App::boot(boot_args.clone(), snapshot.clone(), [points[0], points[1]]),
        app::App::update,
        app::App::view,
    )
    .title(app::App::title)
    .subscription(app::App::subscription)
    .style(app::App::style)
    .theme(app::App::theme)
    .scale_factor(app::App::scale_factor)
    .window(window)
    .font(shot::FONTS[0])
    .font(shot::FONTS[1])
    .font(shot::FONTS[2])
    .font(shot::FONTS[3])
    .font(shot::FONTS[4])
    .font(shot::FONTS[5])
    .default_font(theme::font::BOOK)
    .run()
}
