//! Tunefast's hardware vocabulary, shared by the window and the helper service.
//!
//! Everything that reaches the firmware is spelled out here, once, as the exact
//! payloads the laptop maker's own app sends on the Victus 16-d1xxx (board
//! 8A26). The helper accepts only [`protocol::Request`] values and turns each
//! into one of these calls; there is no way to send anything else.

pub mod bios;
pub mod eco;
pub mod protocol;
pub mod sensors;
