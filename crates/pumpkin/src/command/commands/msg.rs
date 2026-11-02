//! Private messages: `/msg` (`tell`, `w`, `whisper`, `m`), `/r` (`reply`).
//!
//! Formats and answers are configured in `[private_messages]` of `pumpkin.toml`.
//! Before a message is delivered, the plugin named by `private_messages.spy_plugin` (if loaded)
//! receives an IPC message `{"op":"pm",...}` and may refuse it, see `CHANGES-msg.md`.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use pumpkin_data::sound::SoundCategory;
use pumpkin_data::world::{MSG_COMMAND_INCOMING, MSG_COMMAND_OUTGOING};
use pumpkin_util::PermissionLvl;
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::{TextComponent, markup};
use uuid::Uuid;

use crate::command::argument_builder::{ArgumentBuilder, argument, command};
use crate::command::argument_types::core::string::StringArgumentType;
use crate::command::context::command_context::CommandContext;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};
use crate::command::suggestion::provider::{SuggestionProvider, SuggestionProviderResult};
use crate::command::suggestion::suggestions::SuggestionsBuilder;
use crate::entity::EntityBase;
use crate::entity::player::Player;
use crate::server::Server;
use pumpkin_config::PrivateMessagesConfig as Cfg;

const NAMES: [&str; 5] = ["msg", "tell", "w", "whisper", "m"];
const REPLY_NAMES: [&str; 2] = ["r", "reply"];
const DESCRIPTION: &str = "Sends a private message to a player.";
const REPLY_DESCRIPTION: &str = "Replies to the last player you exchanged a private message with.";
const PERMISSION: &str = "minecraft:command.msg";
const REPLY_PERMISSION: &str = "minecraft:command.reply";
/// Players holding this permission may use `&` formatting codes in their private messages.
pub const COLOR_PERMISSION: &str = "pumpkin.msg.color";

/// Last correspondent of each player: `uuid -> (correspondent uuid, correspondent name)`.
static LAST: LazyLock<Mutex<HashMap<Uuid, (Uuid, String)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Forgets the reply target of a player who disconnected.
pub fn forget_player(id: &Uuid) {
    LAST.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(id);
}

fn last_of(id: &Uuid) -> Option<(Uuid, String)> {
    LAST.lock().unwrap_or_else(std::sync::PoisonError::into_inner).get(id).cloned()
}

fn remember(a: &Player, b: &Player) {
    let mut map = LAST.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    map.insert(a.gameprofile.id, (b.gameprofile.id, b.gameprofile.name.clone()));
    map.insert(b.gameprofile.id, (a.gameprofile.id, a.gameprofile.name.clone()));
}

fn say(context: &CommandContext, template: &str, default: &str, target: &str) {
    let template = Cfg::or_default(template, default);
    for line in markup::render(&template, &[("target", target), ("sender", "")], &[]) {
        context.source.send_message(line);
    }
}

/// Asks the configured plugin whether the message may be delivered.
/// Returns `None` when allowed, `Some(reason)` when refused (empty reason = not found).
fn ask_plugin(server: &Arc<Server>, from: &Player, to: &Player, message: &str) -> Option<String> {
    let cfg = &server.advanced_config.private_messages;
    let plugin = cfg.spy_plugin.trim();
    if plugin.is_empty() {
        return None;
    }
    let payload = serde_json::json!({
        "op": "pm",
        "from": from.gameprofile.id.to_string(),
        "from_name": from.gameprofile.name,
        "to": to.gameprofile.id.to_string(),
        "to_name": to.gameprofile.name,
        "message": message,
    })
    .to_string();
    let timeout = Duration::from_millis(cfg.spy_timeout_ms.max(1));
    let fut = async {
        tokio::time::timeout(
            timeout,
            server
                .plugin_manager
                .send_message("Pumpkin", plugin, payload.as_bytes()),
        )
        .await
    };
    let answer = if tokio::runtime::Handle::try_current().is_ok() {
        tokio::task::block_in_place(|| server.runtime.block_on(fut))
    } else {
        server.runtime.block_on(fut)
    };
    // Plugin not loaded, timeout, error or unreadable answer: the message passes.
    let Ok(Ok(Ok(bytes))) = answer else {
        return None;
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    if value.get("allow").and_then(serde_json::Value::as_bool) == Some(false) {
        let reason = value.get("reason").and_then(|r| r.as_str()).unwrap_or("");
        return Some(if reason == "not_found" { String::new() } else { reason.to_string() });
    }
    None
}

fn deliver(
    context: &CommandContext,
    sender: Option<Arc<Player>>,
    target: &Arc<Player>,
    message: &str,
) -> CommandExecutorResult {
    let server = context.server();
    let cfg = &server.advanced_config.private_messages;
    let target_name = target.gameprofile.name.clone();

    if !cfg.enabled {
        // Vanilla display.
        let text = TextComponent::text(message.to_string());
        if let Some(p) = &sender {
            p.send_message(&text, MSG_COMMAND_OUTGOING, &p.get_display_name(), Some(&target.get_display_name()));
            target.send_message(&text, MSG_COMMAND_INCOMING, &p.get_display_name(), Some(&target.get_display_name()));
        } else {
            target.send_message(&text, MSG_COMMAND_INCOMING, &context.source.display_name, Some(&target.get_display_name()));
        }
        return Ok(1);
    }

    if message.trim().is_empty() {
        say(context, &cfg.empty_message, &Cfg::default().empty_message, &target_name);
        return Ok(0);
    }
    if let Some(p) = &sender {
        if p.gameprofile.id == target.gameprofile.id {
            say(context, &cfg.self_message, &Cfg::default().self_message, &target_name);
            return Ok(0);
        }
        if let Some(reason) = ask_plugin(server, p, target, message) {
            if reason.trim().is_empty() {
                say(context, &cfg.not_found, &Cfg::default().not_found, &target_name);
            } else {
                for line in markup::render(&reason, &[("target", &target_name)], &[]) {
                    context.source.send_message(line);
                }
            }
            return Ok(0);
        }
    }

    let sender_name = sender
        .as_ref()
        .map_or_else(|| "Console".to_string(), |p| p.gameprofile.name.clone());
    let can_color = sender
        .as_ref()
        .is_some_and(|p| p.has_permission(server, COLOR_PERMISSION));
    let (sn, tn): (&str, &str) = (&sender_name, &target_name);
    let (plain, raw): (&[(&str, &str)], &[(&str, &str)]) = if can_color {
        (&[("sender", sn), ("target", tn)][..], &[("message", message)][..])
    } else {
        (&[("sender", sn), ("target", tn), ("message", message)][..], &[][..])
    };

    if let Some(p) = &sender {
        let tpl = Cfg::or_default(&cfg.format_sent, &Cfg::default().format_sent);
        for line in markup::render(&tpl, plain, raw) {
            p.send_system_message(&line);
        }
    }
    let tpl = Cfg::or_default(&cfg.format_received, &Cfg::default().format_received);
    for line in markup::render(&tpl, plain, raw) {
        target.send_system_message(&line);
    }
    let sound = cfg.sound.trim();
    if !sound.is_empty() {
        target.play_custom_sound(sound, SoundCategory::Players, &target.position(), 1.0, 1.0);
    }
    if let Some(p) = &sender {
        remember(p, target);
    }
    Ok(1)
}

struct MsgExecutor;

impl CommandExecutor for MsgExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let server = context.server();
        let name = StringArgumentType::get(context, "target")?;
        let message = StringArgumentType::get(context, "message")?;
        let cfg = &server.advanced_config.private_messages;
        let Some(target) = server.get_player_by_name(name) else {
            say(context, &cfg.not_found, &Cfg::default().not_found, name);
            return Ok(0);
        };
        deliver(context, context.source.as_player(), &target, message)
    }
}

