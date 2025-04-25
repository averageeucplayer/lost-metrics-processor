use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use log::*;
use lost_metrics_core::models::EntityType;
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
    pub fn on_death(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = PKTDeathNotify::new(&data)?;

        let current_boss_name = state.encounter.current_boss_name.clone();
        let mut should_update_boss_dead = false;

        if let Some(entity) = state.get_or_create_encounter_entity(packet.target_id) {

            if entity.entity_type == EntityType::Boss
                && entity.name == current_boss_name
                && !entity.is_dead
            {
                should_update_boss_dead = true;
            }

            entity.current_hp = 0;
            entity.is_dead = true;
            entity.damage_stats.deaths += 1;
            entity.damage_stats.death_time = now.timestamp_millis();
            entity.cap_incapacitation_durations_to_death_time();

            info!("{entity}");
        }

        if should_update_boss_dead {
            state.boss_dead_update = true;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, PLAYER_TEMPLATE_BERSERKER};

    #[tokio::test]
    async fn should_update_entity_on_player_death() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let template = PLAYER_TEMPLATE_BERSERKER;
        let instance_id = template.id;
        let (opcode, data) = PacketBuilder::death(instance_id);
        state_builder.create_player(&template);
        
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    
        let actual = state.get_or_create_encounter_entity(instance_id).unwrap();
        assert_eq!(actual.is_dead, true);
        assert_eq!(actual.current_hp, 0);
    }
}
