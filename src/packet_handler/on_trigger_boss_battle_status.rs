use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use chrono::{DateTime, Utc};
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
    pub fn on_trigger_boss_battle_status(&self, state: &mut EncounterState, version: &str) -> anyhow::Result<()> {

        if !(state.current_boss.is_none()
            || state.started_on == DateTime::<Utc>::MIN_UTC
            || state.current_boss.as_ref().filter(|pr| pr.borrow().name == "Saydon").is_some())
        {
            return Ok(());
        }

        self.event_emitter.emit(AppEvent::PhaseTransition(3))?;

        if let Some(payload) = state.get_raid_info() {
            self.stats_api.send_raid_info(payload);
        }

        if let Some(encounter) = state.get_complete_encounter() {
            self.persister.save(version, encounter)?;
            state.saved = true;
        }

        state.resetting = true;

        info!("phase: 3 - resetting encounter - TriggerBossBattleStatus");

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

        let (opcode, data) = PacketBuilder::trigger_boss_battle_status();

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
        assert!(state.resetting);
    }
}
