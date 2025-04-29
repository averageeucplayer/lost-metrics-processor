use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;

use anyhow::Ok;
use chrono::{DateTime, Utc};
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
    pub fn on_new_pc(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTNewPC {
            pc_struct: PKTNewPCInner {
                player_id,
                name,
                class_id,
                max_item_level,
                character_id,
                stat_pairs,
                equip_item_datas,
                status_effect_datas
            }
        } = PKTNewPC::new(&data)?;

        state.new_pc(
            now,
            player_id,
            name,
            class_id,
            max_item_level,
            character_id,
            stat_pairs,
            equip_item_datas,
            status_effect_datas);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};

    #[tokio::test]
    async fn should_track_player_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let template = PLAYER_TEMPLATE_BERSERKER;
        let (opcode, data) = PacketBuilder::new_player(&template);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        assert_eq!(state.entities_by_id.len(), 1);
        assert_eq!(state.entities_by_character_id.len(), 1);

        let entity = state.entities_by_id.values().next().unwrap().borrow();
        assert_eq!(entity.id, template.id);
        assert_eq!(entity.name, template.name);
        assert_eq!(entity.character_id, template.character_id);
        assert_eq!(entity.class_id, Class::Berserker);
        assert_eq!(entity.entity_type, EntityType::Player);
    }
}
