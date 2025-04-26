use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{build_status_effect, select_most_recent_valid_skill};
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
    pub fn on_status_effect_add(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTStatusEffectAddNotify {
            object_id: target_id,
            status_effect_data
        } = PKTStatusEffectAddNotify::new(&data)?;

        state.on_status_effect_add(now, target_id, status_effect_data);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use lost_metrics_sniffer_stub::packets::structures::StatusEffectData;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::*;

    #[tokio::test]
    async fn should_register_status_effect_case_crowd_control() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_BERSERKER;
        let mut status_effect = STATUS_EFFECT_TEMPLATE_FREEZE;
        status_effect.source_id = player_template.id;
        let expected_duration = status_effect.total_time as i64 * 1000;

        let (opcode, data) = PacketBuilder::status_effect_add(player_template.id, status_effect);

        state_builder.create_player(&player_template);
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
        
        let entity = state.get_or_create_encounter_entity(player_template.id).unwrap();
        let incapacitation = entity.damage_stats.incapacitations.first().unwrap();
        assert_eq!(incapacitation.event_type, IncapacitationEventType::CrowdControl);
        assert_eq!(incapacitation.duration, expected_duration);
    }

    #[tokio::test]
    async fn should_update_boss_shield() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let npc_template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;
        let mut status_effect = STATUS_EFFECT_TEMPLATE_SHIELD;
        let expected_shield_value = 1000;
        status_effect.source_id = npc_template.object_id;
        status_effect.value = to_status_effect_value(expected_shield_value);
        let (opcode, data) = PacketBuilder::status_effect_add(npc_template.object_id, status_effect);

        state_builder.create_npc(&npc_template);
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        let entity = state.get_or_create_encounter_entity(npc_template.object_id).unwrap();
        assert_eq!(entity.current_shield, expected_shield_value);
    }

    #[tokio::test]
    async fn should_record_shield_to_party_member() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let source_player_template = PLAYER_TEMPLATE_BARD;
        let target_player_template = PLAYER_TEMPLATE_SORCERESS;
        let mut status_effect = STATUS_EFFECT_TEMPLATE_SHIELD;
        let expected_shield_value = 1000;
        status_effect.source_id = source_player_template.id;
        status_effect.value = to_status_effect_value(expected_shield_value);
        let status_effect_id = status_effect.status_effect_id;
        let (opcode, data) = PacketBuilder::status_effect_add(target_player_template.id, status_effect);

        state_builder.create_player(&source_player_template);
        state_builder.create_player(&target_player_template);
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        {
            let source_entity = state.get_or_create_encounter_entity(source_player_template.id).unwrap();
            assert_eq!(source_entity.damage_stats.shields_given, expected_shield_value);
            assert_eq!(*source_entity.damage_stats.shields_given_by.get(&status_effect_id).unwrap(), expected_shield_value);
        }

        {
            let target_entity = state.get_or_create_encounter_entity(target_player_template.id).unwrap();
            assert_eq!(target_entity.damage_stats.shields_received, expected_shield_value);
            assert_eq!(*target_entity.damage_stats.shields_received_by.get(&status_effect_id).unwrap(), expected_shield_value);
        }
    }

    #[tokio::test]
    async fn should_record_self_shield() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_BARD;
        let mut status_effect = STATUS_EFFECT_TEMPLATE_SHIELD;
        status_effect.source_id = player_template.id;
        let (opcode, data) = PacketBuilder::status_effect_add(player_template.id, status_effect);

        state_builder.create_player(&player_template);
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
