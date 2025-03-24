use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{on_shield_change, parse_pkt1};
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::StatusEffectType;
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
    pub fn on_status_effect_sync(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = parse_pkt1(&data, PKTStatusEffectSyncDataNotify::new)?;

        let (status_effect, old_value) =
                self.trackers.borrow().status_tracker.borrow_mut().sync_status_effect(
                    packet.status_effect_instance_id,
                    packet.character_id,
                    packet.object_id,
                    packet.value,
                    self.trackers.borrow().entity_tracker.local_character_id,
                );
            if let Some(status_effect) = status_effect {
                if status_effect.status_effect_type == StatusEffectType::Shield {
                    let change = old_value
                        .checked_sub(status_effect.value)
                        .unwrap_or_default();
                    on_shield_change(
                        &mut self.trackers.borrow_mut().entity_tracker,
                        &self.trackers.borrow().id_tracker,
                        state,
                        status_effect,
                        change,
                    );
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
    async fn test() {
        
    }
}
