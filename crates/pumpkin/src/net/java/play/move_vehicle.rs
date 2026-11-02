#[allow(clippy::wildcard_imports)]
use super::*;

impl JavaClient {
    pub fn handle_move_vehicle(&self, player: &Arc<Player>, packet: &SMoveVehicle) {
        // A movement packet was received this tick — tracked for SClientTickEnd zeroing.
        self.received_movement_this_tick
            .store(true, Ordering::Relaxed);
        let entity = player.get_entity();
        let last_pos = entity.pos.load();
        let pos = Vector3::new(packet.x, packet.y, packet.z);
        let vehicle = entity
            .vehicle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        if let Some(vehicle) = vehicle {
            let vehicle_entity = vehicle.get_entity();
            let old = vehicle_entity.last_sent_pos.load();
            vehicle_entity.set_pos(pos);
            vehicle_entity.set_rotation(packet.yaw, packet.pitch);
            vehicle_entity.on_ground.store(packet.on_ground, Ordering::Relaxed);
            // Boats do not run LivingEntity's movement broadcaster. Synchronize the mount
            // itself, independently of whether a recipient can see its controlling rider.
            // Absolute sync also avoids overflowing the relative i16 delta after a jump.
            if old.squared_distance_to_vec(&pos) >= 64.0 {
                vehicle_entity.teleport(pos, Some(packet.yaw), Some(packet.pitch), &vehicle_entity.world.load());
            } else {
                vehicle_entity.send_pos_rot();
            }
        }
        entity.set_pos(pos);
        let distance = last_pos.squared_distance_to_vec(&pos).sqrt();
        let cm = (distance * 100.0).round() as i32;
        if cm > 0 {
            let stat = player.get_movement_statistic();
            player.increment_stat(
                pumpkin_data::statistic::StatisticCategory::Custom,
                stat as i32,
                cm,
            );
        }
        chunker::update_position(player);
    }
}
