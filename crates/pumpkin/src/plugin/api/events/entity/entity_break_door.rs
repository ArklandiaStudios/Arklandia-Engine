use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

/// An event that occurs when an entity breaks a door.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityBreakDoorEvent {
    /// The ID of the entity.
    pub entity_id: i32,

    /// The position of the door.
    pub block_pos: BlockPos,

    /// The world of the door, when known.
    pub world: Option<Arc<World>>,
}

impl EntityBreakDoorEvent {
    #[must_use]
    pub const fn new(entity_id: i32, block_pos: BlockPos) -> Self {
        Self {
            entity_id,
            block_pos,
            world: None,
            cancelled: false,
        }
    }

    /// Attaches the world of the door.
    #[must_use]
    pub fn with_world(mut self, world: Arc<World>) -> Self {
        self.world = Some(world);
        self
    }
}
