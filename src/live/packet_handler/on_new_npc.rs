use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{get_current_and_max_hp, parse_pkt1};
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
    pub fn on_new_npc(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = parse_pkt1(&data, PKTNewNpc::new)?;
        let (hp, max_hp) = get_current_and_max_hp(&packet.npc_struct.stat_pairs);
        let entity = self.trackers.borrow_mut().entity_tracker.new_npc(packet, max_hp);
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
    use crate::live::packet_handler::test_utils::PacketHandlerBuilder;

    #[tokio::test]
    async fn should_track_npc_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let rt = Handle::current();

        let opcode = Pkt::NewNpc;
        let data = PKTNewNpc {
            npc_struct:  NpcStruct {
                type_id: 1,
                object_id: 1,
                level: 60,
                balance_level: None,
                stat_pairs: vec![],
                status_effect_datas: vec![],
            }
            
        };
        let data = data.encode().unwrap();
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
