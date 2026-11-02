use pumpkin_data::translation;
use pumpkin_util::PermissionLvl;
use pumpkin_util::permission::{Permission, PermissionDefault, PermissionRegistry};
use pumpkin_util::text::TextComponent;

use crate::command::argument_builder::{ArgumentBuilder, argument, command};
use crate::command::argument_types::core::string::StringArgumentType;
use crate::command::context::command_context::CommandContext;
use crate::command::errors::error_types::{CommandErrorType, LiteralCommandErrorType};
use crate::command::node::dispatcher::CommandDispatcher;
use crate::command::node::{CommandExecutor, CommandExecutorResult};
use crate::command::suggestion::provider::{SuggestionProvider, SuggestionProviderResult};
use crate::command::suggestion::suggestions::SuggestionsBuilder;
use crate::data::datapack::function_chain;

const DESCRIPTION: &str = "Runs commands found in the corresponding function files.";
const PERMISSION: &str = "minecraft:command.function";

static ERROR_UNKNOWN_FUNCTION: CommandErrorType<1> = CommandErrorType::new(
    translation::java::ARGUMENTS_FUNCTION_UNKNOWN,
    translation::java::ARGUMENTS_FUNCTION_UNKNOWN,
);

static ERROR_CHAIN_LIMIT: LiteralCommandErrorType =
    LiteralCommandErrorType::new("ArkBug: limite d’exécution des fonctions atteinte.");

// Functions are identifiers, optionally prefixed by '#', rather than Brigadier words.
struct FunctionNameArgument;
impl pumpkin_command::argument_types::argument_type::ArgumentType<crate::command::CommandSource>
    for FunctionNameArgument
{
    type Item = String;
    fn parse(
        &self,
        reader: &mut pumpkin_command::string_reader::StringReader,
    ) -> Result<String, pumpkin_command::errors::command_syntax_error::CommandSyntaxError> {
        use pumpkin_command::argument_types::FromStringReader;
        let tag = reader.peek() == Some('#');
        if tag {
            reader.skip();
        }
        let id = pumpkin_util::identifier::Identifier::from_reader(reader)?;
        Ok(format!("{}{}", if tag { "#" } else { "" }, id))
    }
    fn client_side_parser(
        &self,
    ) -> pumpkin_command::argument_types::argument_type::JavaClientArgumentType {
        pumpkin_command::argument_types::argument_type::JavaClientArgumentType::Function
    }
}

struct FunctionSuggestionProvider;

impl SuggestionProvider for FunctionSuggestionProvider {
    fn suggest(
        &self,
        context: &CommandContext,
        mut builder: SuggestionsBuilder,
    ) -> SuggestionProviderResult {
        let server = context.server();
        let function_names = server.datapack_manager.get_function_names();
        for name in function_names {
            builder = builder.suggest(name);
        }
        builder.build()
    }
}

struct FunctionExecutor;

impl CommandExecutor for FunctionExecutor {
    fn execute(&self, context: &CommandContext) -> CommandExecutorResult {
        let name_str = StringArgumentType::get(context, "name")?;
        let server = context.server();

        let executed_count =
            match server
                .datapack_manager
                .execute_function(server, &context.source, name_str)
            {
                Ok(count) => count,
                Err(reason) if reason.starts_with("ArkBug:") => {
                    // Report once at the root, rather than once per recursion level.
                    if function_chain::active() {
                        return Ok(0);
                    }
                    return Err(ERROR_CHAIN_LIMIT.create_without_context());
                }
                Err(_) => {
                    return Err(ERROR_UNKNOWN_FUNCTION
                        .create_without_context(TextComponent::text(name_str.to_string())));
                }
            };
        if function_chain::active() {
            return Ok(executed_count as i32);
        }

        if name_str.starts_with('#') {
            context.source.send_feedback(
                TextComponent::translate_cross(
                    translation::java::COMMANDS_FUNCTION_SUCCESS_MULTIPLE,
                    translation::java::COMMANDS_FUNCTION_SUCCESS_MULTIPLE,
                    [
                        TextComponent::text(executed_count.to_string()),
                        TextComponent::text(name_str.to_string()),
                    ],
                ),
                true,
            );
        } else {
            context.source.send_feedback(
                TextComponent::translate_cross(
                    translation::java::COMMANDS_FUNCTION_SUCCESS_SINGLE,
                    translation::java::COMMANDS_FUNCTION_SUCCESS_SINGLE,
                    [
                        TextComponent::text(executed_count.to_string()),
                        TextComponent::text(name_str.to_string()),
                    ],
                ),
                true,
            );
        }

        Ok(executed_count as i32)
    }
}

pub fn register(dispatcher: &mut CommandDispatcher, registry: &PermissionRegistry) {
    registry.register_permission_or_panic(Permission::new(
        PERMISSION,
        DESCRIPTION,
        PermissionDefault::Op(PermissionLvl::Two),
    ));

    dispatcher.register(
        command("function", DESCRIPTION).requires(PERMISSION).then(
            argument("name", FunctionNameArgument)
                .suggests(FunctionSuggestionProvider)
                .executes(FunctionExecutor),
        ),
    );
}
