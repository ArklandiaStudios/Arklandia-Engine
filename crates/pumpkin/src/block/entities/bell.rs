use crate::block::entities::BlockEntity;
use crate::entity::EntityBase;
use crate::world::World;
use crossbeam::atomic::AtomicCell;
use pumpkin_data::HorizontalFacingExt;
use pumpkin_data::block_properties::HorizontalFacing;
use pumpkin_data::effect::StatusEffect;
use pumpkin_data::potion::Effect;
use pumpkin_data::sound::{Sound, SoundCategory};
use pumpkin_data::tag::{self, Taggable};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::boundingbox::BoundingBox;
use pumpkin_util::math::position::BlockPos;
use std::any::Any;
use std::sync::{Arc, Mutex};

/// Range in which raiders make a ringing bell resonate.
const HEARING_RANGE: f64 = 32.0;
/// Range in which raiders are revealed when the bell stops resonating.
const HIGHLIGHT_RANGE: f64 = 48.0;
/// Duration of the Glowing effect given to revealed raiders.
const GLOW_DURATION: i32 = 60;

pub struct BellBlockEntity {
    pub position: BlockPos,
    pub last_side_hit: AtomicCell<Option<HorizontalFacing>>,
    pub ring_ticks: AtomicCell<i32>,
    pub ringing: AtomicCell<bool>,
    resonating: AtomicCell<bool>,
    resonate_time: AtomicCell<i32>,
    /// Living entities near the bell when it was last rung.
    nearby_entities: Mutex<Vec<Arc<dyn EntityBase>>>,
}

impl BellBlockEntity {
    pub const ID: &'static str = "minecraft:bell";
    #[must_use]
    pub const fn new(position: BlockPos) -> Self {
        Self {
            position,
            last_side_hit: AtomicCell::new(None),
            ring_ticks: AtomicCell::new(0),
            resonate_time: AtomicCell::new(0),
            resonating: AtomicCell::new(false),
            ringing: AtomicCell::new(false),
            nearby_entities: Mutex::new(Vec::new()),
        }
    }
    pub fn activate(&self, world: &World, direction: HorizontalFacing) {
        self.last_side_hit.store(Some(direction));
        if self.ringing.load() {
            self.ring_ticks.store(0);
        } else {
            self.ringing.store(true);
        }
        self.update_entities(world);
        world.add_synced_block_event(self.position, 1, direction.to_block_direction().to_index());
    }

    /// Vanilla `BellBlockEntity.updateEntities`: remembers the living entities around the bell.
    fn update_entities(&self, world: &World) {
        let search_box = BoundingBox::from_block(&self.position).expand(
            HIGHLIGHT_RANGE,
            HIGHLIGHT_RANGE,
            HIGHLIGHT_RANGE,
        );
        let entities = world
            .get_entities_at_box(&search_box)
            .into_iter()
            .filter(|entity| entity.get_living_entity().is_some())
            .collect();
        *self
            .nearby_entities
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = entities;
    }

    fn is_raider_within(&self, entity: &dyn EntityBase, range: f64) -> bool {
        let entity = entity.get_entity();
        entity.is_alive()
            && !entity.is_removed()
            && entity
                .entity_type
                .has_tag(&tag::EntityType::MINECRAFT_RAIDERS)
            && self
                .position
                .to_centered_f64()
                .squared_distance_to_vec(&entity.pos.load())
                < range * range
    }

    /// Vanilla `BellBlockEntity.areRaidersNearby`.
    pub fn raiders_hear_bell(&self) -> bool {
        self.nearby_entities
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .any(|entity| self.is_raider_within(entity.as_ref(), HEARING_RANGE))
    }

    /// Vanilla `BellBlockEntity.makeRaidersGlow`, run when the bell stops resonating.
    fn make_raiders_glow(&self) {
        let entities = self
            .nearby_entities
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        for entity in entities {
            if !self.is_raider_within(entity.as_ref(), HIGHLIGHT_RANGE) {
                continue;
            }
            if let Some(living) = entity.get_living_entity() {
                living.add_effect(Effect {
                    effect_type: &StatusEffect::GLOWING,
                    duration: GLOW_DURATION,
                    amplifier: 0,
                    ambient: false,
                    show_particles: true,
                    show_icon: true,
                    blend: false,
                });
            }
        }
    }
}

impl BlockEntity for BellBlockEntity {
    fn write_nbt(&self, _nbt: &mut NbtCompound) {}

    fn from_nbt(_nbt: &NbtCompound, position: BlockPos) -> Self
    where
        Self: Sized,
    {
        Self::new(position)
    }

    fn tick(&self, world: &Arc<World>) {
        if self.ringing.load() {
            self.ring_ticks.fetch_add(1);
        }
        if self.ring_ticks.load() >= 50 {
            self.ringing.store(false);
            self.ring_ticks.store(0);
        }
        if self.ring_ticks.load() >= 5 && self.resonate_time.load() == 0 && self.raiders_hear_bell()
        {
            self.resonating.store(true);
            world.play_sound_fine(
                Sound::BlockBellResonate,
                SoundCategory::Blocks,
                &self.position.to_f64(),
                1.0,
                1.0,
            );
        }

        if self.resonating.load() {
            if self.resonate_time.load() < 40 {
                self.resonate_time.fetch_add(1);
            } else {
                self.make_raiders_glow();
                self.resonating.store(false);
            }
        }
    }

    fn resource_location(&self) -> &'static str {
        Self::ID
    }

    fn get_position(&self) -> BlockPos {
        self.position
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}
