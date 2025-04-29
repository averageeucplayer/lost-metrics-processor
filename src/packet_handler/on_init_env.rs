use std::cell::RefCell;
use std::rc::Rc;
use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{Class, EncounterEntity, Entity, EntityType};
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::definitions::*;

use super::DefaultPacketHandler;

impl<FL, DH, SA, RS, LP, EE, PE> DefaultPacketHandler<FL, DH, SA, RS, LP, EE, PE>
where
    FL: Flags,
    DH: DamageEncryptionHandlerTrait,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    PE: Persister {
    /// Handles the `InitEnv` packet to initialize the encounter state for the local player.
    ///
    /// This function performs the following:
    /// - Parses the player's ID from the incoming packet data.
    /// - Saves the current encounter to the database, if complete.
    /// - Updates or creates the local player entity with the parsed ID.
    /// - Resets internal encounter state (e.g. party cache, entity maps, status effects).
    /// - Emits a `ZoneChange` event and fetches the current region from the region store.
    /// - Prepares the state for a new encounter session.
    pub fn on_init_env(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState, version: &str) -> anyhow::Result<()> {

        // three methods of getting local player info
        // 1. MigrationExecute    + InitEnv      + PartyInfo
        // 2. Cached Local Player + InitEnv      + PartyInfo
        //    > character_id        > entity_id    > player_info
        // 3. InitPC

        let PKTInitEnv {
            player_id
        } = PKTInitEnv::new(&data)?;
    
        if let Some(encounter) = state.get_complete_encounter() {
            self.persister.save(version, encounter)?;
        }

        let entity = match state.entities_by_id.remove(&state.local_entity_id) {
            Some(entity) => {
                {
                    let mut entity = entity.borrow_mut();
                    entity.id = player_id;
                }
                
                entity
            },
            None => {
                let entity = Entity {
                    id: player_id,
                    created_on: now,
                    entity_type: EntityType::Player,
                    class_id: Class::Unknown,
                    gear_level: 0.0,
                    character_id: state.local_character_id,
                    ..Default::default()
                };
                let entity = Rc::new(RefCell::new(entity));
                entity
            },
        };

        state.local_entity_id = player_id;
        
        state.raid_difficulty = "".to_string();
        state.raid_difficulty_id = 0;
        state.damage_is_valid = true;
        state.party_cache = None;
        state.party_map_cache = HashMap::new();
        state.local_status_effect_registry.clear();
        state.party_status_effect_registry.clear();
        state.parties_by_id.clear();
        state.entities_by_id.clear();
        state.entities_by_character_id.clear();
        state.entities_by_id.insert(player_id, entity.clone());

        if state.local_character_id > 0 {
            state.entities_by_character_id.insert(player_id, entity.clone());
        }

        if let Some(mut local_player) = state.entity_stats.remove(&state.local_entity_id)
        {
            state.entity_stats.insert(player_id, local_player);
        } else {
            let encounter_entity = EncounterEntity::default();
            state.entity_stats.insert(player_id, encounter_entity);
        }

        // remove unrelated entities
        state.entity_stats.retain(|_, e| {
            state.local_player_name.as_ref().is_some_and(|pr| pr == &e.name) || e.damage_stats.damage_dealt > 0
        });

        self.event_emitter
            .emit(AppEvent::ZoneChange)
            .expect("failed to emit zone-change");

        state.soft_reset(false);

        state.is_valid_zone = false;
        
        state.region = self.region_store.get();

        info!("region: {:?}", state.region);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{packet_handler::PacketHandler, test_utils::*};

    #[tokio::test]
    async fn should_set_region() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();
        packet_handler_builder.ensure_event_called();
        packet_handler_builder.ensure_region_getter_called("EUC".into());

        let player_template = PLAYER_TEMPLATE_BERSERKER;
        let (opcode, data) = PacketBuilder::init_env(player_template.id);
        state_builder.create_player(&player_template);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
        assert_eq!(state.region.unwrap(), "EUC");
    }

    #[tokio::test]
    async fn should_save_to_db_and_reset_state() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();
        packet_handler_builder.ensure_event_called();
        packet_handler_builder.ensure_region_getter_called("EUC".into());
        packet_handler_builder.ensure_save_to_db_called();

        let player_template = PLAYER_TEMPLATE_BERSERKER;
        let (opcode, data) = PacketBuilder::init_env(player_template.id);
        state_builder.create_player(&player_template);
        state_builder.create_npc(&NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER);
        state_builder.set_fight_start();
        state_builder.zero_boss_hp();
        state_builder.set_damage_stats(player_template.id, 1000);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
        assert_eq!(state.entities_by_id.len(), 1);
        assert_eq!(state.entities_by_character_id.len(), 1);
    }
}
