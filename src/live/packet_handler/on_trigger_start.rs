use std::time::Instant;

use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
use chrono::{DateTime, Utc};
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
    pub fn on_trigger_start(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState, version: &str) -> anyhow::Result<()> {

        let packet = PKTTriggerStartNotify::new(&data)?;
        match packet.signal {
            57 | 59 | 61 | 63 | 74 | 76 => {
                state.party_freeze = true;
                state.party_info = if let Some(party) = state.party_cache.take() {
                    party
                } else {
                    state.get_party()
                };
                state.raid_clear = true;
                state.on_phase_transition(
                    version,
                    state.client_id,
                    2,
                    self.stats_api.clone(),
                    self.encounter_service.clone(), self.event_emitter.clone());
                state.raid_end_cd = now;
                info!("phase: 2 - clear - TriggerStartNotify");
            }
            58 | 60 | 62 | 64 | 75 | 77 => {
                state.party_freeze = true;
                state.party_info = if let Some(party) = state.party_cache.take() {
                    party
                } else {
                    state.get_party()
                };
                state.raid_clear = false;
                state.on_phase_transition(
                    version,
                    state.client_id,
                    4,
                    self.stats_api.clone(),
                    self.encounter_service.clone(),
                    self.event_emitter.clone());
                state.raid_end_cd = now;
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
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder};

    #[tokio::test]
    async fn should_set_reset_flag() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_called::<i32>("phase-transition".into());
        let mut state_builder = StateBuilder::new();

        let (opcode, data) = PacketBuilder::trigger_start(57);
        
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
        assert!(state.resetting);
    }
}
