//! Water-only navigation and movement for the four fish species (#4090).
use crate::entity::{
    EntityBase,
    ai::{
        goal::{Controls, Goal},
        pathfinder::{Navigator, NavigatorGoal},
    },
    mob::{Mob, MobEntity},
};
use pumpkin_data::{entity::EntityType, fluid::Fluid};
use pumpkin_util::math::vector3::Vector3;
use rand::RngExt;
use std::sync::atomic::Ordering::Relaxed;

pub fn is_fish(kind: &EntityType) -> bool {
    kind == &EntityType::COD
        || kind == &EntityType::SALMON
        || kind == &EntityType::TROPICAL_FISH
        || kind == &EntityType::PUFFERFISH
}
pub fn configure(mob: &mut MobEntity) {
    let size = mob.living_entity.entity.entity_dimension.load();
    let navigator = mob
        .navigator
        .get_mut()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    *navigator = Navigator::water_bound(false);
    navigator.set_mob_dimensions(size.width, size.height);
    navigator.set_can_float(true);
}

#[derive(Default)]
pub struct FishSwimGoal {
    target: Option<Vector3<f64>>,
}
impl Goal for FishSwimGoal {
    fn can_start(&mut self, fish: &dyn Mob) -> bool {
        let mob = fish.get_mob_entity();
        let entity = &mob.living_entity.entity;
        if mob.is_no_ai()
            || !entity.touching_water.load(Relaxed)
            || fish.get_random().random_range(0..20) != 0
        {
            return false;
        }
        let pos = entity.block_pos.load();
        let world = entity.world.load();
        let mut rng = fish.get_random();
        self.target = (0..10).find_map(|_| {
            let candidate = pos.offset(Vector3::new(
                rng.random_range(-5..=5),
                rng.random_range(-3..=3),
                rng.random_range(-5..=5),
            ));
            (world.get_fluid(&candidate).matches_type(&Fluid::WATER)
                && world
                    .get_block_state(&candidate)
                    .get_block_collision_shapes_at(&candidate)
                    .next()
                    .is_none())
            .then(|| {
                Vector3::new(
                    f64::from(candidate.0.x) + 0.5,
                    f64::from(candidate.0.y) + 0.5,
                    f64::from(candidate.0.z) + 0.5,
                )
            })
        });
        self.target.is_some()
    }
    fn start(&mut self, fish: &dyn Mob) {
        if let Some(target) = self.target {
            let mob = fish.get_mob_entity();
            mob.navigator
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .set_progress(NavigatorGoal::new(
                    mob.living_entity.entity.pos.load(),
                    target,
                    1.0,
                ));
        }
    }
    fn should_continue(&mut self, fish: &dyn Mob) -> bool {
        !fish.get_mob_entity().is_no_ai()
            && fish.get_entity().touching_water.load(Relaxed)
            && !fish.is_navigator_idle()
    }
    fn stop(&mut self, fish: &dyn Mob) {
        self.target = None;
        fish.get_mob_entity()
            .navigator
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .stop();
    }
    fn controls(&self) -> Controls {
        Controls::MOVE
    }
}

pub fn swimming_step(fish: &dyn Mob) {
    let mob = fish.get_mob_entity();
    let living = &mob.living_entity;
    let entity = &living.entity;
    if !entity.touching_water.load(Relaxed) {
        return;
    }
    living.jumping.store(false, Relaxed);
    if mob.is_no_ai() {
        living.movement_input.store(Vector3::new(0.0, 0.0, 0.0));
        return;
    }
    let mut velocity = entity.velocity.load();
    if entity.is_submerged_in_water() {
        velocity.y += 0.005;
    }
    // The existing water navigator supplies a signed vertical steering input.
    let mut input = living.movement_input.load();
    velocity.y += input.y * 0.1;
    input.y = 0.0;
    living.movement_input.store(input);
    entity.velocity.store(velocity);
}

pub fn travel(caller: &dyn EntityBase, input: Vector3<f64>) {
    let entity = caller.get_entity();
    entity.update_velocity_from_input(input, 0.01);
    entity.move_entity(caller, entity.velocity.load());
    let mut velocity = entity.velocity.load() * 0.9;
    velocity.y -= 0.005;
    entity.velocity.store(velocity);
}
