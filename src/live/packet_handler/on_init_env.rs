use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::parse_pkt1;
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
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
    pub fn on_init_env(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        // three methods of getting local player info
        // 1. MigrationExecute    + InitEnv      + PartyInfo
        // 2. Cached Local Player + InitEnv      + PartyInfo
        //    > character_id        > entity_id    > player_info
        // 3. InitPC

        let packet = parse_pkt1(&data, PKTInitEnv::new)?;

        self.trackers.borrow().party_tracker.borrow_mut().reset_party_mappings();
        state.raid_difficulty = "".to_string();
        state.raid_difficulty_id = 0;
        state.damage_is_valid = true;
        state.party_cache = None;
        state.party_map_cache = HashMap::new();
        let entity = self.trackers.borrow_mut().entity_tracker.init_env(packet);
        state.on_init_env(
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
    use crate::live::packet_handler::test_utils::PacketHandlerBuilder;

    #[tokio::test]
    async fn should_emit_event() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_called::<&str>("zone-change".into());
        packet_handler_builder.ensure_region_getter_called("EUC".into());
        let rt = Handle::current();

        let opcode = Pkt::InitEnv;
        let data = PKTInitEnv {
            player_id: 1,
        };
        let data = data.encode().unwrap();

        let entity_name = "test".to_string();
        packet_handler_builder.create_player(1, entity_name.clone());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();

    }

    #[tokio::test]
    async fn should_save_to_db() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_called::<&str>("zone-change".into());
        packet_handler_builder.ensure_region_getter_called("EUC".into());
        let rt = Handle::current();

        let opcode = Pkt::InitEnv;
        let data = PKTInitEnv {
            player_id: 1,
        };
        let data = data.encode().unwrap();

        let entity_name = "test".to_string();
        let boss_name = "Thaemine the Lightqueller";
        packet_handler_builder.create_player(1, entity_name.clone());
        packet_handler_builder.create_npc(2, boss_name);
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();

        let boss_entity_stats = state.encounter.entities.get_mut(boss_name).unwrap();
        boss_entity_stats.current_hp = 0;
        let player_entity_stats = state.encounter.entities.get_mut(&entity_name).unwrap();
        player_entity_stats.damage_stats.damage_dealt = 1000;
        state.encounter.current_boss_name = boss_name.into();
        state.encounter.fight_start = Utc::now().timestamp_millis();

        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();

    }
}
