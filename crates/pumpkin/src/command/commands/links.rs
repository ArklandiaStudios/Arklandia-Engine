use std::borrow::Cow;

use pumpkin_config::ServerLinksConfig;
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::{TextComponent, click::ClickEvent, color::RGBColor};

use crate::command::argument_builder::{ArgumentBuilder, command};
use crate::command::context::command_context::CommandContext;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};

struct LinkExecutor {
    label: &'static str,
    url: String,
}

impl CommandExecutor for LinkExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let message = TextComponent::text(format!("{} : ", self.label))
            .color_rgb(RGBColor::new(255, 179, 71))
            .add_child(
                TextComponent::text(self.url.clone())
                    .color_rgb(RGBColor::new(255, 209, 102))
                    .underlined()
                    .click_event(ClickEvent::OpenUrl {
                        url: Cow::from(self.url.clone()),
                    }),
            );
        context.source.send_feedback(message, false);
        Ok(1)
    }
}

/// Registers `/bug`, `/discord` and `/site`. A command exists only while its link is set,
/// whatever `server_links.enabled` says (it only controls the links sent to the client menu).
pub fn register(
    dispatcher: &mut CommandDispatcher,
    registry: &PermissionRegistry,
    config: &ServerLinksConfig,
) {
    let links: [(&'static str, &'static str, &str, &'static str, &'static str); 3] = [
        (
            "bug",
            "Signaler un bug",
            &config.bug_report,
            "Arklandia:command.bug",
            "Gives the link to report a bug.",
        ),
        (
            "discord",
            "Discord",
            &config.discord,
            "Arklandia:command.discord",
            "Gives the Discord link.",
        ),
        (
            "site",
            "Site web",
            &config.website,
            "Arklandia:command.site",
            "Gives the website link.",
        ),
    ];
    for (name, label, url, permission, description) in links {
        let url = url.trim();
        if url.is_empty() {
            continue;
        }
        registry.register_permission_or_panic(Permission::new(
            permission,
            description,
            PermissionDefault::Allow,
        ));
        dispatcher.register(command(name, description).requires(permission).executes(
            LinkExecutor {
                label,
                url: url.to_string(),
            },
        ));
    }
}
