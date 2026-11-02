use crate::world::World;
use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

/// An event that occurs when a block is fading.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockFadeEvent {
    /// The position of the block.
    pub block_pos: BlockPos,

    /// The block involved.
    pub block: &'static Block,

    /// World in which the change happens, when known.
    pub world: Option<Arc<World>>,

    /// Block present before the change, when known.
    pub old_block: Option<&'static Block>,

    /// Block present after the change, when known.
    pub new_block: Option<&'static Block>,
}

impl BlockFadeEvent {
    #[must_use]
    pub const fn new(block_pos: BlockPos, block: &'static Block) -> Self {
        Self {
            block_pos,
            block,
            world: None,
            old_block: None,
            new_block: None,
            cancelled: false,
        }
    }

    /// Attaches the world and the old/new block of the change.
    #[must_use]
    pub fn with_change(
        mut self,
        world: Arc<World>,
        old_block: &'static Block,
        new_block: &'static Block,
    ) -> Self {
        self.world = Some(world);
        self.old_block = Some(old_block);
        self.new_block = Some(new_block);
        self
    }
}
