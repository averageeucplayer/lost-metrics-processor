use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{build_status_effect, get_current_and_max_hp, truncate_gear_level};
use anyhow::Ok;
use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{EncounterEntity, Entity, EntityType, StatusEffectTargetType};
use lost_metrics_misc::get_class_from_id;
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
    pub fn on_new_pc(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTNewPC {
            pc_struct: PKTNewPCInner {
                player_id,
                name,
                class_id,
                max_item_level,
                character_id,
                stat_pairs,
                equip_item_datas,
                status_effect_datas
            }
        } = PKTNewPC::new(&data)?;

        state.new_pc(now, player_id, name, class_id, max_item_level, character_id, stat_pairs, equip_item_datas, status_effect_datas);

        // let (hp, max_hp) = get_current_and_max_hp(&stat_pairs);
        // let entity = {
        //     let entity = Entity {
        //         id: player_id,
        //         entity_type: EntityType::Player,
        //         name,
        //         class_id: class_id as u32,
        //         gear_level: truncate_gear_level(max_item_level), // todo?
        //         character_id: character_id,
        //         stats: stat_pairs.iter()
        //             .map(|sp| (sp.stat_type, sp.value))
        //             .collect(),
        //         ..Default::default()
        //     };
    
        //     state.entities.insert(entity.id, entity.clone());
        //     let old_entity_id = state.character_id_to_entity_id.get(&character_id).copied();

        //     if let Some(old_entity_id) = old_entity_id {
        //         if let Some(party_id) = state.entity_id_to_party_id.get(&old_entity_id).cloned() {
        //             state.entity_id_to_party_id.remove(&old_entity_id);
        //             state.entity_id_to_party_id.insert(player_id, party_id);
        //         }
        //     }

        //     state.character_id_to_entity_id.insert(character_id, player_id);
        //     state.entity_id_to_character_id.insert(player_id, character_id);
   
        //     state.complete_entry(character_id, player_id);
        //     // println!("party status: {:?}", self.party_tracker.borrow().character_id_to_party_id);
        //     let local_character_id = if state.local_character_id != 0 {
        //         state.local_character_id
        //     } else {
        //         state.entity_id_to_character_id
        //             .get(&state.local_entity_id)
        //             .copied()
        //             .unwrap_or_default()
        //     };

        //     let use_party_status_effects =
        //         state.should_use_party_status_effect(character_id, local_character_id);
        //     if use_party_status_effects {
        //         state.party_status_effect_registry.remove(&character_id);
        //     } else {
        //         state.local_status_effect_registry.remove(&character_id);
        //     }
        //     let (target_id, target_type) = if use_party_status_effects {
        //         (character_id, StatusEffectTargetType::Party)
        //     } else {
        //         (player_id, StatusEffectTargetType::Local)
        //     };

        //     for sed in status_effect_datas.into_iter() {
        //         let source_id = sed.source_id;
        //         let status_effect = build_status_effect(sed, target_id, source_id, target_type, now, None);
        //         state.register_status_effect(status_effect);
        //     }

        //     entity
        // };
        
        // info!(
        //     "new PC: {}, {}, {}, eid: {}, cid: {}",
        //     entity.name,
        //     get_class_from_id(&entity.class_id),
        //     entity.gear_level,
        //     entity.id,
        //     entity.character_id
        // );
        // info!("{entity}");

        // state.encounter
        //     .entities
        //     .entry(entity.name.clone())
        //     .and_modify(|player| {
        //         player.id = entity.id;
        //         player.gear_score = entity.gear_level;
        //         player.current_hp = hp;
        //         player.max_hp = max_hp;
        //         if entity.character_id > 0 {
        //             player.character_id = entity.character_id;
        //         }
        //     })
        //     .or_insert_with(|| {
        //         let mut player: EncounterEntity = entity.into();
        //         player.current_hp = hp;
        //         player.max_hp = max_hp;
        //         player
        //     });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, PLAYER_TEMPLATE_BERSERKER};

    #[tokio::test]
    async fn should_track_player_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let template = PLAYER_TEMPLATE_BERSERKER;
        let (opcode, data) = PacketBuilder::new_player(&template);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
