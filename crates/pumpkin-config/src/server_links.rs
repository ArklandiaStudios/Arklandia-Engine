use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Links and announcements shown to players through commands and in the chat.
///
/// `/bug`, `/discord` and `/site` exist only while their link is not empty.
#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct ServerLinksConfig {
    /// Whether server links are enabled.
    pub enabled: bool,
    /// Link given by `/bug`.
    pub bug_report: String,
    /// Link given by `/discord`.
    pub discord: String,
    /// Link given by `/site`.
    pub website: String,
    /// Message sent in the chat to every player who joins. Nothing is sent while it is empty.
    pub announcements: String,
    /// Custom key-value links.
    pub custom: HashMap<String, String>,
}

impl Default for ServerLinksConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bug_report: String::new(),
            discord: String::new(),
            website: String::new(),
            announcements: String::new(),
            custom: HashMap::default(),
        }
    }
}
