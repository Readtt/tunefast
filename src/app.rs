//! Application state, messages and the update loop (the Elm architecture):
//! views emit `Message`s, `update` changes state and returns `Task`s, and
//! nothing in the UI path blocks: firmware requests, display changes and
//! sensor reads run on background threads.
//!
//! Built to feel instant (references/feel.md): tabs, menu rows and mode cards
//! act on mouse-down, a chosen mode or fan setting shows at once and is
//! confirmed in the background, the first frame draws the last session's
//! numbers, and sensors run only while the window is open.

use std::cell::Cell;
use std::collections::HashSet;
use std::time::{Duration, Instant};

use iced::{Element, Size, Subscription, Task, Theme, window};
use tunefast_core::bios::{self, Mode};
use tunefast_core::protocol::{Request, Response};

use crate::args::{Args, ThemeChoice};
use crate::cache::Snapshot;
use crate::hw::Reading;
use crate::model::{HelperState, Store, SysInfo};
use crate::screens::Screen;
use crate::settings::{EcoSaved, Settings};
use crate::theme::{self, Palette, size};

/// The advanced view's collapsible sections.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Section {
    Cpu,
    Gpu,
    Ram,
    Network,
}

/// What a background firmware request came back with.
#[derive(Debug, Clone)]
pub enum HelperResult {
    Hello(bool),
    MaxFan(Option<bool>),
    ModeSet { mode: Mode, ok: bool },
    MaxFanSet { on: bool, ok: bool },
    Adapter { low_wattage: bool },
}

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Screen),
    SetMode(Mode),
    SetMaxFan(bool),
    SetAutoEco(bool),
    SetFahrenheit(bool),
    SetLight(bool),
    ToggleSettings,
    ToggleMenu,
    ToggleSection(Section),
    CopySysinfo,
    PrintSysinfo,
    OpenTaskManager,
    /// STOP on a TOP PROCESSES row: end that app's processes (ids, executables).
    EndProcesses(Vec<u32>, Vec<String>),
    EnableHelper,
    Reading(Box<Reading>),
    SysInfo(Box<SysInfo>),
    Helper(HelperResult),
    EcoApplied(Option<EcoSaved>),
    PowerTick,
    Power { on_ac: bool },
    Saved(Result<(), String>),
    Resized(Size),
    Maximized(bool),
    DragWindow,
    ToggleMaximize,
    Minimize,
    CloseRequested,
    ResizeWindow(window::Direction),
}

pub struct App {
    pub screen: Screen,
    /// The System Vitals view the tab returns to (basic, advanced, info).
    pub vitals_screen: Screen,
    pub store: Store,
    pub settings: Settings,
    pub collapsed: HashSet<Section>,
    pub settings_open: bool,
    /// Below the menu breakpoint the menu opens over the content.
    pub menu_open: bool,
    /// A short note under the mode cards (firmware refused, helper missing).
    pub notice: Option<String>,
    /// The user picked a mode while auto switch had ECO on: stay on it.
    pub eco_overridden: bool,
    pub on_ac: bool,
    pub low_wattage: bool,
    /// Window size in points, its scale (the original's rule) and the scale
    /// iced last applied (resize events arrive in those units).
    pub window_points: Size,
    pub scale: f32,
    applied_scale: Cell<f32>,
    pub maximized: bool,
    minimized: bool,
    /// Live runs talk to the hardware; demo, fixtures and tooling never do.
    live: bool,
    /// Views built since start, logged on exit (an idle app builds none).
    renders: Cell<u64>,
    started: Instant,
}

