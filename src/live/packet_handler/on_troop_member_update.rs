use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{get_status_effect_value, on_shield_change};
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
                let val = get_status_effect_value(&se.value);
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
                    
                        let target_entity_id = state.character_id_to_entity_id.get(&status_effect.target_id).copied().unwrap_or_default();
                        let source = state.get_source_entity(status_effect.source_id).clone();
                        let target_id = if status_effect.target_type == StatusEffectTargetType::Party {
                            target_entity_id
                        } else {
                            status_effect.target_id
                        };
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
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, PLAYER_TEMPLATE_BARD};

    #[tokio::test]
    async fn should_update_entity_hp() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_BARD;
        let (opcode, data) = PacketBuilder::troop_member_update(
            player_template.character_id,
            0,
            10000
        );
        
        let mut state = state_builder.build();
        // let entity_name = "test".to_string();
        // packet_handler_builder.create_player_with_character_id(1, 1, entity_name.clone());
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
