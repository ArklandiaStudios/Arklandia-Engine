//! Native squid thrust: no terrestrial FloatGoal or water-walking input.
//! Extends ArkBug #4090 after the same surface drift was reproduced on squid.
use crate::entity::{EntityBase, mob::Mob};
use pumpkin_data::entity::{EntityStatus, EntityType};
use pumpkin_util::math::{vector3::Vector3, wrap_degrees};
use rand::RngExt;
use std::{
    f64::consts::{PI, TAU},
    sync::{Mutex, atomic::Ordering::Relaxed},
};

pub fn is_squid(kind: &EntityType) -> bool {
    kind == &EntityType::SQUID || kind == &EntityType::GLOW_SQUID
}
struct Motion {
    phase: f64,
    phase_speed: f64,
    direction: Vector3<f64>,
}
pub struct SquidMotion(Mutex<Motion>);
impl Default for SquidMotion {
    fn default() -> Self {
        Self(Mutex::new(Motion {
            phase: 0.0,
            phase_speed: 0.2 / (rand::random::<f64>() + 1.0),
            direction: Vector3::new(0.0, 0.0, 0.0),
        }))
    }
}
impl SquidMotion {
    pub fn tick(&self, squid: &dyn Mob) {
        let living = &squid.get_mob_entity().living_entity;
        let entity = &living.entity;
        living.jumping.store(false, Relaxed);
        living.movement_input.store(Vector3::new(0.0, 0.0, 0.0));
        if squid.get_mob_entity().is_no_ai() || living.health.load() <= 0.0 {
            if entity.touching_water.load(Relaxed) {
                entity.velocity.store(entity.velocity.load() * 0.9);
            }
            return;
        }
        let mut motion = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut rng = squid.get_random();
        motion.phase += motion.phase_speed;
        if motion.phase > TAU {
            motion.phase -= TAU;
            if rng.random_range(0..10) == 0 {
                motion.phase_speed = 0.2 / (rng.random::<f64>() + 1.0);
            }
            entity
                .world
                .load()
                .send_entity_status(entity, EntityStatus::SquidAnimSynch, None);
        }
        if !entity.touching_water.load(Relaxed) {
            motion.direction = Vector3::new(0.0, 0.0, 0.0);
            let velocity = entity.velocity.load();
            entity.velocity.store(Vector3::new(0.0, velocity.y, 0.0));
            return;
        }
        if rng.random_range(0..50) == 0 || motion.direction == Vector3::new(0.0, 0.0, 0.0) {
            let angle = rng.random::<f64>() * TAU;
            motion.direction = Vector3::new(
                angle.cos() * 0.2,
                rng.random::<f64>() * 0.2 - 0.1,
                angle.sin() * 0.2,
            );
        }
        // Mojang Squid.aiStep: a short thrust near the end of the first half
        // of each tentacle cycle, then a drag phase in the second half.
        if motion.phase < PI {
            if motion.phase / PI > 0.75 {
                entity.velocity.store(motion.direction);
            }
        } else {
            entity.velocity.store(entity.velocity.load() * 0.9);
        }
        let velocity = entity.velocity.load();
        if velocity.x.hypot(velocity.z) > 1.0e-6 {
            let target_yaw = (-velocity.x.atan2(velocity.z).to_degrees()) as f32;
            let yaw = entity.yaw.load() + wrap_degrees(target_yaw - entity.yaw.load()) * 0.1;
            entity.yaw.store(yaw);
            entity.body_yaw.store(yaw);
            entity.head_yaw.store(yaw);
        }
    }
}

pub fn travel(caller: &dyn EntityBase) {
    let entity = caller.get_entity();
    entity.move_entity(caller, entity.velocity.load());
}
