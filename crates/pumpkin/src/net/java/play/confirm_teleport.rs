#[allow(clippy::wildcard_imports)]
use super::*;

impl JavaClient {
    pub fn handle_confirm_teleport(&self, player: &Player, confirm_teleport: &SConfirmTeleport) {
        enum TeleportResult {
            Success,
            Superseded,
            WrongId,
            NotTeleporting,
        }

        let result = {
            let mut awaiting_teleport = player
                .awaiting_teleport
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some((id, position)) = awaiting_teleport.as_ref() {
                if id == &confirm_teleport.teleport_id {
                    // We should set the position now to what we requested in the teleport packet.
                    // This may fix issues when the client sends the position while being teleported.
                    player.get_entity().set_pos(*position);
                    *awaiting_teleport = None;
                    TeleportResult::Success
                } else if player.teleport_id_count.load(Ordering::Relaxed)
                    .wrapping_sub(confirm_teleport.teleport_id.0) >= 0
                {
                    // A newer teleport may supersede a freeze correction before its
                    // acknowledgement arrives. Never apply the older destination.
                    TeleportResult::Superseded
                } else {
                    TeleportResult::WrongId
                }
            } else if player.teleport_id_count.load(Ordering::Relaxed)
                .wrapping_sub(confirm_teleport.teleport_id.0) >= 0
            {
                // Delayed/duplicate acknowledgements for already issued requests.
                TeleportResult::Superseded
            } else {
                TeleportResult::NotTeleporting
            }
        };

        match result {
            TeleportResult::Success | TeleportResult::Superseded => {}
            TeleportResult::WrongId => {
                self.try_kick(&TextComponent::text("Wrong teleport id"));
            }
            TeleportResult::NotTeleporting => {
                self.try_kick(&TextComponent::text(
                    "Send Teleport confirm, but we did not teleport",
                ));
            }
        }
    }
}
