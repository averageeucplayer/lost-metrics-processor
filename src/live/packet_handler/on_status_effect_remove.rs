use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{on_shield_change, parse_pkt1};
use anyhow::Ok;
use chrono::Utc;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{EntityType, StatusEffectTargetType, StatusEffectType};
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::definitions::*;
use lost_metrics_store::encounter_service::EncounterService;

use super::DefaultPacketHandler;

impl<FL, DH, SA, RS, LP, EE, ES> DefaultPacketHandler<FL, DH, SA, RS, LP, EE, ES>
where
    FL: Flags,
    DH: DamageEncryptionHandlerTrait,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    ES: EncounterService {
    pub fn on_status_effect_remove(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = parse_pkt1(&data, PKTStatusEffectRemoveNotify::new)?;

        let (is_shield, shields_broken, effects_removed, _left_workshop) =
        self.trackers.borrow().status_tracker.borrow_mut().remove_status_effects(
            packet.object_id,
            packet.status_effect_instance_ids,
            packet.reason,
            StatusEffectTargetType::Local,
        );
        
        if is_shield {
            if shields_broken.is_empty() {
                let target = self.trackers.borrow_mut().entity_tracker.get_source_entity(packet.object_id);
                state.on_boss_shield(&target, 0);
            } else {
                for status_effect in shields_broken {
                    let change = status_effect.value;
                    on_shield_change(
                        &mut self.trackers.borrow_mut().entity_tracker,
                        &self.trackers.borrow().id_tracker,
                        state,
                        status_effect,
                        change,
                    );
                }
            }
        }
        
        let now = Utc::now().timestamp_millis();
        for effect_removed in effects_removed {
            if effect_removed.status_effect_type == StatusEffectType::HardCrowdControl {
                let target = self.trackers.borrow_mut().entity_tracker.get_source_entity(effect_removed.target_id);
                if target.entity_type == EntityType::Player {
                    state.on_cc_removed(&target, &effect_removed, now);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::PacketHandlerBuilder;

    #[tokio::test]
    async fn test() {
        
    }
}
