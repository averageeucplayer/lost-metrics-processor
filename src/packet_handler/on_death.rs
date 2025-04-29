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
    pub fn on_death(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTDeathNotify {
            target_id
        } = PKTDeathNotify::new(&data)?;

        let mut first_death = false;

        if let Some(entity) = state.get_or_create_encounter_entity(target_id) {

            if !entity.is_dead {
                first_death = true;
            }

            entity.current_hp = 0;
            entity.is_dead = true;
            entity.damage_stats.deaths += 1;
            entity.damage_stats.death_time = now.timestamp_millis();
            entity.cap_incapacitation_durations_to_death_time();

            info!("{entity}");
        }

        if state.current_boss.as_ref().filter(|pr| pr.borrow().id == target_id && first_death).is_some() {
            state.boss_dead_update = true;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{packet_handler::PacketHandler, test_utils::*};

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

    #[test]
    fn should_update_boss_dead_flag() {
        let options = create_start_options();
        let packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;
        let instance_id = template.object_id;
        let (opcode, data) = PacketBuilder::death(instance_id);
        state_builder.create_npc(&template);
        
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    
        assert_eq!(state.boss_dead_update, true);
    }
}
