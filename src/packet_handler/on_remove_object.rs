use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;

use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::definitions::*;
use lost_metrics_store::encounter_service::EncounterService;

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
    pub fn on_remove_object(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = PKTRemoveObject::new(&data)?;

        for upo in packet.unpublished_objects {
            state.entities_by_id.remove(&upo.object_id);
            state.local_status_effect_registry.remove(&upo.object_id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};

    #[tokio::test]
    async fn should_remove_entities_from_tracker() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let (opcode, data) = PacketBuilder::remove_object(1);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
