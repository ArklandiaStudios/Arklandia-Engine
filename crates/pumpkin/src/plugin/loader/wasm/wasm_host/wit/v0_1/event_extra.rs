use crate::plugin::loader::wasm::wasm_host::{
    state::PluginHostState,
    wit::v0_1::{
        events::{from_wasm_block_position, to_wasm_block_position},
        pumpkin::plugin::{common::BlockPos, event_extra::Host},
    },
};

impl Host for PluginHostState {
    async fn get_attribute(&mut self, key: String) -> wasmtime::Result<Option<String>> {
        Ok(self.event_ctx.last().and_then(|ctx| {
            ctx.attributes
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value.clone())
        }))
    }

    async fn get_attributes(&mut self) -> wasmtime::Result<Vec<(String, String)>> {
        Ok(self
            .event_ctx
            .last()
            .map(|ctx| ctx.attributes.clone())
            .unwrap_or_default())
    }

    async fn get_blocks(&mut self) -> wasmtime::Result<Vec<BlockPos>> {
        Ok(self
            .event_ctx
            .last()
            .map(|ctx| ctx.blocks.iter().copied().map(to_wasm_block_position).collect())
            .unwrap_or_default())
    }

    async fn remove_blocks(&mut self, blocks: Vec<BlockPos>) -> wasmtime::Result<()> {
        if let Some(ctx) = self.event_ctx.last_mut() {
            ctx.removed
                .extend(blocks.into_iter().map(from_wasm_block_position));
        }
        Ok(())
    }
}
