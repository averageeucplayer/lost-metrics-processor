use crate::constants::WORKSHOP_BUFF_ID;
use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::*;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{EntityType, StatusEffectDetails, StatusEffectTargetType, StatusEffectType};
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
    pub fn on_status_effect_remove(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTStatusEffectRemoveNotify {
            character_id,
            object_id: target_id,
            status_effect_instance_ids: instance_ids,
            reason
        } = PKTStatusEffectRemoveNotify::new(&data)?;

        let mut has_shield_buff = false;
        let mut shields_broken: Vec<StatusEffectDetails> = Vec::new();
        let mut left_workshop = false;
        let mut effects_removed = Vec::new();

        if let Some(ser) = state.local_status_effect_registry.get_mut(&target_id) {
            for id in instance_ids {
                if let Some(se) = ser.remove(&id) {
                    if se.status_effect_id == WORKSHOP_BUFF_ID {
                        left_workshop = true;
                    }
                    if se.status_effect_type == StatusEffectType::Shield {
                        has_shield_buff = true;
                        if reason == 4 {
                            shields_broken.push(se);
                            continue;
                        }
                    }
                    effects_removed.push(se);
                }
            }
        }

        let target = state.get_source_entity(target_id).clone();

        if has_shield_buff {
            if shields_broken.is_empty() {
                state.on_boss_shield(&target, 0);
            } else {
                for status_effect in shields_broken {
                    let change = status_effect.value;

                    let target_id = if status_effect.target_type == StatusEffectTargetType::Party {
                        state.character_id_to_entity_id
                            .get(&status_effect.target_id)
                            .copied()
                            .unwrap_or_default()
                    } else {
                        status_effect.target_id
                    };

                    if change == 0 {
                        continue;
                    }
                    
                    let source = state.get_source_entity(status_effect.source_id).clone();
                    state.on_boss_shield(&target, status_effect.value);
                    state.on_shield_used(&source, &target, status_effect.status_effect_id, change);
                }
            }
        }
        
        let now = now.timestamp_millis();
        for effect_removed in effects_removed {
            if effect_removed.status_effect_type == StatusEffectType::HardCrowdControl {
                let target = state.get_source_entity(effect_removed.target_id).clone();
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
    use crate::live::packet_handler::test_utils::{PacketHandlerBuilder, StateBuilder};

    #[tokio::test]
    async fn should_register_status_effect() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let opcode = Pkt::StatusEffectRemoveNotify;
        let data = PKTStatusEffectRemoveNotify {
            object_id: 1,
            character_id: 1,
            status_effect_instance_ids: vec![1],
            reason: 0
        };
        let data = data.encode().unwrap();

        let mut state = state_builder.build();

        // packet_handler_builder.create_player(1, "Player_1".into());
        // packet_handler_builder.create_player(2, "Player_2".into());
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
