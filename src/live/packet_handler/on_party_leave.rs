use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
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
    pub fn on_party_leave(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTPartyLeaveResult {
            name,
            party_instance_id
        }  = PKTPartyLeaveResult::new(&data)?;

        if state.local_player_name.as_ref().filter(|pr| *pr == &name).is_some() {
            state.character_id_to_party_id.retain(|_, &mut p_id| p_id != party_instance_id);
            state.entity_id_to_party_id.retain(|_, &mut p_id| p_id != party_instance_id);
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
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder};

    #[tokio::test]
    async fn should_update_references() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let name = "test".to_string();
        let (opcode, data) = PacketBuilder::party_leave(name.clone(), 1);

        state_builder.set_local_player_name(name);
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
