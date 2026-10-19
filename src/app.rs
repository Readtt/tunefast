//! Application state, messages and the update loop (the Elm architecture):
//! views emit `Message`s, `update` changes state and returns `Task`s for
//! async work, and nothing in the UI path blocks.
//!
//! Built to feel instant (the fast-rebuild skill's references/feel.md): the
//! first frame draws the last session from a disk snapshot, rows act on
//! mouse-down, hovering a row starts loading what it opens, sends show at once,
//! the wheel scrolls as far per notch as Chromium does, the keyboard reaches
//! everything, and a loading hint appears only after 300 ms of waiting.

use std::cell::Cell;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use iced::widget::operation::{self, AbsoluteOffset};
use iced::window::screenshot::Screenshot;
use iced::{Element, Size, Subscription, Task, Theme, keyboard, window};

use crate::args::{Args, ThemeChoice};
use crate::cache::Snapshot;
use crate::model::{Conversation, Message as ChatMessage, Store};
use crate::screens::Screen;
use crate::theme::{Palette, hex, size};
use crate::ui::widgets::ids;

/// How long work may run before the interface admits it is waiting. Shorter
/// waits show nothing: a hint that flashes for 100 ms reads as a glitch.
pub const SLOW_AFTER: Duration = Duration::from_millis(300);

