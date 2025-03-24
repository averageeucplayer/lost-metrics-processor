use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::status_tracker::get_status_effect_value;
use crate::live::utils::{on_shield_change, parse_pkt1};
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

        let packet = parse_pkt1(&data, PKTTroopMemberUpdateMinNotify::new)?;
        let mut trackers = self.trackers.borrow_mut();
        let object_id = trackers.id_tracker.borrow().get_entity_id(packet.character_id);
        
        if let Some(object_id) = object_id {
            let entity = trackers.entity_tracker.get_entity_ref(object_id);

            if let Some(entity) = entity {
                state
                    .encounter
                    .entities
                    .entry(entity.name.clone())
                    .and_modify(|e| {
                        e.current_hp = packet.cur_hp;
                        e.max_hp = packet.max_hp;
                    });
            }
            for se in packet.status_effect_datas.iter() {
                let val = get_status_effect_value(&se.value);
                let (status_effect, old_value) =
                        trackers.status_tracker.borrow_mut().sync_status_effect(
                        se.status_effect_instance_id,
                        packet.character_id,
                        object_id,
                        val,
                        trackers.entity_tracker.local_character_id,
                    );
                if let Some(status_effect) = status_effect {
                    if status_effect.status_effect_type == StatusEffectType::Shield {
                        let change = old_value
                            .checked_sub(status_effect.value)
                            .unwrap_or_default();
                        // on_shield_change(
                        //     &mut trackers.entity_tracker,
                        //     &trackers.id_tracker,
                        //     state,
                        //     status_effect,
                        //     change,
                        // );

                        if change == 0 {
                            return Ok(());
                        }
                    
                        let source = trackers.entity_tracker.get_source_entity(status_effect.source_id);
                        let target_id = if status_effect.target_type == StatusEffectTargetType::Party {
                            trackers.id_tracker
                                .borrow()
                                .get_entity_id(status_effect.target_id)
                                .unwrap_or_default()
                        } else {
                            status_effect.target_id
                        };
                        let target = trackers.entity_tracker.get_source_entity(target_id);
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
    use crate::live::packet_handler::test_utils::PacketHandlerBuilder;

    #[tokio::test]
    async fn should_update_entity_hp() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let rt = Handle::current();

        let opcode = Pkt::TroopMemberUpdateMinNotify;
        let data = PKTTroopMemberUpdateMinNotify {
            character_id: 1,
            cur_hp: 3e6 as i64,
            max_hp: 3e6 as i64,
            status_effect_datas: vec![]
        };
        let data = data.encode().unwrap();

        let entity_name = "test".to_string();
        packet_handler_builder.create_player_with_character_id(1, 1, entity_name.clone());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
