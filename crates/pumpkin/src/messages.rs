//! Join / quit announcements configured in `[messages]` of `pumpkin.toml`.
//!
//! An empty configuration field means "send nothing": the vanilla message is then
//! suppressed too. Plugins can still cancel (or edit) the message through
//! `PlayerJoinEvent` / `PlayerLeaveEvent`.

use std::sync::Arc;

use pumpkin_util::text::{TextComponent, markup};
use tracing::info;

use crate::entity::EntityBase;
use crate::entity::player::Player;
use crate::plugin::api::events::player::{player_join::PlayerJoinEvent, player_leave::PlayerLeaveEvent};
use crate::server::Server;

/// Joins rendered lines in one component (lines separated by a line feed).
#[must_use]
pub fn lines_to_component(lines: &[TextComponent]) -> TextComponent {
    let mut root = TextComponent::text("");
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            root = root.add_text("\n");
        }
        root = root.add_child(line.clone());
    }
    root
}

fn world_name(player: &Player) -> String {
    let world = player.world();
    let name = world.dimension.minecraft_name;
    name.strip_prefix("minecraft:").unwrap_or(name).to_string()
}

fn render_for(
    server: &Arc<Server>,
    player: &Arc<Player>,
    template: &str,
    count: usize,
) -> Vec<TextComponent> {
    let name = player.gameprofile.name.clone();
    let display = player.get_display_name().get_text();
    let display = if display.trim().is_empty() { name.clone() } else { display };
    let online = server.get_player_count().to_string();
    let max = server.advanced_config.networking.java.max_players.to_string();
    let world = world_name(player);
    let count = count.to_string();
    markup::render(
        template,
        &[
            ("player", &name),
            ("displayname", &display),
            ("online", &online),
            ("max", &max),
            ("world", &world),
            ("count", &count),
        ],
        &[],
    )
}

fn broadcast(server: &Arc<Server>, message: &TextComponent) {
    server.for_each_player(|p| p.send_system_message(message));
    info!("{}", message.clone().to_pretty_console());
}

/// Announces a joined player. `first_join` must be computed *before* the player is flagged
/// as having played (no saved player data file).
pub async fn announce_join(server: &Arc<Server>, player: &Arc<Player>, first_join: bool) {
    let cfg = &server.advanced_config.messages;
    let count = server.player_data_storage.saved_player_count() + usize::from(first_join);
    let count = count.max(server.get_player_count());
    let template = if first_join && !cfg.first_join.trim().is_empty() {
        cfg.first_join.as_str()
    } else {
        cfg.join.as_str()
    };
    let lines = render_for(server, player, template, count);
    let silent = lines.is_empty();

    let mut event = PlayerJoinEvent::new(player.clone(), lines_to_component(&lines));
    server.plugin_manager.fire(server, &mut event).await;
    if !event.cancelled && !silent {
        broadcast(server, &event.join_message);
    }

    if first_join {
        for line in render_for(server, player, &cfg.welcome, count) {
            player.send_system_message(&line);
        }
    }
}

/// Announces a leaving player (already removed from its world's player list).
pub async fn announce_quit(server: &Arc<Server>, player: &Arc<Player>) {
    let cfg = &server.advanced_config.messages;
    let count = server.player_data_storage.saved_player_count();
    let lines = render_for(server, player, &cfg.quit, count);
    let silent = lines.is_empty();

    let mut event = PlayerLeaveEvent::new(player.clone(), lines_to_component(&lines));
    server.plugin_manager.fire(server, &mut event).await;
    if !event.cancelled && !silent {
        broadcast(server, &event.leave_message);
    }
    crate::command::commands::msg::forget_player(&player.gameprofile.id);
}

/// Command line as written in the console log. The text of private messages is never logged.
#[must_use]
pub fn loggable_command(server: &Server, command: &str) -> String {
    if !server.advanced_config.private_messages.enabled {
        return command.to_string();
    }
    let name = command.trim_start().split_whitespace().next().unwrap_or("");
    let bare = name.rsplit(':').next().unwrap_or(name).to_ascii_lowercase();
    if matches!(bare.as_str(), "msg" | "tell" | "w" | "whisper" | "m" | "r" | "reply") {
        format!("{name} [private message hidden]")
    } else {
        command.to_string()
    }
}
