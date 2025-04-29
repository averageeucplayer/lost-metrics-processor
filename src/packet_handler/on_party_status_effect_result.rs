use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;

use anyhow::Ok;
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
    pub fn on_party_status_effect_result(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTPartyStatusEffectResultNotify {
            character_id,
            party_instance_id,
            raid_instance_id
        } = PKTPartyStatusEffectResultNotify::new(&data)?;
        
        state.on_party_status_effect_result(
            raid_instance_id,
            party_instance_id,
            character_id,
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};

    #[tokio::test]
    async fn should_map_entity_to_party() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let (opcode, data) = PacketBuilder::party_status_effect_result(1, 1, 1);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
