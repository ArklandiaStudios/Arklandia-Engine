use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

/// An event that occurs when a TNT block is primed.
#[cancellable]
#[derive(Event, Clone)]
pub struct TNTPrimeEvent {
    /// The position of the TNT block.
    pub block_pos: BlockPos,

    /// The reason the TNT was primed.
    pub prime_reason: String,

    /// The world of the TNT block, when known.
    pub world: Option<Arc<World>>,
}

impl TNTPrimeEvent {
    #[must_use]
    pub const fn new(block_pos: BlockPos, prime_reason: String) -> Self {
        Self {
            block_pos,
            prime_reason,
            world: None,
            cancelled: false,
        }
    }

    /// Attaches the world the TNT is in.
    #[must_use]
    pub fn with_world(mut self, world: Arc<World>) -> Self {
        self.world = Some(world);
        self
    }
}
