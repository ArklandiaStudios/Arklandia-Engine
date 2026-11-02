use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

/// An event that occurs when a block explodes.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockExplodeEvent {
    /// The center of the explosion.
    pub block_pos: BlockPos,

    /// The yield rate of the destroyed blocks.
    pub yield_rate: f32,

    /// The world of the explosion, when known.
    pub world: Option<Arc<World>>,

    /// Blocks about to be destroyed. Plugins can remove entries (`event-extra`).
    pub blocks: Vec<BlockPos>,
}

impl BlockExplodeEvent {
    #[must_use]
    pub const fn new(block_pos: BlockPos, yield_rate: f32) -> Self {
        Self {
            block_pos,
            yield_rate,
            world: None,
            blocks: Vec::new(),
            cancelled: false,
        }
    }

    /// Attaches the world and the list of affected blocks.
    #[must_use]
    pub fn with_context(mut self, world: Arc<World>, blocks: Vec<BlockPos>) -> Self {
        self.world = Some(world);
        self.blocks = blocks;
        self
    }
}
