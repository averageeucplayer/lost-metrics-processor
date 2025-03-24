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

        let now = Utc::now().timestamp_millis();
        let owner = self.trackers.borrow_mut().entity_tracker.get_source_entity(packet.source_id);
        let local_character_id = self.trackers.borrow().id_tracker
            .borrow()
            .get_local_character_id(self.trackers.borrow().entity_tracker.local_entity_id);
        let target_count = packet.skill_damage_events.len() as i32;

        for mut event in packet.skill_damage_events.into_iter() {
            if !self.damage_encryption_handler.decrypt_damage_event(&mut event) {
                state.damage_is_valid = false;
                continue;
            }
            let target_entity = self.trackers.borrow_mut().entity_tracker.get_or_create_entity(event.target_id);
            // source_entity is to determine battle item
            let source_entity = self.trackers.borrow_mut().entity_tracker.get_or_create_entity(packet.source_id);
            let (se_on_source, se_on_target) = self.trackers.borrow().status_tracker
                .borrow_mut()
                .get_status_effects(&owner, &target_entity, local_character_id);
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
                &owner,
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
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::PacketHandlerBuilder;

    #[tokio::test]
    async fn test() {
        
    }
}
