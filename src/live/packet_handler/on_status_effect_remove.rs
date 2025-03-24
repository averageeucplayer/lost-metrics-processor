use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::*;
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

        let trackers = &mut self.trackers.borrow_mut();
        let packet = parse_pkt1(&data, PKTStatusEffectRemoveNotify::new)?;

        let (is_shield, shields_broken, effects_removed, _left_workshop) =
        trackers.status_tracker.borrow_mut().remove_status_effects(
            packet.object_id,
            packet.status_effect_instance_ids,
            packet.reason,
            StatusEffectTargetType::Local,
        );
        
        if is_shield {
            if shields_broken.is_empty() {
                let target = trackers.entity_tracker.get_source_entity(packet.object_id);
                state.on_boss_shield(&target, 0);
            } else {
                for status_effect in shields_broken {
                    let change = status_effect.value;

                    let target_id = if status_effect.target_type == StatusEffectTargetType::Party {
                        trackers.id_tracker
                            .borrow()
                            .get_entity_id(status_effect.target_id)
                            .unwrap_or_default()
                    } else {
                        status_effect.target_id
                    };

                    if change == 0 {
                        continue;
                    }
                    
                    let entity_tracker = &mut trackers.entity_tracker;
                    let source = entity_tracker.get_source_entity(status_effect.source_id);
                    let target = entity_tracker.get_source_entity(target_id);
                    state.on_boss_shield(&target, status_effect.value);
                    state.on_shield_used(&source, &target, status_effect.status_effect_id, change);
                }
            }
        }
        
        let now = Utc::now().timestamp_millis();
        for effect_removed in effects_removed {
            if effect_removed.status_effect_type == StatusEffectType::HardCrowdControl {
                let target = trackers.entity_tracker.get_source_entity(effect_removed.target_id);
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
    async fn should_register_status_effect() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let rt = Handle::current();

        let opcode = Pkt::StatusEffectRemoveNotify;
        let data = PKTStatusEffectRemoveNotify {
            object_id: 1,
            character_id: 1,
            status_effect_instance_ids: vec![1],
            reason: 0
        };
        let data = data.encode().unwrap();

        packet_handler_builder.create_player(1, "Player_1".into());
        packet_handler_builder.create_player(2, "Player_2".into());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
