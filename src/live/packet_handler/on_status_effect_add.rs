use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
use chrono::{DateTime, Utc};
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
    pub fn on_status_effect_add(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = PKTStatusEffectAddNotify::new(&data)?;

        let status_effect = state.build_and_register_status_effect(
            &packet.status_effect_data,
            packet.object_id,
            now
        );

        if status_effect.status_effect_type == StatusEffectType::Shield {
            let target_entity_id = state.character_id_to_entity_id.get(&status_effect.target_id).copied().unwrap_or_default();
            let source = state.get_source_entity(status_effect.source_id).clone();
            

            let target_id =
                if status_effect.target_type == StatusEffectTargetType::Party {
                    target_entity_id
                } else {
                    status_effect.target_id
                };
            let target = state.get_source_entity(target_id).clone();
            state.on_boss_shield(&target, status_effect.value);
            state.on_shield_applied(
                &source,
                &target,
                status_effect.status_effect_id,
                status_effect.value,
            );
        }

        if status_effect.status_effect_type == StatusEffectType::HardCrowdControl {
            let target = state.get_source_entity(status_effect.target_id).clone();
            if target.entity_type == EntityType::Player {
                state.on_cc_applied(&target, &status_effect);
            }
        }

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

        let buff = get_skill_buff_by_type_and_lt_duration("freeze", 3600);
      
        // packet_handler_builder.create_player(1, "Losing".into());
        // packet_handler_builder.create_player(2, "Baker".into());
        
        let (opcode, data) = PacketBuilder::skill_start(1, 1);

        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
        let incapacitation = state.encounter.entities.get("Baker").unwrap().damage_stats.incapacitations.first().unwrap();
        assert_eq!(incapacitation.event_type, IncapacitationEventType::CrowdControl);
        assert_eq!(incapacitation.duration, buff.duration as i64 * 1000);
    }

    #[tokio::test]
    async fn should_register_status_effect_case_shield() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let opcode = Pkt::StatusEffectAddNotify;
        let data = PKTStatusEffectAddNotify {
            object_id: 2,
            status_effect_data: StatusEffectData {
                source_id: 1,
                status_effect_id: 171805,
                status_effect_instance_id: 1,
                value: Some(vec![]),
                total_time: 10.0,
                stack_count: 0,
                end_tick: 0
            }
        };
        let data = data.encode().unwrap();

        let mut state = state_builder.build();

        // packet_handler_builder.create_player(1, "player_1".into());
        // packet_handler_builder.create_player(2, "player_1".into());
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }

    #[tokio::test]
    async fn should_register_status_effect() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let (opcode, data) = PacketBuilder::skill_start(1, 1);

        let mut state = state_builder.build();

        // packet_handler_builder.create_player(1, "player_1".into());
        // packet_handler_builder.create_player(2, "player_1".into());
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
