use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{on_shield_change, parse_pkt1};
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{StatusEffectTargetType, StatusEffectType};
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

        let mut trackers = self.trackers.borrow_mut();
        let packet = parse_pkt1(&data, PKTStatusEffectSyncDataNotify::new)?;

        let (status_effect, old_value) =
                trackers.status_tracker.borrow_mut().sync_status_effect(
                    packet.status_effect_instance_id,
                    packet.character_id,
                    packet.object_id,
                    packet.value,
                    trackers.entity_tracker.local_character_id,
                );
            if let Some(status_effect) = status_effect {
                if status_effect.status_effect_type == StatusEffectType::Shield {
                    let change = old_value
                        .checked_sub(status_effect.value)
                        .unwrap_or_default();
                    // on_shield_change(
                    //     &mut trackers.entity_tracker,
                    //     &trackers.id_tracker,
                    //     state,
                    //     status_effect,
                    //     change,
                    // );

                    if change == 0 {
                        return Ok(());
                    }
                
                    let source = trackers.entity_tracker.get_source_entity(status_effect.source_id);
                    let target_id = if status_effect.target_type == StatusEffectTargetType::Party {
                        trackers.id_tracker
                            .borrow()
                            .get_entity_id(status_effect.target_id)
                            .unwrap_or_default()
                    } else {
                        status_effect.target_id
                    };
                    let target = trackers.entity_tracker.get_source_entity(target_id);
                    state.on_boss_shield(&target, status_effect.value);
                    state.on_shield_used(&source, &target, status_effect.status_effect_id, change);
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
    async fn should_register_status_effect() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let rt = Handle::current();

        let opcode = Pkt::StatusEffectSyncDataNotify;
        let data = PKTStatusEffectSyncDataNotify {
            character_id: 1,
            object_id: 1,
            value: 0,
            status_effect_instance_id: 0
        };
        let data = data.encode().unwrap();

        packet_handler_builder.create_player(1, "Player_1".into());
        packet_handler_builder.create_player(2, "Player_2".into());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
