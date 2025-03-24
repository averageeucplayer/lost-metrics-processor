use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::parse_pkt1;
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
use lost_metrics_data::VALID_ZONES;
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::definitions::*;
use lost_metrics_store::encounter_service::EncounterService;

use super::DefaultPacketHandler;

const RAID_DIFFICULTIES: &[(&str, u32)] = &[
    ("Normal", 0),
    ("Hard", 1),
    ("Inferno", 2),
    ("Challenge", 3),
    ("Solo", 4),
    ("The First", 5),
];

impl<FL, DH, SA, RS, LP, EE, ES> DefaultPacketHandler<FL, DH, SA, RS, LP, EE, ES>
where
    FL: Flags,
    DH: DamageEncryptionHandlerTrait,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    ES: EncounterService {
    pub fn on_zone_member_load(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = parse_pkt1(&data, PKTZoneMemberLoadStatusNotify::new)?;

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
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::PacketHandlerBuilder;

    #[tokio::test]
    async fn should_set_raid_difficulty() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let rt = Handle::current();

        let opcode = Pkt::ZoneMemberLoadStatusNotify;
        let data = PKTZoneMemberLoadStatusNotify {
            zone_id: 1,
            zone_level: 1
        };
        let data = data.encode().unwrap();
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
