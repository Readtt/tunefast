//! Views: functions from `&App` to `Element<Message>`. They read state and
//! emit messages; `App::update` changes state. Nothing here blocks.

pub mod home;
pub mod settings;
pub mod shell;
pub mod widgets;
