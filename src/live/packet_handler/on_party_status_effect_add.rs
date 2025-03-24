use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::parse_pkt1;
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::StatusEffectTargetType;
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
    pub fn on_party_status_effect_add(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let entity_tracker = &mut self.trackers.borrow_mut().entity_tracker;
        let packet = parse_pkt1(&data, PKTPartyStatusEffectAddNotify::new)?;

        // info!("{:?}", pkt);
        let shields =
        entity_tracker.party_status_effect_add(packet, &state.encounter.entities);

        for status_effect in shields {
            let source = entity_tracker.get_source_entity(status_effect.source_id);
            let target_id =
                if status_effect.target_type == StatusEffectTargetType::Party {
                    self.trackers.borrow().id_tracker
                        .borrow()
                        .get_entity_id(status_effect.target_id)
                        .unwrap_or_default()
                } else {
                    status_effect.target_id
                };
            let target = entity_tracker.get_source_entity(target_id);
            // info!("SHIELD SOURCE: {} > TARGET: {}", source.name, target.name);
            state.on_boss_shield(&target, status_effect.value);
            state.on_shield_applied(
                &source,
                &target,
                status_effect.status_effect_id,
                status_effect.value,
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use lost_metrics_sniffer_stub::packets::structures::StatusEffectData;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::PacketHandlerBuilder;

    #[tokio::test]
    async fn should_register_status_effect() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let rt = Handle::current();

        let opcode = Pkt::PartyStatusEffectAddNotify;
        let data = PKTPartyStatusEffectAddNotify {
            character_id: 1,
            status_effect_datas: vec![StatusEffectData {
                source_id: 1,
                status_effect_id: 211401,
                status_effect_instance_id: 1,
                value: Some(vec![]),
                total_time: 10.0,
                stack_count: 1,
                end_tick: 1
            }]
        };
        let data = data.encode().unwrap();

        let entity_name = "test".to_string();
        packet_handler_builder.create_player_with_character_id(1, 1, entity_name.clone());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
