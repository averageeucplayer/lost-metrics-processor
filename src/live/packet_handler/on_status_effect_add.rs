use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::parse_pkt1;
use anyhow::Ok;
use chrono::Utc;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{EntityType, StatusEffectTargetType, StatusEffectType};
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
    pub fn on_status_effect_add(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let mut trackers=  self.trackers.borrow_mut();
        let packet = parse_pkt1(&data, PKTStatusEffectAddNotify::new)?;

        let status_effect = trackers.entity_tracker.build_and_register_status_effect(
            &packet.status_effect_data,
            packet.object_id,
            Utc::now(),
            Some(&state.encounter.entities),
        );

        if status_effect.status_effect_type == StatusEffectType::Shield {
            let source = trackers.entity_tracker.get_source_entity(status_effect.source_id);
            let target_id =
                if status_effect.target_type == StatusEffectTargetType::Party {
                    trackers.id_tracker.borrow().get_entity_id(status_effect.target_id)
                        .unwrap_or_default()
                } else {
                    status_effect.target_id
                };
            let target = trackers.entity_tracker.get_source_entity(target_id);
            state.on_boss_shield(&target, status_effect.value);
            state.on_shield_applied(
                &source,
                &target,
                status_effect.status_effect_id,
                status_effect.value,
            );
        }

        if status_effect.status_effect_type == StatusEffectType::HardCrowdControl {
            let target = trackers.entity_tracker.get_source_entity(status_effect.target_id);
            if target.entity_type == EntityType::Player {
                state.on_cc_applied(&target, &status_effect);
            }
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
    async fn should_update_status_effect_registry() {
        
    }
}
