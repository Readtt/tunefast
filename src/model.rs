//! What the screens draw. Replace these with the rebuilt app's own domain
//! types, shaped after the original's API (see `recon/api-surface.md`).

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: u64,
    pub title: String,
    pub preview: String,
    pub time: String,
    pub unread: u32,
    /// 0..1, picks the avatar colour when there is no picture.
    pub hue: f32,
    pub online: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: u64,
    pub conversation: u64,
    pub author: String,
    pub body: String,
    pub time: String,
    pub mine: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Channel {
    pub name: String,
    pub unread: bool,
}

/// Everything loaded so far. The UI reads it; only `App::apply` and backend
/// events change it. Serializable so `--fixtures` can load a screen's exact
/// content from JSON for visual comparisons.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Store {
    pub workspace: String,
    pub conversations: Vec<Conversation>,
    pub channels: Vec<Channel>,
    pub messages: Vec<Message>,
}

impl Store {
    pub fn conversation(&self, id: u64) -> Option<&Conversation> {
        self.conversations.iter().find(|c| c.id == id)
    }

    pub fn messages_of(&self, conversation: u64) -> impl Iterator<Item = &Message> {
        self.messages.iter().filter(move |m| m.conversation == conversation)
    }

    pub fn next_message_id(&self) -> u64 {
        self.messages.iter().map(|m| m.id).max().unwrap_or(0) + 1
    }

    /// Swaps in a conversation's messages as the server sent them.
    pub fn replace_messages(&mut self, conversation: u64, messages: Vec<Message>) {
        self.messages.retain(|m| m.conversation != conversation);
        self.messages.extend(messages);
    }
}

/// Up to two initials for an avatar.
pub fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|word| word.chars().find(|c| c.is_alphanumeric()))
        .take(2)
        .flat_map(char::to_uppercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initials_take_the_first_letters() {
        assert_eq!(initials("Ada Lovelace"), "AL");
        assert_eq!(initials("design-team"), "D");
        assert_eq!(initials("  grace   brewster murray "), "GB");
        assert_eq!(initials(""), "");
    }
}
