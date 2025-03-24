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
    pub fn on_new_trap(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = parse_pkt1(&data, PKTNewTrap::new)?;
        let object_id = packet.trap_struct.object_id;
        let owner_id = packet.trap_struct.owner_id;
        let skill_id = packet.trap_struct.skill_id;

        self.trackers.borrow_mut().entity_tracker.new_trap(&packet);
        if self.trackers.borrow_mut().entity_tracker.id_is_player(owner_id)
            && packet.trap_struct.skill_id > 0
        {
            let key = (owner_id, packet.trap_struct.skill_id);
            if let Some(timestamp) = state.skill_tracker.skill_timestamp.get(&key) {
                state
                    .skill_tracker
                    .projectile_id_to_timestamp
                    .insert(object_id, timestamp);
            }
        }

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
    async fn should_track_trap_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let rt = Handle::current();

        let opcode = Pkt::NewTrap;
        let data = PKTNewTrap {
            trap_struct: PKTNewTrapInner {
                object_id: 1,
                owner_id: 1,
                skill_id: 1,
                skill_effect: 0
            }
        };
        let data = data.encode().unwrap();
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
