use crate::entity::player::Player;
use crate::world::World;
use pumpkin_data::Block;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

/// An event that occurs when a block is ignited.
#[cancellable]
#[derive(Event, Clone)]
pub struct BlockIgniteEvent {
    /// The position of the block being ignited.
    pub block_pos: BlockPos,

    /// The block that is igniting the target (fire, lava, flint and steel...).
    pub igniting_block: &'static Block,

    /// The player who ignited the block, if any.
    pub player: Option<Arc<Player>>,

    /// The world in which the block is ignited, when known.
    pub world: Option<Arc<World>>,

    /// Cause of the ignition (`LAVA`, `SPREAD`, `FLINT_AND_STEEL`, `FIREBALL`...); empty when unknown.
    pub cause: String,
}

impl BlockIgniteEvent {
    #[must_use]
    pub const fn new(
        block_pos: BlockPos,
        igniting_block: &'static Block,
        player: Option<Arc<Player>>,
    ) -> Self {
        Self {
            block_pos,
            igniting_block,
            player,
            world: None,
            cause: String::new(),
            cancelled: false,
        }
    }

    /// Attaches the world and the cause of the ignition.
    #[must_use]
    pub fn with_world(mut self, world: Arc<World>, cause: &str) -> Self {
        self.world = Some(world);
        self.cause = cause.to_string();
        self
    }
}
