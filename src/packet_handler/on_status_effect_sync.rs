use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use lost_metrics_core::models::{StatusEffectTargetType, StatusEffectType};
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::definitions::*;
use super::DefaultPacketHandler;

impl<FL, DH, SA, RS, LP, EE, PE> DefaultPacketHandler<FL, DH, SA, RS, LP, EE, PE>
where
    FL: Flags,
    DH: DamageEncryptionHandlerTrait,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    PE: Persister {
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
                
                    let target_entity_id = state.entities_by_character_id.get(&status_effect.target_id).map(|pr| pr.borrow().id).unwrap_or_default();
                    let target_id = if status_effect.target_type == StatusEffectTargetType::Party {
                        target_entity_id
                    } else {
                        status_effect.target_id
                    };
                    let target = state.get_source_entity(target_id);
                    let target = target.borrow();
                    let source = state.get_source_entity(status_effect.source_id);
                    let source = source.borrow();
                    state.on_boss_shield(target.id, status_effect.value);
                    state.on_shield_used(source.id, target.id, status_effect.status_effect_id, change);
                }
            }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};
    
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
