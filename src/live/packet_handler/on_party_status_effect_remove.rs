use crate::constants::WORKSHOP_BUFF_ID;
use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{StatusEffectDetails, StatusEffectTargetType, StatusEffectType};
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
    pub fn on_party_status_effect_remove(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTPartyStatusEffectRemoveNotify {
            character_id: target_id,
            reason,
            status_effect_instance_ids: instance_ids 
        } = PKTPartyStatusEffectRemoveNotify::new(&data)?;

        let mut has_shield_buff = false;
        let mut shields_broken: Vec<StatusEffectDetails> = Vec::new();
        let mut left_workshop = false;
        let mut effects_removed = Vec::new();

        if let Some(ser) = state.party_status_effect_registry.get_mut(&target_id) {
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

        if has_shield_buff {
            for status_effect in shields_broken {
                let change = status_effect.value;

                if change == 0 {
                    continue;
                }
            
                let target_entity_id = state.character_id_to_entity_id.get(&status_effect.target_id).cloned().unwrap_or_default();
                let target_id = if status_effect.target_type == StatusEffectTargetType::Party {
                    target_entity_id
                } else {
                    status_effect.target_id
                };
                
                let target = state.get_source_entity(target_id).clone();
                let source = state.get_source_entity(status_effect.source_id).clone();
                state.on_boss_shield(&target, status_effect.value);
                state.on_shield_used(&source, &target, status_effect.status_effect_id, change);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use lost_metrics_sniffer_stub::packets::structures::StatusEffectData;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER, PLAYER_TEMPLATE_BARD, STATUS_EFFECT_TEMPLATE_BARD_ATTACK_POWER_BUFF, STATUS_EFFECT_TEMPLATE_SHIELD};

    #[tokio::test]
    async fn should_remove_status_effect() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_BARD;
        let status_effect = STATUS_EFFECT_TEMPLATE_SHIELD;

        state_builder.create_player(&player_template);
        state_builder.add_party_status_effect(player_template.id, status_effect);
        let mut state = state_builder.build();

        let (opcode, data) = PacketBuilder::party_status_effect_remove(player_template.character_id, vec![], 4);
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