impl App {
    pub fn boot(args: Args, snapshot: Option<Snapshot>, points: [f32; 2]) -> (App, Task<Message>) {
        let mut tasks = Vec::new();
        let live = !args.demo;
        let mut settings = if live { crate::settings::load() } else { Settings::default() };
        let (mut store, last_screen) = if let Some(path) = &args.fixtures {
            // A reference captured at another window size showed other live
            // numbers: `<id>@WxH.json` next to the fixture holds those.
            let sized = args.size.map(|[w, h]| path.with_file_name(format!(
                "{}@{}x{}.json",
                path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                w as u32,
                h as u32
            )));
            let path = sized.filter(|p| p.exists()).unwrap_or_else(|| path.clone());
            let store = std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|text| serde_json::from_str::<Store>(&text).map_err(|e| e.to_string()))
                .unwrap_or_else(|error| {
                    log::error!("could not load fixtures {}: {error}", path.display());
                    crate::demo::store()
                });
            (store, None)
        } else if args.demo {
            (crate::demo::store(), None)
        } else {
            match &snapshot {
                Some(last) => (last.store.clone(), Screen::from_id(&last.screen)),
                None => (Store::default(), None),
            }
        };
        if live {
            store.controls.mode = settings.mode;
            store.controls.max_fan = settings.max_fan;
            store.controls.auto_eco = settings.auto_eco;
            store.controls.helper = if cfg!(windows) { HelperState::Missing } else { HelperState::Unsupported };
            store.fahrenheit = settings.fahrenheit;
            store.version = env!("CARGO_PKG_VERSION").to_owned();
            tasks.push(blocking(|| crate::hw::ask_helper(Request::Hello), |r| Message::Helper(HelperResult::Hello(r.ok))));
            tasks.push(blocking(crate::hw::read_system_info, |info| Message::SysInfo(Box::new(info))));
            tasks.push(Task::done(Message::PowerTick));
        } else {
            settings.fahrenheit = store.fahrenheit;
        }
        if store.version.is_empty() {
            store.version = env!("CARGO_PKG_VERSION").to_owned();
        }
        if store.device.is_empty() {
            store.device = crate::model::device_label(&store.sysinfo.model);
        }
        match args.theme {
            Some(ThemeChoice::Light) => settings.light = true,
            Some(ThemeChoice::Dark) => settings.light = false,
            None => {}
        }
        let screen = args.screen.as_deref().and_then(Screen::from_id).or(last_screen).unwrap_or(Screen::Performance);
        let scale = theme::window_scale(points[0], points[1]);
        crate::hw::show_page(screen);
        let app = App {
            screen,
            vitals_screen: if screen.is_vitals() { screen } else { Screen::Vitals },
            store,
            settings,
            collapsed: HashSet::new(),
            settings_open: false,
            menu_open: false,
            notice: None,
            eco_overridden: false,
            on_ac: true,
            low_wattage: false,
            window_points: Size::new(points[0], points[1]),
            scale,
            applied_scale: Cell::new(scale),
            maximized: args.size.is_none() && snapshot.as_ref().is_none_or(|s| s.maximized),
            minimized: false,
            live,
            renders: Cell::new(0),
            started: Instant::now(),
        };
        (app, Task::batch(tasks))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(screen) => {
                self.screen = screen;
                crate::hw::show_page(screen);
                if screen.is_vitals() {
                    self.vitals_screen = screen;
                }
                self.menu_open = false;
                self.settings_open = false;
            }
            Message::SetMode(mode) => return self.set_mode(mode, false),
            Message::SetMaxFan(on) => {
                if self.store.controls.mode == Mode::Eco || !self.controls_ready() {
                    return Task::none();
                }
                // Optimistic: the switch moves now; the firmware confirms.
                self.store.controls.max_fan = on;
                self.settings.max_fan = on;
                return Task::batch([
                    self.save_settings(),
                    blocking(move || crate::hw::ask_helper(Request::SetMaxFan { on }), move |r| {
                        Message::Helper(HelperResult::MaxFanSet { on, ok: r.ok && r.return_code == Some(0) })
                    }),
                ]);
            }
            Message::SetAutoEco(on) => {
                self.store.controls.auto_eco = on;
                self.settings.auto_eco = on;
                self.eco_overridden = false;
                let mut tasks = vec![self.save_settings()];
                tasks.push(self.check_auto_eco());
                return Task::batch(tasks);
            }
            Message::SetFahrenheit(on) => {
                self.store.fahrenheit = on;
                self.settings.fahrenheit = on;
                return self.save_settings();
            }
            Message::SetLight(on) => {
                self.settings.light = on;
                return self.save_settings();
            }
            Message::ToggleSettings => self.settings_open = !self.settings_open,
            Message::ToggleMenu => self.menu_open = !self.menu_open,
            Message::ToggleSection(section) => {
                if !self.collapsed.remove(&section) {
                    self.collapsed.insert(section);
                }
            }
            Message::CopySysinfo => return iced::clipboard::write(sysinfo_text(&self.store.sysinfo)),
            Message::PrintSysinfo => {
                let text = sysinfo_text(&self.store.sysinfo);
                return Task::perform(
                    tokio::task::spawn_blocking(move || crate::hw::print_text(&text)),
                    |_| Message::Saved(Ok(())),
                );
            }
            Message::OpenTaskManager => crate::hw::open_task_manager(),
            Message::EndProcesses(pids, exes) => {
                if self.live && !pids.is_empty() {
                    self.store.vitals.processes.retain(|p| p.pids != pids);
                    return Task::perform(tokio::task::spawn_blocking(move || crate::hw::end_processes(&pids, &exes)), |_| Message::Saved(Ok(())));
                }
            }
            Message::EnableHelper => {
                if self.live {
                    return blocking(crate::hw::install_helper, |ok| Message::Helper(HelperResult::Hello(ok)));
                }
            }
            Message::Reading(reading) => {
                let reading = *reading;
                self.store.vitals = reading.vitals;
                self.store.advanced = reading.advanced;
                if reading.on_ac != self.on_ac {
                    return self.update(Message::Power { on_ac: reading.on_ac });
                }
            }
            Message::SysInfo(info) => {
                if self.store.device.is_empty() || self.store.device == "This Laptop" || self.store.sysinfo.model != info.model {
                    self.store.device = crate::model::device_label(&info.model);
                }
                self.store.sysinfo = *info;
            }
            Message::Helper(result) => return self.helper_result(result),
            Message::EcoApplied(saved) => {
                self.settings.eco_saved = saved;
                return self.save_settings();
            }
            Message::PowerTick => {
                if self.live {
                    return blocking(crate::hw::on_ac, |on_ac| Message::Power { on_ac });
                }
            }
            Message::Power { on_ac } => {
                let changed = on_ac != self.on_ac;
                self.on_ac = on_ac;
                if changed {
                    self.eco_overridden = false;
                }
                let mut tasks = vec![self.check_auto_eco()];
                if self.live && self.store.controls.helper == HelperState::Ready {
                    tasks.push(blocking(
                        || crate::hw::ask_helper(Request::GetAdapterStatus),
                        |r: Response| Message::Helper(HelperResult::Adapter { low_wattage: r.ok && bios::adapter_status(&r.data).low_wattage() }),
                    ));
                }
                return Task::batch(tasks);
            }
            Message::Saved(result) => {
                if let Err(error) = result {
                    log::warn!("could not save: {error}");
                }
            }
            Message::Resized(size) => {
                if size.width <= 1.0 || size.height <= 1.0 {
                    // Minimized: the sensor subscription stops with this.
                    self.minimized = true;
                    return Task::none();
                }
                self.minimized = false;
                let applied = self.applied_scale.get();
                self.window_points = Size::new(size.width * applied, size.height * applied);
                self.scale = theme::window_scale(self.window_points.width, self.window_points.height);
                return window::latest().and_then(window::is_maximized).map(Message::Maximized);
            }
            Message::Maximized(maximized) => self.maximized = maximized,
            Message::DragWindow => return window::latest().and_then(window::drag),
            Message::ToggleMaximize => {
                self.maximized = !self.maximized;
                return window::latest().and_then(window::toggle_maximize);
            }
            Message::Minimize => return window::latest().and_then(|id| window::minimize(id, true)),
            Message::ResizeWindow(direction) => {
                return window::latest().and_then(move |id| window::drag_resize(id, direction));
            }
            Message::CloseRequested => {
                if self.live {
                    if let Err(error) = crate::cache::save_now(self.snapshot()) {
                        log::warn!("could not save the snapshot: {error}");
                    }
                    if let Err(error) = crate::settings::save_now(&self.settings) {
                        log::warn!("could not save settings: {error}");
                    }
                }
                let seconds = self.started.elapsed().as_secs_f32();
                log::info!("built {} views in {seconds:.1} s", self.renders.get());
                println!("FRAMES {} in {seconds:.1} s", self.renders.get());
                return iced::exit();
            }
        }
        Task::none()
    }

    /// Modes and the fan switch work once the helper answers (and always in demo runs).
    pub fn controls_ready(&self) -> bool {
        self.store.controls.helper == HelperState::Ready
    }

    fn set_mode(&mut self, mode: Mode, automatic: bool) -> Task<Message> {
        if !self.controls_ready() {
            self.notice = Some("Turn on hardware control to change modes.".to_owned());
            return Task::none();
        }
        let previous = self.store.controls.mode;
        let was_auto = self.store.controls.auto_eco_active;
        if automatic {
            self.store.controls.auto_eco_active = mode == Mode::Eco;
        } else {
            // A choice made while auto switch holds ECO overrides it until power changes.
            self.eco_overridden = was_auto;
            self.store.controls.auto_eco_active = false;
            self.settings.mode = mode;
        }
        self.store.controls.mode = mode;
        self.notice = None;
        if mode == Mode::Eco {
            self.store.controls.max_fan = false;
        }
        if !self.live {
            return Task::none();
        }
        let mut tasks = vec![self.save_settings()];
        tasks.push(blocking(move || crate::hw::ask_helper(Request::SetMode { mode }), move |r| {
            Message::Helper(HelperResult::ModeSet { mode, ok: r.ok && r.return_code == Some(0) })
        }));
        // ECO's Windows side: 60 Hz and the NVIDIA cap on entry, put back on exit.
        if mode == Mode::Eco && previous != Mode::Eco {
            let saved = self.settings.eco_saved.clone();
            tasks.push(blocking(move || Some(crate::eco::enter(saved)), Message::EcoApplied));
            if self.settings.max_fan {
                tasks.push(blocking(|| crate::hw::ask_helper(Request::SetMaxFan { on: false }), |_| Message::Saved(Ok(()))));
            }
        } else if mode != Mode::Eco && previous == Mode::Eco {
            if let Some(saved) = self.settings.eco_saved.clone() {
                tasks.push(blocking(move || crate::eco::leave(&saved), Message::EcoApplied));
            }
            if self.settings.max_fan {
                tasks.push(blocking(|| crate::hw::ask_helper(Request::SetMaxFan { on: true }), |_| Message::Saved(Ok(()))));
                self.store.controls.max_fan = true;
            }
        }
        Task::batch(tasks)
    }

    fn check_auto_eco(&mut self) -> Task<Message> {
        if !self.live || !self.controls_ready() {
            return Task::none();
        }
        let c = &self.store.controls;
        let want = tunefast_core::eco::should_auto_switch(c.auto_eco, self.on_ac, self.low_wattage, self.eco_overridden);
        if want && !c.auto_eco_active && c.mode != Mode::Eco {
            return self.set_mode(Mode::Eco, true);
        }
        if !want && c.auto_eco_active {
            let back = self.settings.mode;
            self.store.controls.auto_eco_active = false;
            return self.set_mode(back, true);
        }
        Task::none()
    }

    fn helper_result(&mut self, result: HelperResult) -> Task<Message> {
        match result {
            HelperResult::Hello(ok) => {
                self.store.controls.helper = if ok { HelperState::Ready } else { HelperState::Missing };
                if ok {
                    self.notice = None;
                    return Task::batch([
                        blocking(|| crate::hw::ask_helper(Request::GetMaxFan), |r: Response| {
                            Message::Helper(HelperResult::MaxFan(r.ok.then(|| bios::max_fan_on(&r.data))))
                        }),
                        Task::done(Message::PowerTick),
                    ]);
                }
            }
            HelperResult::MaxFan(Some(on)) => {
                self.store.controls.max_fan = on;
                self.settings.max_fan = on;
            }
            HelperResult::MaxFan(None) => {}
            HelperResult::ModeSet { mode, ok } => {
                if !ok && self.store.controls.mode == mode {
                    self.notice = Some(format!("The firmware did not accept {}.", mode.name()));
                }
            }
            HelperResult::MaxFanSet { on, ok } => {
                if !ok && self.store.controls.max_fan == on {
                    // Never let a late failure undo a newer choice; this one failed, so show the truth.
                    self.store.controls.max_fan = !on;
                    self.settings.max_fan = !on;
                    self.notice = Some("The fan setting was not accepted.".to_owned());
                }
            }
            HelperResult::Adapter { low_wattage } => {
                if low_wattage != self.low_wattage {
                    self.low_wattage = low_wattage;
                    return self.check_auto_eco();
                }
            }
        }
        Task::none()
    }

    fn save_settings(&self) -> Task<Message> {
        if !self.live {
            return Task::none();
        }
        Task::perform(crate::settings::save(self.settings.clone()), Message::Saved)
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            version: 0,
            store: self.store.clone(),
            screen: self.screen.id().to_owned(),
            window: (!self.maximized).then_some([self.window_points.width, self.window_points.height]),
            maximized: self.maximized,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        self.renders.set(self.renders.get() + 1);
        crate::ui::shell::view(self, self.palette())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let mut subscriptions = vec![
            // The OS close button and Alt+F4 come here (exit_on_close_request is off).
            window::close_requests().map(|_| Message::CloseRequested),
            window::resize_events().map(|(_, size)| Message::Resized(size)),
        ];
        // Sensors are read only while the window is open (not minimized), as
        // the original reads them only while its pages are visible.
        if self.live && !self.minimized {
            subscriptions.push(Subscription::run(sensor_stream));
        }
        // Auto switch to ECO follows the power source even while minimized.
        if self.live && self.store.controls.auto_eco {
            subscriptions.push(iced::time::every(Duration::from_secs(5)).map(|_| Message::PowerTick));
        }
        Subscription::batch(subscriptions)
    }

    pub fn title(&self) -> String {
        crate::APP_NAME.to_owned()
    }

    /// The original's whole-window scale; iced lays the views out on the
    /// design canvas and multiplies by this.
    pub fn scale_factor(&self) -> f32 {
        self.applied_scale.set(self.scale);
        self.scale
    }

    pub fn palette(&self) -> Palette {
        if self.settings.light { Palette::LIGHT } else { Palette::DARK }
    }

    /// Whether the left menu is docked (window at least 974.4 points wide).
    pub fn menu_docked(&self) -> bool {
        self.window_points.width >= size::MENU_BREAKPOINT
    }

    pub fn theme(&self) -> Theme {
        Theme::custom(
            "Tunefast",
            iced::theme::Palette {
                background: theme::c::WINDOW,
                text: theme::c::TEXT,
                primary: theme::c::BLUE,
                success: theme::c::NORMAL,
                warning: theme::c::WARNING,
                danger: theme::c::HOT,
            },
        )
    }

    pub fn style(&self, _theme: &Theme) -> iced::theme::Style {
        iced::theme::Style { background_color: theme::c::WINDOW, text_color: theme::c::TEXT }
    }
}

