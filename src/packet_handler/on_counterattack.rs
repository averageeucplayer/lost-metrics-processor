use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;

use anyhow::Ok;
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
    pub fn on_counterattack(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {
        let packet = PKTCounterAttackNotify::new(&data)?;
        let source_id = packet.source_id;
        
        if let Some(entity) = state.get_or_create_encounter_entity(source_id) {
            entity.skill_stats.counters += 1;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{packet_handler::PacketHandler, test_utils::*};

    #[test]
    fn should_update_stats_when_counter() {
        let options = create_start_options();
        let packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let template = PLAYER_TEMPLATE_BERSERKER;
        let instance_id = template.id;
        let (opcode, data) = PacketBuilder::counterattack(instance_id);
        state_builder.create_player(&template);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    
        assert_eq!(state.get_or_create_encounter_entity(instance_id).unwrap().skill_stats.counters, 1);
    }
}