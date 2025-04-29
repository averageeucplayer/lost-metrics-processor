use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use lost_metrics_core::models::Class;
use lost_metrics_data::EntityExtensions;
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
    pub fn on_skill_cast(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTSkillCastNotify {
            skill_id,
            source_id
        } = PKTSkillCastNotify::new(&data)?;

        let mut entity = state.get_source_entity(source_id).clone();
        entity.borrow_mut().guess_is_player(skill_id);

        if entity.borrow().class_id == Class::Arcanist {
            state.on_skill_start(
                source_id,
                skill_id,
                None,
                None,
                now,
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};
    
    #[tokio::test]
    async fn should_update_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();
        
        let template = PLAYER_TEMPLATE_SORCERESS;
        let (opcode, data) = PacketBuilder::skill_cast(template.id, SorceressSkills::Doomsday as u32);
        state_builder.create_player(&template);

        // packet_handler_builder.create_unknown(1);
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
