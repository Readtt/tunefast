//! Network and disk work. Each call is an async function the app runs with
//! `Task::perform`, on tokio, off the UI thread; the result comes back as a
//! `Message`. Long-lived streams (websockets, push) use a `Subscription`
//! built with `iced::stream::channel` instead.

use std::sync::OnceLock;

use crate::model::{Message as ChatMessage, Store};

/// The HTTP client, created on first use so demo runs never load TLS roots.
#[allow(dead_code)]
pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(concat!("tunefast/", env!("CARGO_PKG_VERSION")))
            .build()
            .unwrap_or_default()
    })
}

/// Loads everything the first screen needs. Replace with the real API call;
/// keep the demo data for --demo.
pub async fn load() -> Result<Store, String> {
    Ok(crate::demo::store())
}

/// Loads one conversation's messages: when its row is hovered (so they are
/// here by the time the click lands) or opened. Replace with the real call.
pub async fn messages(conversation: u64) -> Result<Vec<ChatMessage>, String> {
    Ok(crate::demo::store().messages.into_iter().filter(|m| m.conversation == conversation).collect())
}

/// Posts a message and returns the server's copy.
pub async fn send(conversation: u64, body: String, id: u64) -> Result<ChatMessage, String> {
    Ok(ChatMessage {
        id,
        conversation,
        author: "You".to_owned(),
        body,
        time: "now".to_owned(),
        mine: true,
    })
}
