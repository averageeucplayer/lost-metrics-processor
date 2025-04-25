use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
use chrono::{DateTime, Utc};
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
    pub fn on_party_status_effect_add(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTPartyStatusEffectAddNotify {
            character_id,
            status_effect_datas,
        } = PKTPartyStatusEffectAddNotify::new(&data)?;

        let target_entity_id = state.character_id_to_entity_id.get(&character_id).copied().unwrap_or_default();
        let shields = state.party_status_effect_add(now, character_id, status_effect_datas);
        let current_boss_name = state.encounter.current_boss_name.clone();

        for status_effect in shields {
           
            let target_id =
                if status_effect.target_type == StatusEffectTargetType::Party {
                    target_entity_id
                } else {
                    status_effect.target_id
                };
            let target_name = state.get_source_entity(target_id).name.clone();
            
            if target_name == current_boss_name {
                state.encounter.entities
                    .entry(target_name)
                    .and_modify(|e| {
                        e.current_shield = status_effect.value;
                    });
            }

            let source = state.get_source_entity(status_effect.source_id).clone();
            let target = state.get_source_entity(target_id).clone();

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
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, STATUS_EFFECT_TEMPLATE_BARD_ATTACK_POWER_BUFF};

    #[tokio::test]
    async fn should_register_status_effect() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let (opcode, data) = PacketBuilder::party_status_effect_add(STATUS_EFFECT_TEMPLATE_BARD_ATTACK_POWER_BUFF);
        
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
