use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
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
    pub fn on_party_info(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTPartyInfo {
            party_instance_id,
            raid_instance_id,
            party_member_datas
        } = PKTPartyInfo::new(&data)?;

        let local_player_store = self.local_player_store.read().unwrap();
        let local_info = local_player_store.get();
        state.party_info(
            party_instance_id,
            raid_instance_id,
            party_member_datas,
            local_info);
        
        let local_player_id = state.local_entity_id;

        if let Some(entity) = state.entities.get(&local_player_id) {
            let entities = &mut state.encounter.entities;

            // we replace the existing local player if it exists, since its name might have changed (from hex or "You" to character name)
            if let Some(mut local) = entities.remove(&state.encounter.local_player) {
                // update local player name, insert back into encounter
                state.encounter.local_player.clone_from(&entity.name);
                
                local.update(&entity);
                local.class = entity.class_id.as_ref().to_string();
    
                entities.insert(state.encounter.local_player.clone(), local);
            } else {
                // cannot find old local player by name, so we look by local player's entity id
                // this can happen when the user started meter late
                let old_local = entities
                    .iter()
                    .find(|(_, e)| e.id == entity.id)
                    .map(|(key, _)| key.clone());
    
                // if we find the old local player, we update its name and insert back into encounter
                if let Some(old_local) = old_local {
                    let mut new_local = entities[&old_local].clone();
                    
                    new_local.update(&entity);
                    new_local.class = entity.class_id.as_ref().to_string();
    
                    entities.remove(&old_local);
                    state.encounter.local_player.clone_from(&entity.name);
                    entities.insert(state.encounter.local_player.clone(), new_local);
                }
            }
        }

        state.party_cache = None;
        state.party_map_cache = HashMap::new();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, PartyTemplate, StateBuilder, PLAYER_TEMPLATE_BARD, PLAYER_TEMPLATE_BERSERKER, PLAYER_TEMPLATE_SORCERESS, PLAYER_TEMPLATE_SOULEATER};

    #[tokio::test]
    async fn should_update_local_player() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let local_info = LocalInfo::default();
        packet_handler_builder.setup_local_store_get(local_info);

        let mut state_builder = StateBuilder::new();

        let mut party_template = PartyTemplate {
            party_instance_id: 1,
            raid_instance_id: 1,
            members: [
                PLAYER_TEMPLATE_BARD,
                PLAYER_TEMPLATE_BERSERKER,
                PLAYER_TEMPLATE_SORCERESS,
                PLAYER_TEMPLATE_SOULEATER
            ]
        };
        let (opcode, data) = PacketBuilder::party_info(&party_template);

        let local_player_template = PLAYER_TEMPLATE_SOULEATER;
        state_builder.create_player(&local_player_template);
        state_builder.set_local_player_id(local_player_template.id);
        state_builder.set_local_player_name(local_player_template.name.to_string());

        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }

    #[tokio::test]
    async fn should_update_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let local_info = LocalInfo::default();
        packet_handler_builder.setup_local_store_get(local_info);
        
        let mut state_builder = StateBuilder::new();

        let mut party_template = PartyTemplate {
            party_instance_id: 1,
            raid_instance_id: 1,
            members: [
                PLAYER_TEMPLATE_BARD,
                PLAYER_TEMPLATE_BERSERKER,
                PLAYER_TEMPLATE_SORCERESS,
                PLAYER_TEMPLATE_SOULEATER
            ]
        };
        let (opcode, data) = PacketBuilder::party_info(&party_template);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
