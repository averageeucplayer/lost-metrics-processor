use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::parse_pkt1;
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
    pub fn on_status_effect_add(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let mut trackers=  self.trackers.borrow_mut();
        let packet = parse_pkt1(&data, PKTStatusEffectAddNotify::new)?;

        let status_effect = trackers.entity_tracker.build_and_register_status_effect(
            &packet.status_effect_data,
            packet.object_id,
            Utc::now(),
            Some(&state.encounter.entities),
        );

        if status_effect.status_effect_type == StatusEffectType::Shield {
            let source = trackers.entity_tracker.get_source_entity(status_effect.source_id);
            let target_id =
                if status_effect.target_type == StatusEffectTargetType::Party {
                    trackers.id_tracker.borrow().get_entity_id(status_effect.target_id)
                        .unwrap_or_default()
                } else {
                    status_effect.target_id
                };
            let target = trackers.entity_tracker.get_source_entity(target_id);
            state.on_boss_shield(&target, status_effect.value);
            state.on_shield_applied(
                &source,
                &target,
                status_effect.status_effect_id,
                status_effect.value,
            );
        }

        if status_effect.status_effect_type == StatusEffectType::HardCrowdControl {
            let target = trackers.entity_tracker.get_source_entity(status_effect.target_id);
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
        let rt = Handle::current();

        let buff = get_skill_buff_by_type_and_lt_duration("freeze", 3600);
        let opcode = Pkt::StatusEffectAddNotify;
        let data = PKTStatusEffectAddNotify {
            object_id: 2,
            status_effect_data: StatusEffectData {
                source_id: 1,
                status_effect_id: buff.id as u32,
                status_effect_instance_id: 1,
                value: Some(vec![]),
                total_time: buff.duration as f32,
                stack_count: 0,
                end_tick: 0
            }
        };
        let data = data.encode().unwrap();
        packet_handler_builder.create_player(1, "Losing".into());
        packet_handler_builder.create_player(2, "Baker".into());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
        let incapacitation = state.encounter.entities.get("Baker").unwrap().damage_stats.incapacitations.first().unwrap();
        assert_eq!(incapacitation.event_type, IncapacitationEventType::CrowdControl);
        assert_eq!(incapacitation.duration, buff.duration as i64 * 1000);
    }

    #[tokio::test]
    async fn should_register_status_effect_case_shield() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let rt = Handle::current();

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

        packet_handler_builder.create_player(1, "player_1".into());
        packet_handler_builder.create_player(2, "player_1".into());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }

    #[tokio::test]
    async fn should_register_status_effect() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let rt = Handle::current();

        let opcode = Pkt::StatusEffectAddNotify;
        let data = PKTStatusEffectAddNotify {
            object_id: 2,
            status_effect_data: StatusEffectData {
                source_id: 1,
                status_effect_id: 920017,
                status_effect_instance_id: 1,
                value: Some(vec![]),
                total_time: 10.0,
                stack_count: 0,
                end_tick: 0
            }
        };
        let data = data.encode().unwrap();

        packet_handler_builder.create_player(1, "player_1".into());
        packet_handler_builder.create_player(2, "player_1".into());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
