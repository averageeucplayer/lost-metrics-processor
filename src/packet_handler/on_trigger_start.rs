use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use log::*;
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
    pub fn on_trigger_start(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState, version: &str) -> anyhow::Result<()> {

        let PKTTriggerStartNotify {
            signal
        } = PKTTriggerStartNotify::new(&data)?;

        match signal {
            57 | 59 | 61 | 63 | 74 | 76 => {
                state.party_freeze = true;
                state.party_info = if let Some(party) = state.party_cache.take() {
                    party
                } else {
                    state.get_party()
                };
                state.raid_clear = true;
                // state.on_phase_transition(
                //     version,
                //     state.client_id,
                //     2,
                //     self.stats_api.clone(),
                //     self.encounter_service.clone(), self.event_emitter.clone());

                self.event_emitter
                    .emit(AppEvent::PhaseTransition(2))
                    .expect("failed to emit phase-transition");
        
                if !state.current_boss.is_some() {
                
                    // self.send_raid_info(self.stats_api.clone());
                    
                    // save_to_db(
                    //     version,
                    //     client_id,
                    //     stats_api,
                    //     false,
                    //     encounter_service,
                    //     event_emitter);
                    state.saved = true;
                }
        
                state.resetting = true;

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
                // state.on_phase_transition(
                //     version,
                //     state.client_id,
                //     4,
                //     self.stats_api.clone(),
                //     self.encounter_service.clone(),
                //     self.event_emitter.clone());

                self.event_emitter
                    .emit(AppEvent::PhaseTransition(4))
                    .expect("failed to emit phase-transition");
    
                if !state.current_boss.is_none() {
                
                    // self.send_raid_info(stats_api.clone());
        
                    // self.save_to_db(
                    //     version,
                    //     client_id,
                    //     stats_api,
                    //     false,
                    //     encounter_service,
                    //     event_emitter);
                    state.raid_clear = false;
                    state.saved = true;
                }
        
                state.resetting = true;

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
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};

    #[tokio::test]
    async fn should_set_reset_flag() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_called();
        let mut state_builder = StateBuilder::new();

        let (opcode, data) = PacketBuilder::trigger_start(57);
        
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
        assert!(state.resetting);
    }
}
