use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use chrono::{DateTime, Utc};
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
    pub fn on_skill_start(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTSkillStartNotify {
            skill_id,
            skill_option_data,
            source_id
        } = PKTSkillStartNotify::new(&data)?;

            state.on_skill_start(
                source_id,
                skill_id,
                skill_option_data.tripod_index,
                skill_option_data.tripod_level,
            now);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};
    use lost_metrics_sniffer_stub::packets::definitions::TripodIndex as TTripodIndex;
    use lost_metrics_sniffer_stub::packets::definitions::TripodLevel as TTripodLevel;

    #[tokio::test]
    async fn should_register_skill_in_tracker() {
        let options = create_start_options();
        let packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let skill_id = SouleaterSkills::LethalSpinning as u32;
        let player_template = PLAYER_TEMPLATE_SOULEATER;
        let (opcode, data) = PacketBuilder::skill_start(
            player_template.id,
            skill_id,
            Some(TTripodIndex {
                first: 1,
                second: 1,
                third: 1
            }),
            Some(TTripodLevel {
                first: 1,
                second: 1,
                third: 1
            }));

        state_builder.create_player(&player_template);
        state_builder.set_fight_start();
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        let stats = state.entity_stats.get(&player_template.id).unwrap();
        let skill_stat = stats.skills.get(&skill_id).unwrap();
        
        assert_eq!(skill_stat.casts, 1);
        assert!(skill_stat.tripod_level.is_some());
        assert!(skill_stat.tripod_index.is_some());
        assert!(!state.cast_log.is_empty());
    }

    #[tokio::test]
    async fn should_update_entity() {
        let options = create_start_options();
        let packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let skill_id = SouleaterSkills::LethalSpinning as u32;
        let player_template = PLAYER_TEMPLATE_SOULEATER;
        let (opcode, data) = PacketBuilder::skill_start(
            player_template.id,
            skill_id,
            Some(TTripodIndex {
                first: 1,
                second: 1,
                third: 1
            }),
            Some(TTripodLevel {
                first: 1,
                second: 1,
                third: 1
            }));

        state_builder.set_fight_start();
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        let entity = state.entities_by_id.values().next().unwrap().borrow();
        assert_eq!(entity.id, player_template.id);
        assert_eq!(entity.class_id, Class::Souleater);
        assert_eq!(entity.entity_type, EntityType::Player);
    }
}