#[derive(Debug, Clone)]
pub enum Message {
    Navigate(Screen),
    Select(u64),
    /// Moves the selection up or down the visible list (keyboard).
    Step(i32),
    /// The pointer entered a row: start loading what it opens.
    Prefetch(u64),
    Fetched(u64, Result<Vec<ChatMessage>, String>),
    SlowLoad(u64),
    SearchChanged(String),
    FocusSearch,
    Escape,
    DraftChanged(String),
    Send,
    SetTheme(ThemeSetting),
    Toggle(Setting, bool),
    Loaded(Result<Store, String>),
    Sent(Result<ChatMessage, String>),
    Saved(Result<(), String>),
    ListScrolled { offset: f32, height: f32 },
    Resized(Size),
    DragWindow,
    ToggleMaximize,
    Minimize,
    CloseRequested,
    ResizeWindow(window::Direction),
    ShotDue,
    Shot(Screenshot),
    SmokeStep(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeSetting {
    System,
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Setting {
    Notifications,
    Badge,
    Autostart,
}

pub struct Settings {
    pub theme: ThemeSetting,
    pub notifications: bool,
    pub badge: bool,
    pub autostart: bool,
}

enum Tooling {
    Off,
    Screenshot { path: PathBuf },
    Smoke { report: Option<PathBuf>, visited: Vec<&'static str> },
}

pub struct App {
    pub screen: Screen,
    pub store: Store,
    pub selected: Option<u64>,
    pub search: String,
    pub draft: String,
    pub settings: Settings,
    /// Scroll position and height of the conversation list, for drawing
    /// only the rows that are visible.
    pub list_offset: f32,
    pub list_height: f32,
    pub error: Option<String>,
    /// A fetch that has run longer than SLOW_AFTER: the view shows a quiet hint.
    pub slow: Option<u64>,
    /// Conversations whose messages are in the store, and those on their way.
    fetched: HashSet<u64>,
    loading: HashSet<u64>,
    /// Window size to reopen at (not updated while maximized).
    window_size: Option<Size>,
    maximized: bool,
    /// Live runs keep a snapshot on disk; demo and tooling runs never touch it.
    persist: bool,
    tooling: Tooling,
    /// Views built since start, logged on exit. An idle app builds none: a
    /// count that keeps growing while nothing changes is a redraw loop.
    renders: Cell<u64>,
    started: Instant,
}

impl App {
    pub fn boot(args: Args, snapshot: Option<Snapshot>) -> (App, Task<Message>) {
        let mut tasks = Vec::new();
        let persist = !args.demo;
        let (store, last_selected, last_screen) = if let Some(path) = &args.fixtures {
            let store = std::fs::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|text| serde_json::from_str::<Store>(&text).map_err(|e| e.to_string()))
                .unwrap_or_else(|error| {
                    log::error!("could not load fixtures {}: {error}", path.display());
                    crate::demo::store()
                });
            (store, None, None)
        } else if args.demo {
            (crate::demo::store(), None, None)
        } else {
            // Draw the last session now; refresh it behind the scenes.
            tasks.push(Task::perform(crate::backend::load(), Message::Loaded));
            match &snapshot {
                Some(last) => (last.store.clone(), last.selected, Screen::from_id(&last.screen)),
                None => (Store::default(), None, None),
            }
        };
        let fetched: HashSet<u64> = if args.demo {
            store.conversations.iter().map(|c| c.id).collect()
        } else {
            store.messages.iter().map(|m| m.conversation).collect()
        };
        let tooling = if let Some(path) = &args.screenshot {
            // Let fonts, images and layout settle, then capture.
            let settle = Duration::from_millis(u64::from(args.settle_frames.max(2)) * 16);
            tasks.push(Task::perform(tokio::time::sleep(settle), |_| Message::ShotDue));
            Tooling::Screenshot { path: path.clone() }
        } else if args.smoke {
            tasks.push(Task::done(Message::SmokeStep(0)));
            Tooling::Smoke { report: args.report.clone(), visited: Vec::new() }
        } else {
            Tooling::Off
        };
        if args.is_tooling() {
            // Screenshots and smoke runs ignore the real mouse, so a cursor
            // resting over the window cannot add hover states to a capture.
            tasks.push(window::latest().and_then(window::enable_mouse_passthrough));
        }
        let screen = args.screen.as_deref().and_then(Screen::from_id).or(last_screen).unwrap_or(Screen::Home);
        let theme = match args.theme {
            Some(ThemeChoice::Dark) => ThemeSetting::Dark,
            Some(ThemeChoice::Light) => ThemeSetting::Light,
            None => ThemeSetting::System,
        };
        let selected = if screen == Screen::Empty {
            None
        } else {
            last_selected.filter(|id| store.conversation(*id).is_some()).or_else(|| store.conversations.first().map(|c| c.id))
        };
        let app = App {
            screen,
            selected,
            store,
            search: String::new(),
            draft: String::new(),
            settings: Settings { theme, notifications: true, badge: true, autostart: false },
            list_offset: 0.0,
            list_height: 800.0,
            error: None,
            slow: None,
            fetched,
            loading: HashSet::new(),
            window_size: snapshot.as_ref().and_then(|s| s.window).map(|[w, h]| Size::new(w, h)),
            maximized: snapshot.as_ref().is_some_and(|s| s.maximized),
            persist,
            tooling,
            renders: Cell::new(0),
            started: Instant::now(),
        };
        (app, Task::batch(tasks))
    }

    /// The conversations the list shows, filtered by the search box. The view
    /// and keyboard navigation both use this, so they always agree.
    pub fn rows(&self) -> Vec<&Conversation> {
        let query = self.search.to_lowercase();
        self.store.conversations.iter().filter(|c| query.is_empty() || c.title.to_lowercase().contains(&query)).collect()
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(screen) => {
                self.screen = screen;
                if screen == Screen::Empty {
                    self.selected = None;
                }
            }
            Message::Select(id) => {
                self.selected = Some(id);
                self.screen = Screen::Home;
                if let Some(c) = self.store.conversations.iter_mut().find(|c| c.id == id) {
                    c.unread = 0;
                }
                // Ready to type at once, as in the original.
                let mut tasks = vec![self.fetch(id), operation::focus(ids::COMPOSER)];
                if self.loading.contains(&id) {
                    // An async block: the timer is created when the task runs, on the runtime.
                    tasks.push(Task::perform(async { tokio::time::sleep(SLOW_AFTER).await }, move |_| Message::SlowLoad(id)));
                }
                // The neighbours too, so stepping through the list never waits.
                let rows: Vec<u64> = self.rows().iter().map(|c| c.id).collect();
                if let Some(index) = rows.iter().position(|row| *row == id) {
                    for neighbour in [index.checked_sub(1), Some(index + 1)].into_iter().flatten() {
                        if let Some(&neighbour) = rows.get(neighbour) {
                            tasks.push(self.fetch(neighbour));
                        }
                    }
                }
                return Task::batch(tasks);
            }
            Message::Step(delta) => {
                let rows: Vec<u64> = self.rows().iter().map(|c| c.id).collect();
                if rows.is_empty() {
                    return Task::none();
                }
                let next = match self.selected.and_then(|id| rows.iter().position(|row| *row == id)) {
                    Some(index) => (index as i64 + i64::from(delta)).clamp(0, rows.len() as i64 - 1) as usize,
                    None => 0,
                };
                let reveal = self.reveal_row(next);
                let select = self.update(Message::Select(rows[next]));
                return Task::batch([select, reveal]);
            }
            Message::Prefetch(id) => return self.fetch(id),
            Message::Fetched(id, result) => {
                self.loading.remove(&id);
                if self.slow == Some(id) {
                    self.slow = None;
                }
                match result {
                    Ok(messages) => {
                        self.fetched.insert(id);
                        self.store.replace_messages(id, messages);
                    }
                    Err(error) => {
                        log::warn!("loading conversation {id}: {error}");
                        self.error = Some(error);
                    }
                }
            }
            Message::SlowLoad(id) => {
                if self.loading.contains(&id) && self.selected == Some(id) {
                    self.slow = Some(id);
                }
            }
            Message::SearchChanged(text) => self.search = text,
            Message::FocusSearch => return Task::batch([operation::focus(ids::SEARCH), operation::select_all(ids::SEARCH)]),
            Message::Escape => {
                if !self.search.is_empty() {
                    self.search.clear();
                } else if self.screen == Screen::Settings {
                    self.screen = Screen::Home;
                }
            }
            Message::DraftChanged(text) => self.draft = text,
            Message::Send => {
                let body = self.draft.trim().to_owned();
                if let (Some(conversation), false) = (self.selected, body.is_empty()) {
                    self.draft.clear();
                    let id = self.store.next_message_id();
                    // Optimistic: the message shows at once; the backend
                    // confirms it behind the scenes.
                    self.store.messages.push(ChatMessage {
                        id,
                        conversation,
                        author: "You".to_owned(),
                        body: body.clone(),
                        time: "now".to_owned(),
                        mine: true,
                    });
                    return Task::perform(crate::backend::send(conversation, body, id), Message::Sent);
                }
            }
            Message::SetTheme(setting) => self.settings.theme = setting,
            Message::Toggle(setting, value) => match setting {
                Setting::Notifications => self.settings.notifications = value,
                Setting::Badge => self.settings.badge = value,
                Setting::Autostart => self.settings.autostart = value,
            },
            Message::Loaded(Ok(store)) => {
                self.fetched.extend(store.messages.iter().map(|m| m.conversation));
                self.selected = self
                    .selected
                    .filter(|id| store.conversation(*id).is_some())
                    .or_else(|| (self.screen != Screen::Empty).then(|| store.conversations.first().map(|c| c.id)).flatten());
                self.store = store;
                if self.persist {
                    return Task::perform(crate::cache::save(self.snapshot()), Message::Saved);
                }
            }
            Message::Sent(Ok(message)) => {
                if let Some(shown) = self.store.messages.iter_mut().find(|m| m.id == message.id) {
                    *shown = message;
                }
            }
            Message::Loaded(Err(error)) | Message::Sent(Err(error)) => {
                log::warn!("backend: {error}");
                self.error = Some(error);
            }
            Message::Saved(result) => {
                if let Err(error) = result {
                    log::warn!("could not save the snapshot: {error}");
                }
            }
            Message::ListScrolled { offset, height } => {
                self.list_offset = offset;
                self.list_height = height;
            }
            Message::Resized(size) => {
                if !self.maximized {
                    self.window_size = Some(size);
                }
            }
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
                if self.persist {
                    if let Err(error) = crate::cache::save_now(self.snapshot()) {
                        log::warn!("could not save the snapshot: {error}");
                    }
                }
                let seconds = self.started.elapsed().as_secs_f32();
                log::info!("built {} views in {seconds:.1} s", self.renders.get());
                println!("FRAMES {} in {seconds:.1} s", self.renders.get());
                return iced::exit();
            }
            Message::ShotDue => {
                return window::latest().and_then(window::screenshot).map(Message::Shot);
            }
            Message::Shot(shot) => {
                if let Tooling::Screenshot { path } = &self.tooling {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let (w, h) = (shot.size.width, shot.size.height);
                    match image::RgbaImage::from_raw(w, h, shot.rgba.to_vec()).map(|img| img.save(path)) {
                        Some(Ok(())) => println!("SCREENSHOT {w}x{h} {}", path.display()),
                        _ => {
                            log::error!("could not write {}", path.display());
                            std::process::exit(4);
                        }
                    }
                }
                return iced::exit();
            }
            Message::SmokeStep(index) => {
                if let Tooling::Smoke { report, visited } = &mut self.tooling {
                    if let Some(screen) = Screen::ALL.get(index) {
                        self.screen = *screen;
                        self.selected = if *screen == Screen::Empty {
                            None
                        } else {
                            self.selected.or_else(|| self.store.conversations.first().map(|c| c.id))
                        };
                        visited.push(screen.id());
                        return Task::perform(tokio::time::sleep(Duration::from_millis(150)), move |_| {
                            Message::SmokeStep(index + 1)
                        });
                    }
                    let json = serde_json::json!({
                        "ok": true,
                        "detail": "ok",
                        "visited": visited,
                        "expected": Screen::ALL.iter().map(|s| s.id()).collect::<Vec<_>>(),
                        "version": env!("CARGO_PKG_VERSION"),
                    });
                    if let Some(path) = report {
                        if let Some(parent) = path.parent() {
                            let _ = std::fs::create_dir_all(parent);
                        }
                        let _ = std::fs::write(path, json.to_string());
                    }
                    println!("SMOKE OK {} screens: {}", visited.len(), visited.join(", "));
                    return iced::exit();
                }
            }
        }
        Task::none()
    }

    /// Starts loading a conversation's messages unless they are here or on their way.
    fn fetch(&mut self, id: u64) -> Task<Message> {
        if self.fetched.contains(&id) || !self.loading.insert(id) {
            return Task::none();
        }
        Task::perform(crate::backend::messages(id), move |result| Message::Fetched(id, result))
    }

    /// Scrolls the list just enough to show row `index`.
    fn reveal_row(&mut self, index: usize) -> Task<Message> {
        let top = index as f32 * size::LIST_ROW;
        let bottom = top + size::LIST_ROW;
        let offset = if top < self.list_offset {
            top
        } else if bottom > self.list_offset + self.list_height {
            bottom - self.list_height
        } else {
            return Task::none();
        };
        self.list_offset = offset.max(0.0);
        operation::scroll_to(ids::LIST, AbsoluteOffset { x: 0.0, y: self.list_offset })
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            version: 0,
            store: self.store.clone(),
            selected: self.selected,
            screen: self.screen.id().to_owned(),
            window: self.window_size.map(|size| [size.width, size.height]),
            maximized: self.maximized,
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        self.renders.set(self.renders.get() + 1);
        crate::ui::shell::view(self, self.palette())
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            // The OS close button and Alt+F4 come here (exit_on_close_request is off).
            window::close_requests().map(|_| Message::CloseRequested),
            window::resize_events().map(|(_, size)| Message::Resized(size)),
            keyboard::listen().filter_map(shortcut),
        ])
    }

    pub fn title(&self) -> String {
        crate::APP_NAME.to_owned()
    }

    pub fn palette(&self) -> Palette {
        match self.settings.theme {
            ThemeSetting::Light => Palette::LIGHT,
            // System follows dark here; read the OS setting if the original does.
            ThemeSetting::Dark | ThemeSetting::System => Palette::DARK,
        }
    }

    /// The built-in widgets (toggles, inputs) take their colours from this.
    pub fn theme(&self) -> Theme {
        let p = self.palette();
        Theme::custom(
            "Tunefast",
            iced::theme::Palette {
                background: p.window,
                text: p.text,
                primary: p.accent,
                success: p.positive,
                warning: hex(0xF5A524),
                danger: p.danger,
            },
        )
    }

    pub fn style(&self, _theme: &Theme) -> iced::theme::Style {
        let p = self.palette();
        iced::theme::Style { background_color: p.window, text_color: p.text }
    }
}

