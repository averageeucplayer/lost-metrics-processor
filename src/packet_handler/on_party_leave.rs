use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use hashbrown::HashMap;
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
    pub fn on_party_leave(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTPartyLeaveResult {
            name,
            party_instance_id
        }  = PKTPartyLeaveResult::new(&data)?;

        if state.local_player_name.as_ref().filter(|pr| *pr == &name).is_some() {
            state.parties_by_id.remove(&party_instance_id);
        }

        state.party_cache = None;
        state.party_map_cache = HashMap::new();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};

    #[test]
    fn should_update_references() {
        let options = create_start_options();
        let packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let name = "test".to_string();
        let (opcode, data) = PacketBuilder::party_leave(name.clone(), 1);

        let party_template = PartyTemplate {
            party_instance_id: 1,
            raid_instance_id: 1,
            members: [
                PLAYER_TEMPLATE_BARD,
                PLAYER_TEMPLATE_BERSERKER,
                PLAYER_TEMPLATE_SORCERESS,
                PLAYER_TEMPLATE_SOULEATER
            ]
        };

        state_builder.create_party(&party_template);
        state_builder.set_local_player_name(name);
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        assert!(state.parties_by_id.is_empty());
    }
}
