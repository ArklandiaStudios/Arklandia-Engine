use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::position::BlockPos;
use std::sync::Arc;

/// An event that occurs when a splash potion hits something.
#[cancellable]
#[derive(Event, Clone)]
pub struct PotionSplashEvent {
    /// The ID of the potion entity.
    pub entity_id: i32,

    /// Where the potion splashed.
    pub location: BlockPos,

    /// The potion item.
    pub potion_item: String,

    /// IDs of the entities affected by the potion.
    pub affected_entities: Vec<i32>,

    /// The world of the potion, when known.
    pub world: Option<Arc<World>>,
}

impl PotionSplashEvent {
    #[must_use]
    pub const fn new(
        entity_id: i32,
        location: BlockPos,
        potion_item: String,
        affected_entities: Vec<i32>,
    ) -> Self {
        Self {
            entity_id,
            location,
            potion_item,
            affected_entities,
            world: None,
            cancelled: false,
        }
    }

    /// Attaches the world the potion is in.
    #[must_use]
    pub fn with_world(mut self, world: Arc<World>) -> Self {
        self.world = Some(world);
        self
    }
}
