use crate::world::World;
use pumpkin_macros::{Event, cancellable};
use pumpkin_util::math::vector3::Vector3;
use std::sync::Arc;

/// An event that occurs when lightning strikes.
#[cancellable]
#[derive(Event, Clone)]
pub struct LightningStrikeEvent {
    /// Where the lightning strikes.
    pub position: Vector3<f64>,

    /// Whether the lightning is only a visual effect.
    pub is_effect: bool,

    /// The world of the lightning, when known.
    pub world: Option<Arc<World>>,

    /// Cause of the lightning (`WEATHER`, `COMMAND`, `CUSTOM`...); empty when unknown.
    pub cause: String,
}

impl LightningStrikeEvent {
    #[must_use]
    pub const fn new(position: Vector3<f64>, is_effect: bool) -> Self {
        Self {
            position,
            is_effect,
            world: None,
            cause: String::new(),
            cancelled: false,
        }
    }

    /// Attaches the world and the cause of the lightning.
    #[must_use]
    pub fn with_world(mut self, world: Arc<World>, cause: &str) -> Self {
        self.world = Some(world);
        self.cause = cause.to_string();
        self
    }
}