/// Runs blocking work (a firmware request, a display change) off the UI thread.
fn blocking<T: Send + 'static>(work: impl FnOnce() -> T + Send + 'static, done: impl Fn(T) -> Message + Send + 'static) -> Task<Message> {
    Task::perform(async move { tokio::task::spawn_blocking(work).await }, move |result| match result {
        Ok(value) => done(value),
        Err(error) => Message::Saved(Err(error.to_string())),
    })
}

/// One reading a second from the sensor thread, for as long as the
/// subscription lives (the thread stops when the receiver is dropped).
fn sensor_stream() -> impl iced::futures::Stream<Item = Message> {
    use iced::futures::SinkExt;
    iced::stream::channel(4, async |mut output: iced::futures::channel::mpsc::Sender<Message>| {
        let (tx, mut rx) = tokio::sync::mpsc::channel(2);
        std::thread::spawn(move || crate::hw::sensor_thread(tx));
        while let Some(reading) = rx.recv().await {
            if output.send(Message::Reading(Box::new(reading))).await.is_err() {
                break;
            }
        }
    })
}

/// The text COPY TO CLIPBOARD and PRINT produce, in the original's order.
pub fn sysinfo_text(info: &SysInfo) -> String {
    let mut lines = vec![
        "MODEL".to_owned(),
        format!("    {}", info.model),
        "SERIAL NUMBER".to_owned(),
        format!("    {}", info.serial),
        "OPERATING SYSTEM".to_owned(),
        format!("    {}", info.os_name),
        format!("        - Release ID: {}", info.os_release),
        format!("        - Version: {}", info.os_version),
        format!("        - Build: {}", info.os_build),
        "CPU".to_owned(),
        format!("    {}", info.cpu_name),
        format!("        - Clock speed: {}MHz", info.cpu_clock_mhz),
        format!("        - Description: {}", info.cpu_description),
        "GPU".to_owned(),
    ];
    for gpu in &info.gpus {
        lines.push(format!("    {}", gpu.name));
        if let Some(hz) = gpu.refresh_hz {
            lines.push(format!("        - Refresh rate: {hz}Hz"));
        }
        lines.push(format!("        - Driver version: {}", gpu.driver));
    }
    lines.push("RAM".to_owned());
    for module in &info.memory {
        lines.push(format!("    {}", module.part.trim()));
        lines.push(format!("        - Capacity: {}GB", module.capacity_gb));
        lines.push(format!("        - Type: {}", module.kind));
        lines.push(format!("        - Default clock speed: {}MHz", module.speed_mhz));
    }
    lines.push("AUDIO".to_owned());
    lines.extend(info.audio.iter().map(|a| format!("    {a}")));
    lines.join("\r\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app() -> App {
        let args = <Args as clap::Parser>::parse_from(["tunefast", "--demo"]).normalized();
        App::boot(args, None, [1280.0, 800.0]).0
    }

    #[test]
    fn choosing_a_mode_shows_at_once_and_eco_turns_max_fan_off() {
        let mut app = app();
        app.store.controls.max_fan = true;
        let _ = app.update(Message::SetMode(Mode::Performance));
        assert_eq!(app.store.controls.mode, Mode::Performance);
        assert!(app.store.controls.max_fan);
        let _ = app.update(Message::SetMode(Mode::Eco));
        assert_eq!(app.store.controls.mode, Mode::Eco);
        assert!(!app.store.controls.max_fan, "ECO disables max fan, as in the original");
        let _ = app.update(Message::SetMaxFan(true));
        assert!(!app.store.controls.max_fan, "the fan switch is disabled in ECO");
    }

    #[test]
    fn modes_need_the_helper() {
        let mut app = app();
        app.store.controls.helper = HelperState::Missing;
        let _ = app.update(Message::SetMode(Mode::Quiet));
        assert_eq!(app.store.controls.mode, Mode::Default);
        assert!(app.notice.is_some());
    }

    #[test]
    fn a_failed_fan_write_shows_the_real_state_but_a_newer_choice_wins() {
        let mut app = app();
        let _ = app.update(Message::SetMaxFan(true));
        let _ = app.update(Message::SetMaxFan(false));
        // The first write's late failure must not undo the newer choice.
        let _ = app.update(Message::Helper(HelperResult::MaxFanSet { on: true, ok: false }));
        assert!(!app.store.controls.max_fan);
        let _ = app.update(Message::Helper(HelperResult::MaxFanSet { on: false, ok: false }));
        assert!(app.store.controls.max_fan, "a failed write shows what the firmware still has");
    }

    #[test]
    fn tabs_remember_the_vitals_view() {
        let mut app = app();
        let _ = app.update(Message::Navigate(Screen::VitalsAdvanced));
        let _ = app.update(Message::Navigate(Screen::Performance));
        assert_eq!(app.vitals_screen, Screen::VitalsAdvanced);
    }

    #[test]
    fn resizing_follows_the_original_scale_and_breakpoint() {
        let mut app = app();
        app.applied_scale.set(0.6);
        let _ = app.update(Message::Resized(Size::new(1600.0, 1066.67)));
        assert!((app.window_points.width - 960.0).abs() < 0.5);
        assert_eq!(app.scale, 0.6);
        assert!(!app.menu_docked());
        let _ = app.update(Message::Resized(Size::new(0.0, 0.0)));
        assert!(app.minimized, "a minimized window stops the sensors");
    }

    #[test]
    fn sysinfo_text_lists_every_section() {
        let text = sysinfo_text(&crate::demo::store().sysinfo);
        for heading in ["MODEL", "SERIAL NUMBER", "OPERATING SYSTEM", "CPU", "GPU", "RAM", "AUDIO"] {
            assert!(text.contains(heading), "{heading}");
        }
    }
}
