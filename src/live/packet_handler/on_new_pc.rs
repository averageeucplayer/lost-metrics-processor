use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{get_current_and_max_hp, parse_pkt1};
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
    pub fn on_new_pc(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = parse_pkt1(&data, PKTNewPC::new)?;
        let (hp, max_hp) = get_current_and_max_hp(&packet.pc_struct.stat_pairs);
        let entity = self.trackers.borrow_mut().entity_tracker.new_pc(packet);
        info!(
            "new PC: {}, {}, {}, eid: {}, cid: {}",
            entity.name,
            get_class_from_id(&entity.class_id),
            entity.gear_level,
            entity.id,
            entity.character_id
        );
        state.on_new_pc(entity, hp, max_hp);

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
    async fn should_track_player_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let rt = Handle::current();

        let opcode = Pkt::NewPC;
        let data = PKTNewPC {
            pc_struct:  PKTNewPCInner {
                player_id: 1,
                name: "test".into(),
                character_id: 1,
                class_id: 1,
                max_item_level: 1700.0,
                stat_pairs: vec![],
                status_effect_datas: vec![],
                equip_item_datas: vec![]
            }
            
        };
        let data = data.encode().unwrap();

        let entity_name = "test".to_string();
        packet_handler_builder.create_player(1, entity_name.clone());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
