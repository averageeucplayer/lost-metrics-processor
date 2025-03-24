use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{on_shield_change, parse_pkt1};
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
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
    pub fn on_party_status_effect_remove(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let entity_tracker = &mut self.trackers.borrow_mut().entity_tracker;
        let packet = parse_pkt1(&data, PKTPartyStatusEffectRemoveNotify::new)?;

        let (is_shield, shields_broken, _effects_removed, _left_workshop) =
        entity_tracker.party_status_effect_remove(packet);
        if is_shield {
            for status_effect in shields_broken {
                let change = status_effect.value;
                on_shield_change(
                    entity_tracker,
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
    use std::vec;

    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use lost_metrics_sniffer_stub::packets::structures::StatusEffectData;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::PacketHandlerBuilder;

    #[tokio::test]
    async fn should_remove_status_effect() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        
        let rt = Handle::current();

        let opcode = Pkt::PartyStatusEffectRemoveNotify;
        let data = PKTStatusEffectRemoveNotify {
            object_id: 1,
            character_id: 1,
            reason: 0,
            status_effect_instance_ids: vec![1]
        };
        let data = data.encode().unwrap();

        let entity_name = "test".to_string();
        packet_handler_builder.create_player_with_character_id(1, 1, entity_name.clone());
        packet_handler_builder.add_party_status_effect(1, 1, 1);
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
