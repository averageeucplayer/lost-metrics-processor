use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{Entity, EntityType};
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
    pub fn on_init_env(&self, data: &[u8], state: &mut EncounterState, version: &str) -> anyhow::Result<()> {

        // three methods of getting local player info
        // 1. MigrationExecute    + InitEnv      + PartyInfo
        // 2. Cached Local Player + InitEnv      + PartyInfo
        //    > character_id        > entity_id    > player_info
        // 3. InitPC

        let packet = PKTInitEnv::new(&data)?;
        let player_id = packet.player_id;

        state.character_id_to_party_id.clear();
        state.entity_id_to_party_id.clear();
        state.raid_instance_to_party_ids.clear();

        state.raid_difficulty = "".to_string();
        state.raid_difficulty_id = 0;
        state.damage_is_valid = true;
        state.party_cache = None;
        state.party_map_cache = HashMap::new();
        
        let entity = {
            if !state.local_entity_id == 0 {
                let party_id = state.entity_id_to_party_id.get(&state.local_entity_id).cloned();

                if let Some(party_id) = party_id {
                    state.entity_id_to_party_id.remove(&state.local_entity_id);
                    state.entity_id_to_party_id.insert(player_id, party_id);
                }
            }
        
            info!("init env: eid: {}->{}", state.local_entity_id, player_id);
        
            let mut local_player = state
                .entities
                .get(&state.local_entity_id)
                .cloned()
                .unwrap_or_else(|| Entity {
                    entity_type: EntityType::Player,
                    name: "You".to_string(),
                    class_id: 0,
                    gear_level: 0.0,
                    character_id: state.local_character_id,
                    ..Default::default()
                });

            local_player.id = player_id;
            state.local_entity_id = player_id;
        
            state.entities.clear();
            state.entities.insert(local_player.id, local_player.clone());

            state.character_id_to_entity_id.clear();
            state.entity_id_to_character_id.clear();
            
            state.local_status_effect_registry.clear();
            state.party_status_effect_registry.clear();

            let character_id = local_player.character_id;

            if character_id > 0 {
                state.character_id_to_entity_id.insert(character_id, player_id);
                state.entity_id_to_character_id.insert(player_id, character_id);
                state.complete_entry(character_id, local_player.id);
            }

            local_player
        };

        state.on_init_env(
            version,
            state.client_id,
            entity, self.stats_api.clone(),
            self.encounter_service.clone(),
            self.event_emitter.clone());
        state.is_valid_zone = false;
        
        state.region = self.region_store.get();

        info!("region: {:?}", state.region);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER, PLAYER_TEMPLATE_BERSERKER};

    #[tokio::test]
    async fn should_emit_event() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();
        packet_handler_builder.ensure_event_called::<&str>("zone-change".into());
        packet_handler_builder.ensure_region_getter_called("EUC".into());

        let player_template = PLAYER_TEMPLATE_BERSERKER;
        let (opcode, data) = PacketBuilder::init_env(player_template.id);
        state_builder.create_player(&player_template);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

    }

    #[tokio::test]
    async fn should_save_to_db() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();
        packet_handler_builder.ensure_event_called::<&str>("zone-change".into());
        packet_handler_builder.ensure_region_getter_called("EUC".into());

        let player_template = PLAYER_TEMPLATE_BERSERKER;
        let (opcode, data) = PacketBuilder::init_env(player_template.id);
        state_builder.create_player(&player_template);
        state_builder.create_npc(&NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER);

        // let entity_name = "test".to_string();
        // let boss_name = "Thaemine the Lightqueller";
        // packet_handler_builder.create_player(1, entity_name.clone());
        // packet_handler_builder.create_npc(2, boss_name);

        state_builder.set_fight_start();

        state_builder.zero_boss_hp();
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();



        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

    }
}
