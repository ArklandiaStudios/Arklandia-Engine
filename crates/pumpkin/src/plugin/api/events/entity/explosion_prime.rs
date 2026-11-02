use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use std::sync::Arc;

/// An event that occurs when an entity is about to explode.
#[cancellable]
#[derive(Event, Clone)]
pub struct ExplosionPrimeEvent {
    /// The ID of the entity.
    pub entity_id: i32,

    /// The radius of the explosion.
    pub radius: f32,

    /// Whether the explosion sets fire.
    pub fire: bool,

    /// The world of the entity, when known.
    pub world: Option<Arc<World>>,
}

impl ExplosionPrimeEvent {
    #[must_use]
    pub const fn new(entity_id: i32, radius: f32, fire: bool) -> Self {
        Self {
            entity_id,
            radius,
            fire,
            world: None,
            cancelled: false,
        }
    }

    /// Attaches the world of the entity.
    #[must_use]
    pub fn with_world(mut self, world: Arc<World>) -> Self {
        self.world = Some(world);
        self
    }
}
