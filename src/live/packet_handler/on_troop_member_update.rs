use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{get_status_effect_value};
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{StatusEffectTargetType, StatusEffectType};
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
    pub fn on_troop_member_update(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTTroopMemberUpdateMinNotify {
            character_id,
            cur_hp,
            max_hp,
            status_effect_datas
        } = PKTTroopMemberUpdateMinNotify::new(&data)?;

        let object_id = state.character_id_to_entity_id.get(&character_id).cloned();
        
        if let Some(object_id) = object_id {
            let entity = state.entities.get(&object_id);

            if let Some(entity) = entity {
                state
                    .encounter
                    .entities
                    .entry(entity.name.clone())
                    .and_modify(|e| {
                        e.current_hp = cur_hp;
                        e.max_hp = max_hp;
                    });
            }

            for se in status_effect_datas.iter() {
                let val = get_status_effect_value(&se.value.bytearray_0);
                let (status_effect, old_value) =
                        state.sync_status_effect(
                            se.status_effect_instance_id,
                            character_id,
                            object_id,
                            val,
                            state.local_character_id,
                        );

                if let Some(status_effect) = status_effect {
                    if status_effect.status_effect_type == StatusEffectType::Shield {
                        let change = old_value
                            .checked_sub(status_effect.value)
                            .unwrap_or_default();

                        if change == 0 {
                            return Ok(());
                        }
                    
                        let target_entity_id = state.character_id_to_entity_id.get(&status_effect.target_id).copied().unwrap_or_default();
                       
                        let target_id = if status_effect.target_type == StatusEffectTargetType::Party {
                            target_entity_id
                        } else {
                            status_effect.target_id
                        };

                        let source = state.get_source_entity(status_effect.source_id).clone();
                        let target = state.get_source_entity(target_id).clone();

                        state.on_boss_shield(&target, status_effect.value);
                        state.on_shield_used(&source, &target, status_effect.status_effect_id, change);
                    }
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
    use crate::live::packet_handler::test_utils::{to_status_effect_value, PacketBuilder, PacketHandlerBuilder, PartyTemplate, StateBuilder, NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER, PLAYER_TEMPLATE_BARD, PLAYER_TEMPLATE_BERSERKER, PLAYER_TEMPLATE_SORCERESS, PLAYER_TEMPLATE_SOULEATER, STATUS_EFFECT_TEMPLATE_BARD_ATTACK_POWER_BUFF, STATUS_EFFECT_TEMPLATE_BARD_WIND_OF_MUSIC_SHIELD, STATUS_EFFECT_TEMPLATE_SHIELD};

    #[tokio::test]
    async fn should_update_entity_hp() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_BARD;
        let (opcode, data) = PacketBuilder::troop_member_update(
            player_template.character_id,
            0,
            10000,
            &STATUS_EFFECT_TEMPLATE_BARD_ATTACK_POWER_BUFF
        );
        
        state_builder.create_player(&player_template);
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }

    #[tokio::test]
    async fn should_update_boss_shield() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let npc_template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;
        let mut status_effect = STATUS_EFFECT_TEMPLATE_SHIELD;
        status_effect.value = to_status_effect_value(1000);
        let (opcode, data) = PacketBuilder::troop_member_update(
            npc_template.object_id,
            0,
            10000,
            &status_effect
        );
        
        state_builder.create_npc(&npc_template);
        // state_builder.add_status_effect(status_effect);
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }

    #[tokio::test]
    async fn should_update_shield_stats() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let shield_value = 10000;
        let source_player_template = PLAYER_TEMPLATE_BARD;
        let target_player_template = PLAYER_TEMPLATE_SORCERESS;
        let mut status_effect = STATUS_EFFECT_TEMPLATE_BARD_WIND_OF_MUSIC_SHIELD;
        status_effect.source_id = source_player_template.id;
        status_effect.value = to_status_effect_value(shield_value);

        let mut party_template = PartyTemplate {
            party_instance_id: 1,
            raid_instance_id: 1,
            members: [
                PLAYER_TEMPLATE_BARD,
                PLAYER_TEMPLATE_BERSERKER,
                PLAYER_TEMPLATE_SORCERESS,
                PLAYER_TEMPLATE_SOULEATER
            ]
        };
        
        state_builder.local_player(&source_player_template);
        state_builder.create_player(&source_player_template);
        state_builder.create_player(&target_player_template);
        state_builder.create_party(&party_template);
        state_builder.add_party_status_effect(target_player_template.character_id, &status_effect);
        
        status_effect.value = to_status_effect_value(5000);
        let (opcode, data) = PacketBuilder::troop_member_update(
            target_player_template.character_id,
            10000,
            10000,
            &status_effect
        );

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
   
        {
            let source = state.get_or_create_encounter_entity(source_player_template.id).unwrap();            
            assert_eq!(source.damage_stats.shields_given, shield_value);
        }

        {
            let target = state.get_or_create_encounter_entity(target_player_template.id).unwrap();
            assert_eq!(target.damage_stats.shields_received, shield_value);
        }
        
    }
}
