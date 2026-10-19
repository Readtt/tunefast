//! Built-in sample data for `--demo`, screenshots and smoke runs. It must be
//! deterministic (no clock, no randomness) so screenshots repeat exactly.
//!
//! For a rebuild, mirror the reference screenshots' layout here: the same
//! number of rows, similar name and message lengths, the same unread counts.
//! Invent the names and text; never copy real people's data from the
//! original app into fixtures.

use crate::model::{Channel, Conversation, Message, Store};

const PEOPLE: &[(&str, &str)] = &[
    ("Maya Chen", "Pushed the new onboarding flow, take a look when you can"),
    ("Design Crit", "Leo: the spacing on the cards feels much better now"),
    ("Jonas Weber", "Sounds good, I'll send the build tonight"),
    ("Release Train", "v2.4 is green on all three platforms"),
    ("Priya Natarajan", "Can we move the sync to 3pm?"),
    ("Sam Okafor", "Ha, that bug was a single missing await"),
    ("Infra", "Disk alerts are back to normal"),
    ("Elena Rossi", "Thanks! Merged."),
    ("Weekend Plans", "Tomas: anyone up for the climbing gym?"),
    ("Hiro Tanaka", "The profiler shows 4 ms per frame now"),
    ("Product", "Roadmap draft is in the shared folder"),
    ("Noor Haddad", "Love the new icon set"),
    ("Book Club", "Next pick: The Soul of a New Machine"),
    ("Chris Martin", "Ping me before you deploy"),
    ("Support", "Two new tickets about login on Windows"),
    ("Ava Johansson", "See you tomorrow"),
];

const TIMES: &[&str] = &[
    "9:41", "9:12", "8:55", "Yesterday", "Yesterday", "Mon", "Mon", "Sun", "Sat", "Fri", "Thu",
    "Wed", "Oct 2", "Sep 28", "Sep 21", "Sep 14",
];

const THREAD: &[(&str, &str, bool)] = &[
    ("Maya Chen", "Morning! I pushed the new onboarding flow to the preview branch.", false),
    ("Maya Chen", "It cuts the setup from five screens to two. Curious what you think about the copy on the second one.", false),
    ("You", "Nice, opening it now.", true),
    ("You", "First impression: it's so much faster. The progress dots make it obvious how far along you are.", true),
    ("Maya Chen", "That was the goal. Analytics said most people dropped off on screen three.", false),
    ("You", "One thing: the Skip button is easy to miss in dark mode. Could it use the accent colour?", true),
    ("Maya Chen", "Good catch, fixing it.", false),
    ("Maya Chen", "Pushed the new onboarding flow, take a look when you can", false),
];

pub fn store() -> Store {
    let conversations = PEOPLE
        .iter()
        .enumerate()
        .map(|(index, (title, preview))| Conversation {
            id: index as u64 + 1,
            title: (*title).to_owned(),
            preview: (*preview).to_owned(),
            time: TIMES[index % TIMES.len()].to_owned(),
            unread: match index {
                0 => 2,
                3 => 12,
                6 => 1,
                _ => 0,
            },
            hue: (index as f32 * 0.137).fract(),
            online: index % 3 == 0,
        })
        .collect();
    let messages = THREAD
        .iter()
        .enumerate()
        .map(|(index, (author, body, mine))| Message {
            id: index as u64 + 1,
            conversation: 1,
            author: (*author).to_owned(),
            body: (*body).to_owned(),
            time: format!("9:{:02}", 20 + index * 3),
            mine: *mine,
        })
        .collect();
    let channels = ["general", "design", "engineering", "releases", "random"]
        .iter()
        .enumerate()
        .map(|(index, name)| Channel {
            name: (*name).to_owned(),
            unread: index == 1 || index == 3,
        })
        .collect();
    Store {
        workspace: "Northwind".to_owned(),
        conversations,
        channels,
        messages,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_demo_is_deterministic_and_has_an_open_thread() {
        let first = super::store();
        let second = super::store();
        assert_eq!(first.conversations, second.conversations);
        assert!(first.messages_of(1).count() >= 5);
        assert!(first.conversations.iter().any(|c| c.unread > 0));
    }
}
