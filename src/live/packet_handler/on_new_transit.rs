use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
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
    pub fn on_new_transit(&mut self, data: &[u8]) -> anyhow::Result<()> {
      
        let packet = PKTNewTransit::new(&data)?;
        self.damage_encryption_handler.update_zone_instance_id(packet.channel_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::{definitions::PKTCounterAttackNotify, opcodes::Pkt};
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder};
    use crate::live::test_utils::MockDamageEncryptionHandlerTrait;

    #[tokio::test]
    async fn should_call_damage_handler() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new()
            .setup_damage_encryption_handler();
        let mut state_builder = StateBuilder::new();
        
        let (opcode, data) = PacketBuilder::new_transit(1);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}