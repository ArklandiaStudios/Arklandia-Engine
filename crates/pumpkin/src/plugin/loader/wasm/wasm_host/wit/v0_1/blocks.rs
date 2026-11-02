use crate::plugin::loader::wasm::wasm_host::{
    state::PluginHostState, wit::v0_1::pumpkin::plugin::blocks::Host,
};
use pumpkin_data::{Block, BlockStateId};

impl Host for PluginHostState {
    async fn parse_block_state(&mut self, text: String) -> wasmtime::Result<Result<u16, String>> {
        Ok(parse_block_state(text.trim()))
    }

    async fn format_block_state(&mut self, state: u16) -> wasmtime::Result<String> {
        let Some(id) = BlockStateId::new(state) else {
            return Ok("minecraft:air".to_string());
        };
        let block = Block::from_state_id(id);
        let mut text = format!("minecraft:{}", block.name);
        if let Some(properties) = block.properties(id) {
            let properties = properties.to_props();
            if !properties.is_empty() {
                text.push('[');
                for (index, (name, value)) in properties.iter().enumerate() {
                    if index > 0 {
                        text.push(',');
                    }
                    text.push_str(name);
                    text.push('=');
                    text.push_str(value);
                }
                text.push(']');
            }
        }
        Ok(text)
    }
}

fn parse_block_state(text: &str) -> Result<u16, String> {
    let (name, properties) = match text.split_once('[') {
        Some((name, rest)) => {
            let inner = rest
                .strip_suffix(']')
                .ok_or_else(|| "missing ] in the block state".to_string())?;
            (name, inner)
        }
        None => (text, ""),
    };
    let block = Block::from_name(name.trim()).ok_or_else(|| format!("unknown block: {name}"))?;
    let mut wanted: Vec<(String, String)> = Vec::new();
    for pair in properties.split(',').filter(|pair| !pair.trim().is_empty()) {
        let (key, value) = pair
            .split_once('=')
            .ok_or_else(|| format!("invalid property: {pair}"))?;
        wanted.push((key.trim().to_string(), value.trim().to_string()));
    }
    if wanted.is_empty() {
        return Ok(block.default_state.id.as_u16());
    }
    // Start from the default value of every property, then apply the requested ones.
    let defaults = block
        .properties(block.default_state.id)
        .map(|properties| properties.to_props())
        .unwrap_or_default();
    let mut merged: Vec<(&str, &str)> = defaults.clone();
    for (key, value) in &wanted {
        let slot = merged
            .iter_mut()
            .find(|(name, _)| name == key)
            .ok_or_else(|| format!("{} has no property {key}", block.name))?;
        slot.1 = value.as_str();
    }
    block
        .state_from_properties(&merged)
        .map(|state| state.id.as_u16())
        .ok_or_else(|| format!("invalid value in {text}"))
}
