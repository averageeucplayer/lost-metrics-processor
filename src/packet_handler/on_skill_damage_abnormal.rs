use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;

use crate::StartOptions;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use log::*;
use lost_metrics_core::models::{DamageEvent, EntityType};
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
    pub fn on_skill_damage_abnormal(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState, options: &StartOptions) -> anyhow::Result<()> {

        if now - state.raid_end_cd < options.raid_end_capture_timeout {
            info!("ignoring damage - SkillDamageAbnormalMoveNotify");
            return Ok(());
        }

        let PKTSkillDamageAbnormalMoveNotify {
            skill_damage_abnormal_move_events: events,
            skill_effect_id,
            skill_id,
            source_id
        } = PKTSkillDamageAbnormalMoveNotify::new(&data)?;

        let now_milliseconds = now.timestamp_millis();
        let mut processed = vec![];

        for mut event in events {
            let target_id = event.skill_damage_event.target_id;
            let target_entity = state.get_or_create_entity(target_id).clone();
            let option_data = &event.skill_move_option_data;

            if let Some(down_time) = option_data.down_time.filter(|_| target_entity.borrow().entity_type == EntityType::Player) {
                let entity = state.get_encounter_entity(target_id);
                entity.update_incapacitation(
                    down_time,
                    option_data.stand_up_time,
                    option_data.move_time,
                    now_milliseconds);
            }
            
            if !self.damage_encryption_handler.decrypt_damage_event(&mut event.skill_damage_event) {
                processed.push(event.skill_damage_event);
            }
        }

        state.on_damage_agg(now, source_id, processed, skill_id, Some(skill_effect_id));


        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};

    #[tokio::test]
    async fn should_update_damage_stats() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_decrypted();
        packet_handler_builder.ensure_event_called();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_SOULEATER;
        let npc_template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;

        let max_hp = 100000;
        let damage = 10000;
        let (opcode, data) = PacketBuilder::skill_damage_abnormal(
            player_template.id,
            npc_template.object_id,
            SouleaterSkills::LethalSpinning as u32,
            damage,
            None,
            HitOption::FlankAttack,
            HitFlag::Normal,
            max_hp - damage,
            max_hp,
            Some(1.0),
            Some(1.0),
            Some(1.0)
        );
        
        state_builder.create_player(&player_template);
        state_builder.create_npc(&npc_template);
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        let source = state.get_or_create_encounter_entity(player_template.id).unwrap();
        assert_eq!(source.damage_stats.damage_dealt, damage);

        let target = state.get_or_create_encounter_entity(npc_template.object_id).unwrap();
        assert_eq!(target.damage_stats.damage_taken, damage);
    }
}
