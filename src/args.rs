//! The command line. Besides normal use it drives the tooling the
//! fast-rebuild skill relies on: `--demo` sample data, `--screenshot` for the
//! visual diff loop and `--smoke` for the release gate.

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug, Clone)]
#[command(name = "tunefast", version, about = "A fast, native desktop app")]
pub struct Args {
    /// Run with built-in sample data: no account, no network.
    #[arg(long)]
    pub demo: bool,

    /// Load the content to show from this JSON file (a serialized `Store`)
    /// instead of the built-in sample data. Used by the visual diff loop with
    /// private fixtures that reproduce a reference screen exactly. Implies
    /// --demo.
    #[arg(long)]
    pub fixtures: Option<PathBuf>,

    /// Open on this screen (see --list-screens).
    #[arg(long)]
    pub screen: Option<String>,

    /// Window inner size in logical points, e.g. 1280x800.
    #[arg(long, value_parser = parse_size)]
    pub size: Option<[f32; 2]>,

    /// Use this theme instead of following the system.
    #[arg(long, value_enum)]
    pub theme: Option<ThemeChoice>,

    /// Write a PNG of the window once the screen has settled, then exit.
    /// Implies --demo.
    #[arg(long)]
    pub screenshot: Option<PathBuf>,

    /// Frames to draw before the screenshot, so fonts and images settle.
    #[arg(long, default_value_t = 30)]
    pub settle_frames: u32,

    /// Visit every screen, check each one draws, then exit (0 = OK).
    /// Implies --demo.
    #[arg(long)]
    pub smoke: bool,

    /// Where --smoke writes its JSON report.
    #[arg(long)]
    pub report: Option<PathBuf>,

    /// Print the screen ids and exit.
    #[arg(long)]
    pub list_screens: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum ThemeChoice {
    Dark,
    Light,
}

impl Args {
    /// Screenshots and smoke runs always use the deterministic sample data.
    pub fn normalized(mut self) -> Self {
        if self.screenshot.is_some() || self.smoke || self.fixtures.is_some() {
            self.demo = true;
        }
        self
    }

    /// Runs made by tools rather than people: no instance lock, no saved
    /// window geometry, fixed window size.
    pub fn is_tooling(&self) -> bool {
        self.screenshot.is_some() || self.smoke
    }
}

fn parse_size(text: &str) -> Result<[f32; 2], String> {
    let (width, height) = text
        .split_once(['x', 'X'])
        .ok_or_else(|| format!("expected WIDTHxHEIGHT, got {text:?}"))?;
    let width: f32 = width.trim().parse().map_err(|_| format!("bad width in {text:?}"))?;
    let height: f32 = height.trim().parse().map_err(|_| format!("bad height in {text:?}"))?;
    if width < 200.0 || height < 200.0 {
        return Err(format!("{text:?} is too small"));
    }
    Ok([width, height])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_parse() {
        assert_eq!(parse_size("1280x800"), Ok([1280.0, 800.0]));
        assert_eq!(parse_size("1024X768"), Ok([1024.0, 768.0]));
        assert!(parse_size("12x8").is_err());
        assert!(parse_size("wide").is_err());
    }

    #[test]
    fn screenshots_imply_demo() {
        let args = Args::parse_from(["tunefast", "--screenshot", "out.png"]).normalized();
        assert!(args.demo && args.is_tooling());
        let args = Args::parse_from(["tunefast"]).normalized();
        assert!(!args.demo && !args.is_tooling());
    }
}
