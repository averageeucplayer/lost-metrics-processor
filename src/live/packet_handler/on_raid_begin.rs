use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
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
    pub fn on_raid_begin(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = PKTRaidBegin::new(&data)?;

        info!("raid begin: {}", packet.raid_id);
        match packet.raid_id {
            308226 | 308227 | 308239 | 308339 => {
                state.raid_difficulty = "Trial".to_string();
                state.raid_difficulty_id = 7;
            }
            308428 | 308429 | 308420 | 308410 | 308411 | 308414 | 308422 | 308424
            | 308421 | 308412 | 308423 | 308426 | 308416 | 308419 | 308415 | 308437
            | 308417 | 308418 | 308425 | 308430 => {
                state.raid_difficulty = "Challenge".to_string();
                state.raid_difficulty_id = 8;
            }
            _ => {
                state.raid_difficulty = "".to_string();
                state.raid_difficulty_id = 0;
            }
        }

        state.is_valid_zone = VALID_ZONES.contains(&packet.raid_id);

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
    async fn should_update_raid_difficulty_to_trial() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();
        
        let (opcode, data) = PacketBuilder::raid_begin(308226);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        assert_eq!(state.raid_difficulty, "Trial");
        assert_eq!(state.raid_difficulty_id, 7);
    }

    #[tokio::test]
    async fn should_update_raid_difficulty_to_challenge() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();
        
        let (opcode, data) = PacketBuilder::raid_begin(308428);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        assert_eq!(state.raid_difficulty, "Challenge");
        assert_eq!(state.raid_difficulty_id, 8);
    }
}