/// Keyboard shortcuts. `keyboard::listen` only reports keys no widget used,
/// so typing in a field never triggers one. Replace these with the
/// original's shortcuts (recon/rea-digest.md lists them).
fn shortcut(event: keyboard::Event) -> Option<Message> {
    use keyboard::key::Named;
    let keyboard::Event::KeyPressed { key, modifiers, .. } = event else {
        return None;
    };
    match key.as_ref() {
        keyboard::Key::Character("k") if modifiers.command() => Some(Message::FocusSearch),
        keyboard::Key::Character(",") if modifiers.command() => Some(Message::Navigate(Screen::Settings)),
        keyboard::Key::Named(Named::ArrowUp) if modifiers.alt() => Some(Message::Step(-1)),
        keyboard::Key::Named(Named::ArrowDown) if modifiers.alt() => Some(Message::Step(1)),
        keyboard::Key::Named(Named::Escape) => Some(Message::Escape),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::widgets;
    use clap::Parser;

    fn app() -> App {
        App::boot(Args::parse_from(["tunefast", "--demo"]).normalized(), None).0
    }

    fn key(name: keyboard::Key, modifiers: keyboard::Modifiers) -> keyboard::Event {
        keyboard::Event::KeyPressed {
            key: name.clone(),
            modified_key: name,
            physical_key: keyboard::key::Physical::Unidentified(keyboard::key::NativeCode::Unidentified),
            location: keyboard::Location::Standard,
            modifiers,
            text: None,
            repeat: false,
        }
    }

    #[test]
    fn sending_shows_the_message_at_once_and_clears_the_draft() {
        let mut app = app();
        let before = app.store.messages_of(1).count();
        app.draft = "  hello  ".to_owned();
        let _ = app.update(Message::Send);
        assert_eq!(app.store.messages_of(1).count(), before + 1);
        assert_eq!(app.store.messages_of(1).last().map(|m| m.body.as_str()), Some("hello"));
        assert!(app.draft.is_empty());
    }

    #[test]
    fn selecting_clears_unread() {
        let mut app = app();
        let id = app.store.conversations.iter().find(|c| c.unread > 0).map(|c| c.id).expect("an unread row");
        let _ = app.update(Message::Select(id));
        assert_eq!(app.selected, Some(id));
        assert_eq!(app.store.conversation(id).map(|c| c.unread), Some(0));
    }

    #[test]
    fn stepping_moves_the_selection_and_stops_at_the_ends() {
        let mut app = app();
        let ids: Vec<u64> = app.rows().iter().map(|c| c.id).collect();
        let _ = app.update(Message::Select(ids[0]));
        let _ = app.update(Message::Step(1));
        assert_eq!(app.selected, Some(ids[1]));
        let _ = app.update(Message::Step(-5));
        assert_eq!(app.selected, Some(ids[0]));
    }

    #[test]
    fn stepping_follows_the_search_filter() {
        let mut app = app();
        let target = app.store.conversations[3].clone();
        app.search = target.title.clone();
        let _ = app.update(Message::Step(1));
        assert_eq!(app.selected, Some(target.id));
    }

    #[test]
    fn a_wheel_notch_scrolls_as_far_as_in_chromium() {
        let notch = widgets::scaled_delta(iced::mouse::ScrollDelta::Lines { x: 0.0, y: -1.0 });
        assert_eq!(notch, iced::mouse::ScrollDelta::Pixels { x: 0.0, y: -widgets::wheel_notch_px() });
        let touchpad = iced::mouse::ScrollDelta::Pixels { x: 0.0, y: 12.5 };
        assert_eq!(widgets::scaled_delta(touchpad), touchpad);
    }

    #[test]
    fn hovering_loaded_rows_starts_no_work() {
        let mut app = app();
        let id = app.store.conversations[0].id;
        assert_eq!(app.update(Message::Prefetch(id)).units(), 0);
    }

    #[test]
    fn opening_a_conversation_also_loads_its_neighbours() {
        let mut app = app();
        app.fetched.clear();
        let ids: Vec<u64> = app.rows().iter().map(|c| c.id).collect();
        let _ = app.update(Message::Select(ids[2]));
        assert!([ids[1], ids[2], ids[3]].iter().all(|id| app.loading.contains(id)));
        assert!(!app.loading.contains(&ids[0]) && !app.loading.contains(&ids[4]));
    }

    #[test]
    fn a_slow_load_shows_a_hint_only_while_it_is_still_loading() {
        let mut app = app();
        let id = 9_999;
        app.fetched.remove(&id);
        assert!(app.fetch(id).units() > 0);
        app.selected = Some(id);
        let _ = app.update(Message::SlowLoad(id));
        assert_eq!(app.slow, Some(id));
        let _ = app.update(Message::Fetched(id, Ok(Vec::new())));
        assert_eq!(app.slow, None);
        let _ = app.update(Message::SlowLoad(id));
        assert_eq!(app.slow, None);
    }

    #[test]
    fn shortcuts_map_to_actions() {
        let ctrl = keyboard::Modifiers::COMMAND;
        assert!(matches!(shortcut(key(keyboard::Key::Character("k".into()), ctrl)), Some(Message::FocusSearch)));
        assert!(matches!(
            shortcut(key(keyboard::Key::Named(keyboard::key::Named::ArrowDown), keyboard::Modifiers::ALT)),
            Some(Message::Step(1))
        ));
        assert!(shortcut(key(keyboard::Key::Character("k".into()), keyboard::Modifiers::empty())).is_none());
    }
}
