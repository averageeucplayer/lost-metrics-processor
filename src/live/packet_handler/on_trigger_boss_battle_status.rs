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
    pub fn on_trigger_boss_battle_status(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let encounter = &state.encounter;
        // need to hard code clown because it spawns before the trigger is sent???
            if encounter.current_boss_name.is_empty()
            || encounter.fight_start == 0
            || encounter.current_boss_name == "Saydon"
        {
            state.on_phase_transition(
                state.client_id, 3,
                self.stats_api.clone(),
                self.encounter_service.clone(),
                self.event_emitter.clone());
            info!("phase: 3 - resetting encounter - TriggerBossBattleStatus");
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
