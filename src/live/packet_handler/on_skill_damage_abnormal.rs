use std::time::Instant;

use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::StartOptions;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{DamageEvent, EntityType};
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
    pub fn on_skill_damage_abnormal(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState, options: &StartOptions) -> anyhow::Result<()> {

        if now - state.raid_end_cd < options.raid_end_capture_timeout {
            info!("ignoring damage - SkillDamageAbnormalMoveNotify");
            return Ok(());
        }

        let PKTSkillDamageAbnormalMoveNotify {
            skill_damage_abnormal_move_events: mut events,
            skill_effect_id,
            skill_id,
            source_id
        } = PKTSkillDamageAbnormalMoveNotify::new(&data)?;

        let mut valid_events = vec![true; events.len()];
        let now_milliseconds = now.timestamp_millis();

        for (event, valid) in events.iter_mut().zip(valid_events.iter_mut()) {
            if !self.damage_encryption_handler.decrypt_damage_event(&mut event.skill_damage_event) {
                *valid = false;
            }

            let target_id = event.skill_damage_event.target_id;
            let target_entity = state.get_or_create_entity(target_id).clone();
            let option_data = &event.skill_move_option_data;

            if let Some(down_time) = option_data.down_time.filter(|_| target_entity.entity_type == EntityType::Player) {
                let entity = state.get_encounter_entity(&target_entity);
                entity.update_incapacitation(
                    down_time,
                    option_data.stand_up_time,
                    option_data.move_time,
                    now_milliseconds);
            }
        }

        let events: Vec<_> = events.into_iter().map(|pr| pr.skill_damage_event).collect();

        state.on_damage_agg(now, source_id, valid_events, events, skill_id, Some(skill_effect_id));


        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lost_metrics_sniffer_stub::packets::common::SkillMoveOptionData;
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use lost_metrics_sniffer_stub::packets::structures::SkillDamageEvent;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{to_modifier, PacketBuilder, PacketHandlerBuilder, StateBuilder, NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER, PLAYER_TEMPLATE_SOULEATER};

    #[tokio::test]
    async fn should_update_damage_stats() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_decrypted();
        packet_handler_builder.ensure_event_called::<i64>("raid-start".into());
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
