use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
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
    pub fn on_raid_boss_kill(&self, state: &mut EncounterState, version: &str) -> anyhow::Result<()> {

        state.on_phase_transition(
            version,
            state.client_id,
            1,
            self.stats_api.clone(),
            self.encounter_service.clone(),
            self.event_emitter.clone());
        state.raid_clear = true;
        info!("phase: 1 - RaidBossKillNotify");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER, PLAYER_TEMPLATE_BERSERKER};

    #[tokio::test]
    async fn should_emit_event_and_save_to_db() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_called::<i32>("phase-transition".into());
        let mut state_builder = StateBuilder::new();

        // let entity_name = "test".to_string();
        // let boss_name = "Thaemine the Lightqueller";
        // state_builder.create_player(1, entity_name.clone());
        // state_builder.create_npc(2, boss_name);

        let (opcode, data) = PacketBuilder::raid_boss_kill();

        let player_template = PLAYER_TEMPLATE_BERSERKER;
        state_builder.set_fight_start();
        state_builder.create_player(&player_template);
        state_builder.create_npc(&NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER);
        state_builder.zero_boss_hp();
        state_builder.set_damage_stats(player_template.id, 1000);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        // let boss_entity_stats = state.encounter.entities.get_mut(boss_name).unwrap();
        // boss_entity_stats.current_hp = 0;
        // let player_entity_stats = state.encounter.entities.get_mut(&entity_name).unwrap();
        // player_entity_stats.damage_stats.damage_dealt = 1000;
        // state.encounter.current_boss_name = boss_name.into();
        // state.encounter.fight_start = Utc::now().timestamp_millis();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

    }
}
