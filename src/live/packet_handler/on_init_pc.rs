use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{build_status_effect, get_current_and_max_hp, truncate_gear_level};
use anyhow::Ok;
use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{Entity, EntityType, StatusEffectTargetType};
use lost_metrics_misc::get_class_from_id;
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
    pub fn on_init_pc(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = PKTInitPC::new(&data)?;

        let (hp, max_hp) = get_current_and_max_hp(&packet.stat_pairs);
        let player_id = packet.player_id;

        let entity = {
            let player = Entity {
                id: player_id,
                is_local_player: true,
                entity_type: EntityType::Player,
                name: packet.name,
                class_id: packet.class_id as u32,
                gear_level: truncate_gear_level(packet.gear_level),
                character_id: packet.character_id,
                stats: packet
                    .stat_pairs
                    .iter()
                    .map(|sp| (sp.stat_type, sp.value))
                    .collect(),
                ..Default::default()
            };
    
            state.local_entity_id = player.id;
            state.local_character_id = player.character_id;
            state.entities.clear();
            state.entities.insert(player.id, player.clone());

            let character_id = player.character_id;
            state.character_id_to_entity_id.insert(character_id, player_id);
            state.entity_id_to_character_id.insert(player_id, character_id);


           state.local_player_name = Some(player.name.clone());
           state.complete_entry(character_id, player_id);
        // self.party_tracker
            //     .borrow_mut()
            //     .complete_entry(player.character_id, player_id);
            // self.status_tracker
            //     .borrow_mut()
            //     .remove_local_object(player.id);
            state.local_status_effect_registry.remove(&player_id);
            // state.build_and_register_status_effects(packet.status_effect_datas, player_id);

            for sed in packet.status_effect_datas.into_iter() {
                // state.build_and_register_status_effect(&sed, player_id, now, None);

                let status_effect = build_status_effect(
                    sed.clone(),
                    player_id,
                    sed.source_id,
                    StatusEffectTargetType::Local,
                    now,
                    None,
                );
        
                state.register_status_effect(status_effect.clone());
            }
            player
        };

        info!("{entity}");

        self.local_player_store.write().unwrap().write(entity.name.clone(), entity.character_id)?;

        state.on_init_pc(entity, hp, max_hp);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, PLAYER_TEMPLATE_BERSERKER};

    #[tokio::test]
    async fn should_track_local_player_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let state_builder = StateBuilder::new();
        packet_handler_builder.ensure_local_store_write_called();

        let template = PLAYER_TEMPLATE_BERSERKER;
        let (opcode, data) = PacketBuilder::local_player(&template);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
