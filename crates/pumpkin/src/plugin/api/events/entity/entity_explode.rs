use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::{position::BlockPos, vector3::Vector3};
use std::sync::Arc;

/// An event that occurs when an entity explodes.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntityExplodeEvent {
    /// The ID of the exploding entity (0 when the explosion has no entity source).
    pub entity_id: i32,

    /// The position where the explosion occurred.
    pub position: Vector3<f64>,

    /// The yield rate of blocks destroyed.
    pub yield_rate: f32,

    /// Registry name of the exploding entity type (`minecraft:creeper`...); empty when unknown.
    pub entity_type: String,

    /// The world of the explosion, when known.
    pub world: Option<Arc<World>>,

    /// Blocks about to be destroyed. Plugins can remove entries (`event-extra`).
    pub blocks: Vec<BlockPos>,
}

impl EntityExplodeEvent {
    #[must_use]
    pub const fn new(entity_id: i32, position: Vector3<f64>, yield_rate: f32) -> Self {
        Self {
            entity_id,
            position,
            yield_rate,
            entity_type: String::new(),
            world: None,
            blocks: Vec::new(),
            cancelled: false,
        }
    }

    /// Attaches the source entity type, the world and the affected blocks.
    #[must_use]
    pub fn with_context(
        mut self,
        entity_type: String,
        world: Arc<World>,
        blocks: Vec<BlockPos>,
    ) -> Self {
        self.entity_type = entity_type;
        self.world = Some(world);
        self.blocks = blocks;
        self
    }
}
