use std::time::Instant;

use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::parse_pkt1;
use crate::live::StartOptions;
use anyhow::Ok;
use chrono::Utc;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::DamageData;
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
    pub fn on_skill_damage(&self, now: Instant, data: &[u8], state: &mut EncounterState, options: &StartOptions) -> anyhow::Result<()> {

         // use this to make sure damage packets are not tracked after a raid just wiped
         if now - state.raid_end_cd < options.raid_end_capture_timeout {
            info!("ignoring damage - SkillDamageNotify");
            return Ok(());
        }

        let packet = parse_pkt1(&data, PKTSkillDamageNotify::new)?;
        let mut trackers = self.trackers.borrow_mut();
        let now = Utc::now().timestamp_millis();
        // source_entity is to determine battle item
        let source_entity = trackers.entity_tracker.get_source_entity(packet.source_id);
        let local_character_id = trackers.id_tracker
            .borrow()
            .get_local_character_id(trackers.entity_tracker.local_entity_id);
        let target_count = packet.skill_damage_events.len() as i32;

        for mut event in packet.skill_damage_events.into_iter() {
            if !self.damage_encryption_handler.decrypt_damage_event(&mut event) {
                state.damage_is_valid = false;
                continue;
            }
            let target_entity = trackers.entity_tracker.get_or_create_entity(event.target_id);

            let (se_on_source, se_on_target) = trackers.status_tracker
                .borrow_mut()
                .get_status_effects(&source_entity, &target_entity, local_character_id);
            
            let damage_data = DamageData {
                skill_id: packet.skill_id,
                skill_effect_id: packet.skill_effect_id.unwrap_or_default(),
                damage: event.damage,
                modifier: event.modifier as i32,
                target_current_hp: event.cur_hp,
                target_max_hp: event.max_hp,
                damage_attribute: event.damage_attr,
                damage_type: event.damage_type,
            };

            state.on_damage(
                &source_entity,
                &source_entity,
                &target_entity,
                damage_data,
                se_on_source,
                se_on_target,
                target_count,
                now,
                self.event_emitter.clone()
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use lost_metrics_sniffer_stub::packets::structures::SkillDamageEvent;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{to_modifier, PacketHandlerBuilder, SorceressSkills};

    #[tokio::test]
    async fn should_update_damage_stats() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_decrypted();
        packet_handler_builder.ensure_event_called::<i64>("raid-start".into());
        let rt = Handle::current();
        let opcode = Pkt::SkillDamageNotify;
        let data = PKTSkillDamageNotify {
            source_id: 1,
            skill_damage_events: vec![
                SkillDamageEvent { 
                    target_id: 2,
                    damage: 3e9 as i64,
                    modifier: to_modifier(HitOption::FlankAttack, HitFlag::Critical),
                    cur_hp: 0 as i64,
                    max_hp: 3e9 as i64,
                    damage_attr: None,
                    damage_type: 0
                }
            ],
            skill_id: SorceressSkills::Doomsday as u32,
            skill_effect_id: None
        };
        let data = data.encode().unwrap();

        let entity_name = "Assun".to_string();
        let boss_name = "Thaemine the Lightqueller";
        packet_handler_builder.create_player(1, entity_name.clone());
        packet_handler_builder.create_npc_with_hp(2, boss_name, 1e10 as i64);
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();

        state.raid_end_cd = state.raid_end_cd - Duration::from_secs(11);

        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
        assert_eq!(state.encounter.entities.get(&entity_name).unwrap().damage_stats.crit_damage, 3e9 as i64);
        assert_eq!(state.encounter.entities.get(boss_name).unwrap().current_hp, 0);
    }
}
