use super::squid_swim::SquidMotion;
use crate::entity::{
    Entity, EntityBase,
    mob::{Mob, MobEntity},
};
use std::sync::Arc;

pub struct SquidEntity {
    pub mob_entity: MobEntity,
    motion: SquidMotion,
}
impl SquidEntity {
    pub fn new(entity: Entity) -> Arc<Self> {
        Arc::new(Self {
            mob_entity: MobEntity::new(entity),
            motion: SquidMotion::default(),
        })
    }
}
impl Mob for SquidEntity {
    fn get_mob_entity(&self) -> &MobEntity {
        &self.mob_entity
    }
    fn custom_server_ai_step(&self, _: &dyn EntityBase) {
        self.motion.tick(self);
    }
}
