use crate::{
    argument_types::argument_type::{ArgumentType, JavaClientArgumentType},
    context::command_context::CommandContext,
    errors::command_syntax_error::CommandSyntaxError,
    errors::error_types::CommandErrorType,
    string_reader::StringReader,
    suggestion::suggestions::{Suggestions, SuggestionsBuilder},
};
use std::collections::HashMap;

use crate::argument_types::block_predicate::parse_properties;
use pumpkin_data::{Block, BlockStateId, translation};
use pumpkin_util::text::TextComponent;

pub const INVALID_BLOCK_ERROR_TYPE: CommandErrorType<1> = CommandErrorType::new(
    translation::java::ARGUMENT_BLOCK_ID_INVALID,
    translation::java::ARGUMENT_BLOCK_ID_INVALID,
);

pub const ERROR_UNKNOWN_PROPERTY: CommandErrorType<2> = CommandErrorType::new(
    translation::java::ARGUMENT_BLOCK_PROPERTY_UNKNOWN,
    translation::java::ARGUMENT_BLOCK_PROPERTY_UNKNOWN,
);

pub const ERROR_INVALID_PROPERTY_VALUE: CommandErrorType<3> = CommandErrorType::new(
    translation::java::ARGUMENT_BLOCK_PROPERTY_INVALID,
    translation::java::ARGUMENT_BLOCK_PROPERTY_INVALID,
);

/// A block with the state selected by its `[property=value]` list.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BlockStateArgument {
    pub block: &'static Block,
    pub state_id: BlockStateId,
}

/// Resolves the state of `block` with the given properties set,
/// the other properties keeping their default value.
pub fn resolve_block_state<H: std::hash::BuildHasher>(
    reader: &StringReader,
    block: &'static Block,
    properties: &HashMap<String, String, H>,
) -> Result<BlockStateId, CommandSyntaxError> {
    if properties.is_empty() {
        return Ok(block.default_state.id);
    }
    let block_name = || TextComponent::text(format!("minecraft:{}", block.name));
    let mut state_props = block
        .properties(block.default_state.id)
        .map(|props| props.to_props())
        .unwrap_or_default();

    for (key, value) in properties {
        let Some(slot) = state_props.iter_mut().find(|prop| prop.0 == key.as_str()) else {
            return Err(ERROR_UNKNOWN_PROPERTY.create(
                reader,
                block_name(),
                TextComponent::text(key.clone()),
            ));
        };
        // Find the static spelling of the requested value among the block's states.
        let new_value = block.states.iter().find_map(|state| {
            block
                .properties(state.id)?
                .to_props()
                .into_iter()
                .find(|(k, v)| *k == key.as_str() && *v == value.as_str())
                .map(|(_, v)| v)
        });
        let Some(new_value) = new_value else {
            return Err(ERROR_INVALID_PROPERTY_VALUE.create(
                reader,
                block_name(),
                TextComponent::text(value.clone()),
                TextComponent::text(key.clone()),
            ));
        };
        slot.1 = new_value;
    }

    block
        .state_from_properties(&state_props)
        .map(|state| state.id)
        .ok_or_else(|| {
            let (key, value) = properties
                .iter()
                .next()
                .map(|(k, v)| (k.clone(), v.clone()))
                .unwrap_or_default();
            ERROR_INVALID_PROPERTY_VALUE.create(
                reader,
                block_name(),
                TextComponent::text(value),
                TextComponent::text(key),
            )
        })
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BlockArgumentType;

impl<S: crate::source::CommandSource> ArgumentType<S> for BlockArgumentType {
    type Item = BlockStateArgument;

    fn parse(&self, reader: &mut StringReader) -> Result<Self::Item, CommandSyntaxError> {
        let start = reader.cursor();
        while let Some(c) = reader.peek() {
            if c.is_alphanumeric() || c == '_' || c == ':' || c == '/' || c == '.' || c == '-' {
                reader.skip();
            } else {
                break;
            }
        }
        let block_name = &reader.string()[start..reader.cursor()];
        let normalized = if block_name.contains(':') {
            block_name.to_string()
        } else {
            format!("minecraft:{block_name}")
        };

        let block = Block::from_name(&normalized).ok_or_else(|| {
            INVALID_BLOCK_ERROR_TYPE.create(reader, TextComponent::text(normalized))
        })?;
        let properties = parse_properties(reader)?;
        let state_id = resolve_block_state(reader, block, &properties)?;

        Ok(BlockStateArgument { block, state_id })
    }

    fn client_side_parser(&'_ self) -> JavaClientArgumentType {
        JavaClientArgumentType::BlockState
    }

    fn list_suggestions(
        &self,
        _context: &CommandContext<S>,
        builder: SuggestionsBuilder,
    ) -> Suggestions {
        builder.build()
    }
}

impl BlockArgumentType {
    pub fn get<S: crate::source::CommandSource>(
        context: &CommandContext<S>,
        name: &str,
    ) -> Result<&'static Block, CommandSyntaxError> {
        Ok(Self::get_state(context, name)?.block)
    }

    pub fn get_state<S: crate::source::CommandSource>(
        context: &CommandContext<S>,
        name: &str,
    ) -> Result<BlockStateArgument, CommandSyntaxError> {
        context.get_argument::<BlockStateArgument>(name).copied()
    }
}
