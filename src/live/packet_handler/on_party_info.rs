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
    pub fn on_party_info(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let entity_tracker = &mut self.trackers.borrow_mut().entity_tracker;
        let packet = parse_pkt1(&data, PKTPartyInfo::new)?;

        let local_player_store = self.local_player_store.read().unwrap();
        let local_info = local_player_store.get();
        entity_tracker.party_info(packet, local_info);
        
        let local_player_id = entity_tracker.local_entity_id;

        if let Some(entity) = entity_tracker.entities.get(&local_player_id) {
            state.update_local_player(entity);
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
    use crate::live::packet_handler::test_utils::PacketHandlerBuilder;

    #[tokio::test]
    async fn should_update_local_player() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let local_info = LocalInfo::default();
        packet_handler_builder.setup_local_store_get(local_info);

        let rt = Handle::current();

        let opcode = Pkt::PartyInfo;
        let data = PKTPartyInfo {
            party_instance_id: 1,
            raid_instance_id: 1,
            party_member_datas: vec![
                PKTPartyInfoInner { 
                    name: "test".into(),
                    class_id: 101,
                    character_id: 1,
                    gear_level: 1700.0
                }
            ]
        };
        let data = data.encode().unwrap();

        let entity_name = "test".to_string();
        packet_handler_builder.create_player_with_character_id(1, 1, entity_name.clone());
        packet_handler_builder.set_local_player_id(1);

        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }

    #[tokio::test]
    async fn should_update_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let local_info = LocalInfo::default();
        packet_handler_builder.setup_local_store_get(local_info);
        
        let rt = Handle::current();

        let opcode = Pkt::PartyInfo;
        let data = PKTPartyInfo {
            party_instance_id: 1,
            raid_instance_id: 1,
            party_member_datas: vec![
                PKTPartyInfoInner { 
                    name: "test".into(),
                    class_id: 101,
                    character_id: 1,
                    gear_level: 1700.0
                }
            ]
        };
        let data = data.encode().unwrap();

        let entity_name = "test".to_string();
        packet_handler_builder.create_player_with_character_id(1, 1, entity_name.clone());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
