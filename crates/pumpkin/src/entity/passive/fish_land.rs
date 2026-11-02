//! Shared fish air supply and small land flops (ArkBug #4063).
use crate::entity::mob::Mob;
use pumpkin_data::{
    damage::DamageType,
    entity::EntityType,
    sound::{Sound, SoundCategory},
};
use pumpkin_nbt::compound::NbtCompound;
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;
use std::sync::atomic::{AtomicI32, Ordering::Relaxed};

pub struct FishAir {
    remaining: AtomicI32,
}
impl Default for FishAir {
    fn default() -> Self {
        Self {
            remaining: AtomicI32::new(300),
        }
    }
}
impl FishAir {
    pub fn tick(&self, fish: &dyn Mob) {
        let living = &fish.get_mob_entity().living_entity;
        let entity = &living.entity;
        if living.health.load() <= 0.0 || entity.touching_water.load(Relaxed) {
            self.remaining.store(300, Relaxed);
        } else if self.remaining.fetch_sub(1, Relaxed) - 1 <= -20 {
            self.remaining.store(0, Relaxed);
            fish.damage(fish, 2.0, DamageType::DROWN);
        }
    }
    pub fn write(&self, nbt: &mut NbtCompound) {
        nbt.put_short("Air", self.remaining.load(Relaxed) as i16);
    }
    pub fn read(&self, nbt: &NbtCompound) {
        if let Some(air) = nbt.get_short("Air") {
            self.remaining
                .store(i32::from(air).clamp(-20, 300), Relaxed);
        }
    }
}

pub fn land_step(fish: &dyn Mob) {
    let mob = fish.get_mob_entity();
    let living = &mob.living_entity;
    let entity = &living.entity;
    if entity.touching_water.load(Relaxed) {
        return;
    }
    // A fish cannot walk towards a land path or retain a previous swimming input.
    mob.navigator
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .stop();
    living.movement_input.store(Vector3::new(0.0, 0.0, 0.0));
    living.jumping.store(false, Relaxed);
    if !mob.is_no_ai() && living.health.load() > 0.0 && entity.on_ground.load(Relaxed) {
        let mut rng = fish.get_random();
        let impulse = Vector3::new(
            (rng.random::<f64>() * 2.0 - 1.0) * 0.05,
            0.4,
            (rng.random::<f64>() * 2.0 - 1.0) * 0.05,
        );
        entity.set_velocity(entity.velocity.load() + impulse);
        entity.on_ground.store(false, Relaxed);
        let sound = match entity.entity_type {
            kind if kind == &EntityType::COD => Sound::EntityCodFlop,
            kind if kind == &EntityType::SALMON => Sound::EntitySalmonFlop,
            kind if kind == &EntityType::PUFFERFISH => Sound::EntityPufferFishFlop,
            _ => Sound::EntityTropicalFishFlop,
        };
        entity
            .world
            .load()
            .play_sound(sound, SoundCategory::Neutral, &entity.pos.load());
    }
}
