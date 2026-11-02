use std::sync::{Arc, Weak};

use pumpkin_data::entity::EntityType;

use crate::entity::{
    Entity,
    ai::goal::{look_around::RandomLookAroundGoal, look_at_entity::LookAtEntityGoal},
    mob::{Mob, MobEntity},
};

pub struct TropicalFishEntity {
    pub mob_entity: MobEntity,
    air: super::fish_land::FishAir,
}

impl TropicalFishEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        let mut mob_entity = MobEntity::new(entity);
        super::fish_swim::configure(&mut mob_entity);
        let tropical_fish = Self {
            mob_entity,
            air: super::fish_land::FishAir::default(),
        };
        let mob_arc = Arc::new(tropical_fish);
        let mob_weak: Weak<dyn Mob> = {
            let mob_arc: Arc<dyn Mob> = mob_arc.clone();
            Arc::downgrade(&mob_arc)
        };

        {
            let mut goal_selector = mob_arc
                .mob_entity
                .goals_selector
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            goal_selector.add_goal(1, Box::new(super::fish_swim::FishSwimGoal::default()));
            goal_selector.add_goal(
                2,
                LookAtEntityGoal::with_default(mob_weak, &EntityType::PLAYER, 6.0),
            );
            goal_selector.add_goal(3, Box::new(RandomLookAroundGoal::default()));
        };

        mob_arc
    }
}

impl Mob for TropicalFishEntity {
    fn custom_server_ai_step(&self, _: &dyn crate::entity::EntityBase) {
        super::fish_land::land_step(self);
        super::fish_swim::swimming_step(self);
    }
    fn post_tick(&self) {
        self.air.tick(self);
    }
    fn mob_write_nbt(&self, nbt: &mut pumpkin_nbt::compound::NbtCompound) {
        self.air.write(nbt);
    }
    fn mob_read_nbt(&self, nbt: &pumpkin_nbt::compound::NbtCompound) {
        self.air.read(nbt);
    }

    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }
}
