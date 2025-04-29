use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::definitions::*;
use lost_metrics_sniffer_stub::packets::structures::NpcStruct;

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
    pub fn on_new_npc_summon(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTNewNpcSummon {
            npc_struct: NpcStruct {
                balance_level,
                object_id,
                level,
                stat_pairs,
                status_effect_datas,
                type_id  
            },
            owner_id
        } = PKTNewNpcSummon::new(&data)?;

        state.on_new_npc(
            true,
            now,
            object_id,
            type_id,
            owner_id,
            level,
            balance_level,
            stat_pairs,
            status_effect_datas
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};
    
    #[test]
    fn should_track_npc_summon() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let template = NPC_TEMPLATE_TABOO_KIN;
        let (opcode, data) = PacketBuilder::new_npc_summon(&template);
        state_builder.create_npc(&template);
        
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
        assert_eq!(state.entities_by_id.len(), 1);
        
        let entity = state.entities_by_id.values().next().unwrap().borrow();
        assert_eq!(entity.id, template.object_id);
        assert_eq!(entity.name, template.name);
        assert_eq!(entity.entity_type, EntityType::Summon);
    }
}
