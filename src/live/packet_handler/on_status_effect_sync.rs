use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
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

        let PKTStatusEffectSyncDataNotify {
            character_id,
            object_id,
            status_effect_instance_id,
            value
        } = PKTStatusEffectSyncDataNotify::new(&data)?;

        let (status_effect, old_value) =
                state.sync_status_effect(
                    status_effect_instance_id,
                    character_id,
                    object_id,
                    value,
                    state.local_character_id,
                );
            if let Some(status_effect) = status_effect {
                if status_effect.status_effect_type == StatusEffectType::Shield {
                    let change = old_value
                        .checked_sub(status_effect.value)
                        .unwrap_or_default();

                    if change == 0 {
                        return Ok(());
                    }
                
                    let target_entity_id = state.character_id_to_entity_id.get(&status_effect.target_id).copied().unwrap_or_default();
                    let target_id = if status_effect.target_type == StatusEffectTargetType::Party {
                        target_entity_id
                    } else {
                        status_effect.target_id
                    };
                    let source = state.get_source_entity(status_effect.source_id).clone();
                    let target = state.get_source_entity(target_id).clone();
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
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, PLAYER_TEMPLATE_BARD, PLAYER_TEMPLATE_SORCERESS};

    #[tokio::test]
    async fn should_register_status_effect() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let (opcode, data) = PacketBuilder::status_effect_sync(
            PLAYER_TEMPLATE_BARD.id,
            PLAYER_TEMPLATE_SORCERESS.id,
            1,
            1
        );
        state_builder.create_player(&PLAYER_TEMPLATE_BARD);
        state_builder.create_player(&PLAYER_TEMPLATE_SORCERESS);
        // packet_handler_builder.create_player(1, "Player_1".into());
        // packet_handler_builder.create_player(2, "Player_2".into());

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
