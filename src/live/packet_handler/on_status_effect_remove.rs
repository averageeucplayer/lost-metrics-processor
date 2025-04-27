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

                    let target_id = if status_effect.target_type == StatusEffectTargetType::Party {
                        state.character_id_to_entity_id
                            .get(&status_effect.target_id)
                            .copied()
                            .unwrap_or_default()
                    } else {
                        status_effect.target_id
                    };

                    let source = state.get_source_entity(status_effect.source_id).clone();
                    state.on_boss_shield(&target, status_effect.value);
                    state.on_shield_used(&source, &target, status_effect.status_effect_id, status_effect.value);
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
    use crate::live::packet_handler::test_utils::{to_status_effect_value, PacketBuilder, PacketHandlerBuilder, PartyTemplate, StateBuilder, NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER, PLAYER_TEMPLATE_BARD, PLAYER_TEMPLATE_BERSERKER, PLAYER_TEMPLATE_SORCERESS, PLAYER_TEMPLATE_SOULEATER, STATUS_EFFECT_TEMPLATE_BARD_WIND_OF_MUSIC_SHIELD, STATUS_EFFECT_TEMPLATE_FREEZE, STATUS_EFFECT_TEMPLATE_SHIELD};

    #[tokio::test]
    async fn should_reapply_boss_shield() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let npc_template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;
        let mut status_effect = STATUS_EFFECT_TEMPLATE_SHIELD;
        let shield_value = 10000;
        status_effect.source_id = npc_template.object_id;
        status_effect.value = to_status_effect_value(shield_value);

        state_builder.create_npc(&npc_template);
        state_builder.add_status_effect(npc_template.object_id, &status_effect);

        let (opcode, data) = PacketBuilder::status_effect_remove(
            npc_template.object_id,
            4,
            &status_effect);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        {
            let target = state.get_or_create_encounter_entity(npc_template.object_id).unwrap();
            assert_eq!(target.current_shield, shield_value);
        }
    }

    #[tokio::test]
    async fn should_reset_boss_shield() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let npc_template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;
        let mut status_effect = STATUS_EFFECT_TEMPLATE_SHIELD;
        status_effect.source_id = npc_template.object_id;
        status_effect.value = to_status_effect_value(10000);

        state_builder.create_npc(&npc_template);
        state_builder.add_status_effect(npc_template.object_id, &status_effect);

        let (opcode, data) = PacketBuilder::status_effect_remove(
            npc_template.object_id,
            3,
            &status_effect);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        {
            let target = state.get_or_create_encounter_entity(npc_template.object_id).unwrap();
            assert_eq!(target.current_shield, 0);
        }
    }

    #[tokio::test]
    async fn should_update_shield_stats() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let source_player_template = PLAYER_TEMPLATE_BARD;
        let target_player_template = PLAYER_TEMPLATE_SORCERESS;
        let mut status_effect = STATUS_EFFECT_TEMPLATE_BARD_WIND_OF_MUSIC_SHIELD;
        let absorb_value = 10000;
        status_effect.source_id = source_player_template.id;
        status_effect.value = to_status_effect_value(absorb_value);

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

        state_builder.create_player(&source_player_template);
        state_builder.create_player(&target_player_template);
        state_builder.create_party(&party_template);
        state_builder.add_status_effect(target_player_template.id, &status_effect);

        let (opcode, data) = PacketBuilder::status_effect_remove(
            target_player_template.id,
            4,
            &status_effect);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        {
            let source_entity = state.get_or_create_encounter_entity(source_player_template.id).unwrap();
            assert_eq!(source_entity.damage_stats.damage_absorbed_on_others, absorb_value);
        }

        {
            let target_entity = state.get_or_create_encounter_entity(target_player_template.id).unwrap();
            assert_eq!(target_entity.damage_stats.damage_absorbed, absorb_value);
        }

    }

    #[tokio::test]
    async fn should_remove_cc() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_SORCERESS;
        let mut status_effect = STATUS_EFFECT_TEMPLATE_FREEZE;
        status_effect.source_id = player_template.id;

        state_builder.create_player(&player_template);
        state_builder.add_status_effect(player_template.id, &status_effect);

        let (opcode, data) = PacketBuilder::status_effect_remove(
            player_template.id,
            4,
            &status_effect);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        {
            let target = state.get_or_create_encounter_entity(player_template.id).unwrap();
            let incapacitation = target.damage_stats.incapacitations.first().unwrap();
            assert_eq!(incapacitation.event_type, IncapacitationEventType::CrowdControl);
            assert_eq!(incapacitation.duration, 0);
        }
    }
}
