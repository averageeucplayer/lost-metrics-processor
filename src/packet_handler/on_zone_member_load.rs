use crate::constants::RAID_DIFFICULTIES;
use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use log::*;
use lost_metrics_data::VALID_ZONES;
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
    pub fn on_zone_member_load(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = PKTZoneMemberLoadStatusNotify::new(&data)?;

        state.is_valid_zone = VALID_ZONES.contains(&packet.zone_id);

        if state.raid_difficulty_id >= packet.zone_id && !state.raid_difficulty.is_empty()
        {
            return Ok(());
        }
        
        info!("raid zone id: {} level: {}", &packet.zone_id, packet.zone_level);

        if let Some(&(name, id)) = RAID_DIFFICULTIES.get(packet.zone_level as usize) {
            state.raid_difficulty = name.to_string();
            state.raid_difficulty_id = id;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};

    #[tokio::test]
    async fn should_set_raid_difficulty() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let (opcode, data) = PacketBuilder::zone_member_load(30801, 1);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
