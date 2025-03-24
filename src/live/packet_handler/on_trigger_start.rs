use std::time::Instant;

use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::parse_pkt1;
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
    pub fn on_trigger_start(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = parse_pkt1(&data, PKTTriggerStartNotify::new)?;
        match packet.signal {
            57 | 59 | 61 | 63 | 74 | 76 => {
                state.party_freeze = true;
                state.party_info = if let Some(party) = state.party_cache.take() {
                    party
                } else {
                    state.get_party_from_tracker()
                };
                state.raid_clear = true;
                state.on_phase_transition(
                    state.client_id,
                    2,
                    self.stats_api.clone(),
                    self.encounter_service.clone(), self.event_emitter.clone());
                state.raid_end_cd = Instant::now();
                info!("phase: 2 - clear - TriggerStartNotify");
            }
            58 | 60 | 62 | 64 | 75 | 77 => {
                state.party_freeze = true;
                state.party_info = if let Some(party) = state.party_cache.take() {
                    party
                } else {
                    state.get_party_from_tracker()
                };
                state.raid_clear = false;
                state.on_phase_transition(
                    state.client_id,
                    4,
                    self.stats_api.clone(),
                    self.encounter_service.clone(),
                    self.event_emitter.clone());
                state.raid_end_cd = Instant::now();
                info!("phase: 4 - wipe - TriggerStartNotify");
            }
            27 | 10 | 11 => {
                // debug_print(format_args!("old rdps sync time - {}", pkt.trigger_signal_type));
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
