use crate::abstractions::*;
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
    pub fn on_new_transit(&mut self, data: &[u8]) -> anyhow::Result<()> {
      
        let packet = PKTNewTransit::new(&data)?;
        self.damage_encryption_handler.update_zone_instance_id(packet.channel_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};

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