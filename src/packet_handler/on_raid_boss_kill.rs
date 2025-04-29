use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use log::*;
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;

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
    pub fn on_raid_boss_kill(&self, state: &mut EncounterState, version: &str) -> anyhow::Result<()> {

        let phase_code = 1;

        self.event_emitter
            .emit(AppEvent::PhaseTransition(phase_code))
            .expect("failed to emit phase-transition");

        state.raid_clear = true;
        info!("phase: 1 - RaidBossKillNotify");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};

    #[tokio::test]
    async fn should_emit_event_and_save_to_db() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_called();
        let mut state_builder = StateBuilder::new();

        let (opcode, data) = PacketBuilder::raid_boss_kill();
        
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

    }
}