struct ReplyExecutor;

impl CommandExecutor for ReplyExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let server = context.server();
        let cfg = &server.advanced_config.private_messages;
        let message = StringArgumentType::get(context, "message")?;
        let Some(sender) = context.source.as_player() else {
            say(context, &cfg.no_reply_target, &Cfg::default().no_reply_target, "");
            return Ok(0);
        };
        let Some((id, name)) = last_of(&sender.gameprofile.id) else {
            say(context, &cfg.no_reply_target, &Cfg::default().no_reply_target, "");
            return Ok(0);
        };
        let Some(target) = server.get_player_by_uuid(id) else {
            forget_player(&sender.gameprofile.id);
            say(context, &cfg.reply_offline, &Cfg::default().reply_offline, &name);
            return Ok(0);
        };
        deliver(context, Some(sender), &target, message)
    }
}

struct ReplyHintExecutor;

impl CommandExecutor for ReplyHintExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let cfg = &context.server().advanced_config.private_messages;
        let target = context
            .source
            .as_player()
            .and_then(|p| last_of(&p.gameprofile.id))
            .map_or_else(String::new, |(_, n)| n);
        say(context, &cfg.reply_hint, &Cfg::default().reply_hint, &target);
        Ok(0)
    }
}

struct OnlinePlayers;

impl SuggestionProvider for OnlinePlayers {
    fn suggest(&self, context: &CommandContext, mut builder: SuggestionsBuilder) -> SuggestionProviderResult {
        let me = context.source.as_player().map(|p| p.gameprofile.id);
        for player in context.source.server().get_all_players() {
            if Some(player.gameprofile.id) != me {
                builder = builder.suggest(player.gameprofile.name.clone());
            }
        }
        builder.build()
    }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(
        PERMISSION,
        DESCRIPTION,
        PermissionDefault::Op(PermissionLvl::Zero),
    ));
    registry.register_permission_or_panic(Permission::new(
        REPLY_PERMISSION,
        REPLY_DESCRIPTION,
        PermissionDefault::Op(PermissionLvl::Zero),
    ));
    registry.register_permission_or_panic(Permission::new(
        COLOR_PERMISSION,
        "Allows formatting codes (&a, &l, &#RRGGBB...) in private messages.",
        PermissionDefault::Deny,
    ));

    for name in NAMES {
        dispatcher.register(
            command(name, DESCRIPTION).requires(PERMISSION).then(
                argument("target", StringArgumentType::SingleWord)
                    .suggests(OnlinePlayers)
                    .then(argument("message", StringArgumentType::GreedyPhrase).executes(MsgExecutor)),
            ),
        );
    }
    for name in REPLY_NAMES {
        dispatcher.register(
            command(name, REPLY_DESCRIPTION)
                .requires(REPLY_PERMISSION)
                .executes(ReplyHintExecutor)
                .then(argument("message", StringArgumentType::GreedyPhrase).executes(ReplyExecutor)),
        );
    }
}
