//! Tunefast's hardware helper.
//!
//! The laptop's firmware interface (WMI `root\wmi:hpqBIntM`) only answers
//! elevated callers, so the window (which never runs elevated) asks this small
//! service. The service accepts the fixed requests of
//! `tunefast_core::protocol::Request` and nothing else: each maps to exactly
//! one of the calls the maker's own app sends on this model.
//!
//! ```text
//! tunefast-helper install     # once, from an elevated prompt (or the app's "Enable" button)
//! tunefast-helper uninstall
//! tunefast-helper selftest    # elevated: reads every value, changes nothing
//! tunefast-helper serve       # foreground, for debugging (elevated)
//! ```

#[cfg(windows)]
mod windows_impl;

#[cfg(windows)]
fn main() {
    std::process::exit(windows_impl::main());
}

#[cfg(not(windows))]
fn main() {
    eprintln!("tunefast-helper only runs on Windows; the app uses its demo backend elsewhere.");
    std::process::exit(2);
}
