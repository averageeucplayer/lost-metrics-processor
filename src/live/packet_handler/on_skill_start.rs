use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::parse_pkt1;
use anyhow::Ok;
use chrono::Utc;
use hashbrown::HashMap;
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
    pub fn on_skill_start(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = parse_pkt1(&data, PKTSkillStartNotify::new)?;
        let skill_id = packet.skill_id;

        let mut entity = self.trackers.borrow_mut().entity_tracker.get_source_entity(packet.source_id);
        self.trackers.borrow_mut().entity_tracker.guess_is_player(&mut entity, packet.skill_id);
        let skill_option_data = packet.skill_option_data;

        let tripod_index =
        skill_option_data.tripod_index
                .map(|tripod_index| lost_metrics_core::models::TripodIndex {
                    first: tripod_index.first,
                    second: tripod_index.second,
                    third: tripod_index.third,
                });
        let tripod_level =skill_option_data
                .tripod_level
                .map(|tripod_level| lost_metrics_core::models::TripodLevel {
                    first: tripod_level.first,
                    second: tripod_level.second,
                    third: tripod_level.third,
                });
        let timestamp = Utc::now().timestamp_millis();
        let (skill_id, summon_source) = state.on_skill_start(
            &entity,
            skill_id,
            tripod_index,
            tripod_level,
            timestamp,
        );

        if entity.entity_type == EntityType::Player && skill_id > 0 {
            state
                .skill_tracker
                .new_cast(entity.id, skill_id, summon_source, timestamp);
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
