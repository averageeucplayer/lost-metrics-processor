use std::time::Instant;

use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use log::*;
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::definitions::*;
use lost_metrics_store::encounter_service::EncounterService;

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
    pub fn on_raid_result(&self, now: DateTime<Utc>, state: &mut EncounterState, version: &str) -> anyhow::Result<()> {

        state.party_freeze = true;
        state.party_info = if let Some(party) = state.party_cache.take() {
            party
        } else {
            state.get_party()
        };
        // state.on_phase_transition(
        //     version,
        //     state.client_id,
        //     0,
        //     self.stats_api.clone(),
        //     self.encounter_service.clone(),
        //     self.event_emitter.clone());

        // self.event_emitter
        //     .emit("phase-transition", phase_code)
        //     .expect("failed to emit phase-transition");

        // if !self.current_boss_name.is_empty() {
        
        //     self.send_raid_info(stats_api.clone());
            
        //     self.is_valid_zone = false;

        //     save_to_db(
        //         version,
        //         client_id,
        //         stats_api,
        //         false,
        //         encounter_service,
        //         event_emitter);
        //     self.saved = true;
        // }

        // self.resetting = true;

        // state.raid_end_cd = now;
        // info!("phase: 0 - RaidResult");

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

        // let entity_name = "test".to_string();
        // let boss_name = "Thaemine the Lightqueller";
        // packet_handler_builder.create_player(1, entity_name.clone());
        // packet_handler_builder.create_npc(2, boss_name);

        let (opcode, data) = PacketBuilder::raid_result();
        
        let mut packet_handler = packet_handler_builder.build();

        let mut state = state_builder.build();

        // let boss_entity_stats = state.encounter.entities.get_mut(boss_name).unwrap();
        // boss_entity_stats.current_hp = 0;
        // let player_entity_stats = state.encounter.entities.get_mut(&entity_name).unwrap();
        // player_entity_stats.damage_stats.damage_dealt = 1000;
        // state.encounter.current_boss_name = boss_name.into();
        // state.encounter.fight_start = Utc::now().timestamp_millis();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

    }
}
