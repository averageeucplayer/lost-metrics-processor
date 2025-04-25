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
    pub fn on_new_npc(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = PKTNewNpc::new(data)?;
        let (hp, max_hp) = get_current_and_max_hp(&packet.npc_struct.stat_pairs);
        let entity = {
            let type_id = packet.npc_struct.type_id;
            let object_id = packet.npc_struct.object_id;
            let status_effect_datas = packet.npc_struct.status_effect_datas;
    
            let (entity_type, name, grade) = get_npc_entity_type_name_grade(
                object_id,
                type_id,
                max_hp);
    
            let npc = Entity {
                id: object_id,
                entity_type,
                name,
                grade,
                npc_id: type_id,
                level: packet.npc_struct.level,
                balance_level: packet.npc_struct.balance_level.unwrap_or(packet.npc_struct.level),
                push_immune: entity_type == EntityType::Boss,
                stats: packet
                    .npc_struct
                    .stat_pairs
                    .iter()
                    .map(|sp| (sp.stat_type, sp.value))
                    .collect(),
                ..Default::default()
            };
            state.entities.insert(object_id, npc.clone());
            // state.status_tracker.borrow_mut().remove_local_object(object_id);
            state.local_status_effect_registry.remove(&object_id);
            // state.build_and_register_status_effects(status_effect_datas, object_id);
            // let timestamp = Utc::now();
            for sed in status_effect_datas.into_iter() {
                // state.build_and_register_status_effect(&sed, object_id, now, None);
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
        info!("{entity}");
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
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER, PLAYER_TEMPLATE_BERSERKER};

    #[tokio::test]
    async fn should_track_npc_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();
        
        let template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;
        let (opcode, data) = PacketBuilder::new_npc(&template);
        state_builder.create_npc(&template);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
