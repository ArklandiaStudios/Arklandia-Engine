use crate::world::World;
use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

use super::BlockEvent;

/// An event that occurs when a block is destroyed as a result of being burnt by fire.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockBurnEvent {
    /// The block that ignited the burning block.
    pub igniting_block: &'static Block,

    /// The block that is burning.
    pub block: &'static Block,

    /// Position of the burning block, when known.
    pub pos: Option<BlockPos>,

    /// World in which the block burns, when known.
    pub world: Option<Arc<World>>,
}

impl BlockBurnEvent {
    #[must_use]
    pub fn new(
        igniting_block: &'static Block,
        block: &'static Block,
        pos: BlockPos,
        world: Arc<World>,
    ) -> Self {
        Self {
            igniting_block,
            block,
            pos: Some(pos),
            world: Some(world),
            cancelled: false,
        }
    }
}

impl BlockEvent for BlockBurnEvent {
    fn get_block(&self) -> &Block {
        self.block
    }
}
