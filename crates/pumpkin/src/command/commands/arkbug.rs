use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::{TextComponent, color::RGBColor};

use crate::command::argument_builder::{ArgumentBuilder, command};
use crate::command::context::command_context::CommandContext;
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};

const DESCRIPTION: &str = "Affiche la version du moteur Arklandia Engine et ses correctifs.";
const PERMISSION: &str = "ArkBug:arkbug.status";
const FIXES: [u64; 30] = [
    3844, 3759, 4048, 3516, 4089, 3972, 4123, 3847, 3918, 3933, 3969, 4063, 4090, 3989, 3065, 3716,
    3698, 3730, 2016, 4045, 3902, 3571, 3510, 3367, 3511, 3512, 3567, 4024, 4075, 4046,
];

struct ArkBugExecutor;

impl CommandExecutor for ArkBugExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let prefix = TextComponent::text("[")
            .color_rgb(RGBColor::new(215, 184, 153))
            .add_child(
                TextComponent::text("A")
                    .color_rgb(RGBColor::new(255, 179, 71))
                    .bold(),
            )
            .add_child(
                TextComponent::text("B")
                    .color_rgb(RGBColor::new(227, 106, 46))
                    .bold(),
            )
            .add_child(TextComponent::text("] ").color_rgb(RGBColor::new(215, 184, 153)));

        let version = env!("CARGO_PKG_VERSION");
        context.source.send_feedback(
            prefix.clone().add_child(
                TextComponent::text(format!("Version du serveur : Arklandia Engine {version} (basé sur Pumpkin)"))
                    .color_rgb(RGBColor::new(230, 161, 90)),
            ),
            false,
        );

        let mut message = prefix.add_child(
            TextComponent::text("Correctifs intégrés : ").color_rgb(RGBColor::new(230, 161, 90)),
        );
        for (index, id) in FIXES.iter().enumerate() {
            if index > 0 {
                message = message
                    .add_child(TextComponent::text(", ").color_rgb(RGBColor::new(215, 184, 153)));
            }
            message = message.add_child(
                TextComponent::text(format!("#{id}")).color_rgb(RGBColor::new(255, 209, 102)),
            );
        }
        context.source.send_feedback(message, false);
        Ok(FIXES.len() as i32)
    }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(
        PERMISSION,
        DESCRIPTION,
        PermissionDefault::Allow,
    ));

    dispatcher.register(
        command("arkbug", DESCRIPTION)
            .requires(PERMISSION)
            .executes(ArkBugExecutor),
    );
}
