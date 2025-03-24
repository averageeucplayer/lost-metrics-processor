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
            
            info!("raid zone id: {}", &packet.zone_id);
            info!("raid zone id: {}", &packet.zone_level);

            match packet.zone_level {
                0 => {
                    state.raid_difficulty = "Normal".to_string();
                    state.raid_difficulty_id = 0;
                }
                1 => {
                    state.raid_difficulty = "Hard".to_string();
                    state.raid_difficulty_id = 1;
                }
                2 => {
                    state.raid_difficulty = "Inferno".to_string();
                    state.raid_difficulty_id = 2;
                }
                3 => {
                    state.raid_difficulty = "Challenge".to_string();
                    state.raid_difficulty_id = 3;
                }
                4 => {
                    state.raid_difficulty = "Solo".to_string();
                    state.raid_difficulty_id = 4;
                }
                5 => {
                    state.raid_difficulty = "The First".to_string();
                    state.raid_difficulty_id = 5;
                }
                _ => {}
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
    async fn test() {
        
    }
}
