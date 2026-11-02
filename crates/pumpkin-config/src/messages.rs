use serde::{Deserialize, Serialize};

/// Join and quit messages (`[messages]` in `pumpkin.toml`).
///
/// Text format: `&0-&9 &a-&f &k &l &m &n &o &r`, `&#RRGGBB`, `&&` for a literal `&`, `\n`
/// for a line break. Variables: `{player}`, `{displayname}`, `{online}`, `{max}`, `{world}`,
/// `{count}`.
///
/// **An empty field means that nothing is sent for this action** (the vanilla
/// "X joined the game" / "X left the game" messages are suppressed too).
#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct MessagesConfig {
    /// Message sent to everybody when a player joins.
    pub join: String,
    /// Message sent to everybody when a player leaves.
    pub quit: String,
    /// Message sent to everybody when a player joins for the first time (replaces `join`).
    pub first_join: String,
    /// Private message sent only to a player joining for the first time (may have several
    /// lines separated by `\n`). Empty by default.
    pub welcome: String,
}

impl Default for MessagesConfig {
    fn default() -> Self {
        Self {
            join: "&8[&a+&8] &7{player}".to_string(),
            quit: "&8[&c-&8] &7{player}".to_string(),
            first_join: "&8[&6★&8] &7Bienvenue à &f{player}&7 sur Arklandia !".to_string(),
            welcome: String::new(),
        }
    }
}

/// Custom private messages (`[private_messages]` in `pumpkin.toml`): `/msg`, `/tell`,
/// `/w`, `/whisper`, `/m`, `/r`, `/reply`.
///
/// Text format: see [`MessagesConfig`]. Variables: `{sender}`, `{target}`, `{message}`.
/// Empty message fields (except `sound`, `spy_plugin`) use the built-in default.
#[derive(Deserialize, Serialize, Clone)]
#[serde(default)]
pub struct PrivateMessagesConfig {
    /// Enables the custom system. When `false`, the commands fall back to the vanilla
    /// display (no reply memory, no plugin hook).
    pub enabled: bool,
    /// What the sender sees.
    pub format_sent: String,
    /// What the recipient sees.
    pub format_received: String,
    /// Sound played to the recipient (e.g. `minecraft:block.note_block.pling`). Empty = none.
    pub sound: String,
    /// Usage hint shown by `/r` without message.
    pub reply_hint: String,
    /// Shown when the recipient is not online (`{target}` is the typed name).
    pub not_found: String,
    /// Shown by `/r` when nobody can be answered.
    pub no_reply_target: String,
    /// Shown by `/r` when the last correspondent has disconnected.
    pub reply_offline: String,
    /// Shown when writing to oneself.
    pub self_message: String,
    /// Shown when the message is empty.
    pub empty_message: String,
    /// Plugin consulted (IPC) before each private message, e.g. `ArkStaff`. Empty = none.
    pub spy_plugin: String,
    /// Maximum wait for the plugin answer, in milliseconds; on timeout the message passes.
    pub spy_timeout_ms: u64,
}

impl Default for PrivateMessagesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            format_sent: "&8[&7Vous &8→ &f{target}&8] &7{message}".to_string(),
            format_received: "&8[&f{sender} &8→ &7Vous&8] &7{message}".to_string(),
            sound: String::new(),
            reply_hint: "&7Utilisation : &f/r <message>".to_string(),
            not_found: "&cJoueur introuvable.".to_string(),
            no_reply_target: "&cPersonne à qui répondre.".to_string(),
            reply_offline: "&c{target} s'est déconnecté.".to_string(),
            self_message: "&cVous ne pouvez pas vous écrire à vous-même.".to_string(),
            empty_message: "&cLe message est vide.".to_string(),
            spy_plugin: String::new(),
            spy_timeout_ms: 250,
        }
    }
}

impl PrivateMessagesConfig {
    /// Returns `value`, or `default` when `value` is blank.
    #[must_use]
    pub fn or_default(value: &str, default: &str) -> String {
        if value.trim().is_empty() { default } else { value }.to_string()
    }
}
