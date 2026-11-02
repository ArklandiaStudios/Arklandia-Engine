use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::vector3::Vector3;
use std::sync::Arc;

/// An event that occurs when an entity is spawned into a world.
#[cancellable]
#[derive(Event, Clone)]
pub struct EntitySpawnEvent {
    /// The ID of the spawned entity.
    pub entity_id: i32,

    /// The registry name of the entity type (`minecraft:zombie`).
    pub entity_type: String,

    /// The position where the entity is spawned.
    pub position: Vector3<f64>,

    /// The world in which the entity is spawned.
    pub world: Arc<World>,

    /// Why the entity spawns (`NATURAL`, `SPAWNER`, `SPAWNER_EGG`, `COMMAND`, `BREEDING`...).
    pub spawn_reason: String,
}

impl EntitySpawnEvent {
    #[must_use]
    pub fn new(
        entity_id: i32,
        entity_type: String,
        position: Vector3<f64>,
        world: Arc<World>,
    ) -> Self {
        Self {
            entity_id,
            entity_type,
            position,
            world,
            spawn_reason: "DEFAULT".to_string(),
            cancelled: false,
        }
    }

    /// Sets the spawn reason.
    #[must_use]
    pub fn with_reason(mut self, reason: &str) -> Self {
        self.spawn_reason = reason.to_string();
        self
    }
}
