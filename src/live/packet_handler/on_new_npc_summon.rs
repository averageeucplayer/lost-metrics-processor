use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{build_status_effect, get_current_and_max_hp};
use anyhow::Ok;
use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{Entity, EntityType, StatusEffectTargetType};
use lost_metrics_misc::get_npc_entity_type_name_grade;
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::definitions::*;
use lost_metrics_sniffer_stub::packets::structures::NpcStruct;
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
    pub fn on_new_npc_summon(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTNewNpcSummon {
            npc_struct: NpcStruct {
                balance_level,
                object_id,
                level,
                stat_pairs,
                status_effect_datas,
                type_id  
            },
            owner_id
        } = PKTNewNpcSummon::new(&data)?;
            
        let (hp, max_hp) = get_current_and_max_hp(&stat_pairs);
        let entity = {
            let (entity_type, name, grade) = get_npc_entity_type_name_grade(
                object_id,
                type_id,
                max_hp);
    
            let entity_type = if entity_type == EntityType::Npc {
                EntityType::Summon
            } else {
                entity_type
            };
            let npc = Entity {
                id: object_id,
                entity_type,
                name,
                grade,
                npc_id: type_id,
                owner_id: owner_id,
                level: level,
                balance_level: balance_level.unwrap_or(level),
                push_immune: entity_type == EntityType::Boss,
                stats: stat_pairs.iter()
                    .map(|sp| (sp.stat_type, sp.value))
                    .collect(),
                ..Default::default()
            };
            state.entities.insert(npc.id, npc.clone());
            state.local_status_effect_registry.remove(&object_id);
            // self.status_tracker.borrow_mut().remove_local_object(npc.id);
            // state.build_and_register_status_effects(packet.npc_struct.status_effect_datas, npc.id);

            for sed in status_effect_datas.into_iter() {
                // state.build_and_register_status_effect(&sed, object_id, timestamp, None);
                let source_id = state.get_source_entity(sed.source_id).id;

                let status_effect = build_status_effect(
                    sed.clone(),
                    object_id,
                    source_id,
                    StatusEffectTargetType::Local,
                    now,
                    None,
                );
        
                state.register_status_effect(status_effect.clone());
            }
            npc
        };

        info!(
            "new {}: {}, eid: {}, id: {}, hp: {}",
            entity.entity_type, entity.name, entity.id, entity.npc_id, max_hp
        );
        state.on_new_npc(entity, hp, max_hp);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use lost_metrics_sniffer_stub::packets::structures::NpcStruct;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER};

    #[tokio::test]
    async fn should_track_npc_summon() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;
        let (opcode, data) = PacketBuilder::new_npc_summon(&template);
        state_builder.create_npc(&template);
        
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
