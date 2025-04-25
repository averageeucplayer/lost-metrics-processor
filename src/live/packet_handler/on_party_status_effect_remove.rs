use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::on_shield_change;
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
    pub fn on_party_status_effect_remove(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTPartyStatusEffectRemoveNotify {
            character_id,
            reason,
            status_effect_instance_ids 
        } = PKTPartyStatusEffectRemoveNotify::new(&data)?;

        let (is_shield, shields_broken, _effects_removed, _left_workshop) =
        state.remove_status_effects(
            character_id,
            status_effect_instance_ids,
            reason,
            StatusEffectTargetType::Party,
        );

        if is_shield {
            for status_effect in shields_broken {
                let change = status_effect.value;
                on_shield_change(
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
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, STATUS_EFFECT_TEMPLATE_BARD_ATTACK_POWER_BUFF};

    #[tokio::test]
    async fn should_remove_status_effect() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        // let opcode = Pkt::PartyStatusEffectRemoveNotify;
        // let data = PKTStatusEffectRemoveNotify {
        //     object_id: 1,
        //     character_id: 1,
        //     reason: 0,
        //     status_effect_instance_ids: vec![1]
        // };
        // let data = data.encode().unwrap();

        let (opcode, data) = PacketBuilder::party_status_effect_remove(STATUS_EFFECT_TEMPLATE_BARD_ATTACK_POWER_BUFF);
        
        let mut state = state_builder.build();

        // let entity_name = "test".to_string();
        // packet_handler_builder.create_player_with_character_id(1, 1, entity_name.clone());
        // packet_handler_builder.add_party_status_effect(1, 1, 1);
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
