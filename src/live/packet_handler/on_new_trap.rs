use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{Entity, EntityType};
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
    pub fn on_new_trap(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTNewTrap {
            trap_struct: PKTNewTrapInner {
                object_id,
                owner_id,
                skill_effect,
                skill_id
            }
        } = PKTNewTrap::new(&data)?;

        state.on_new_trap(object_id, owner_id, skill_id, skill_effect);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, PLAYER_TEMPLATE_BARD, TRAP_TEMPLATE_BARD_STIGMA};

    #[tokio::test]
    async fn should_track_trap_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();
       
        let template = TRAP_TEMPLATE_BARD_STIGMA;
        let (opcode, data) = PacketBuilder::new_trap(&template);

        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }

    #[tokio::test]
    async fn should_update_timestamp_cache() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();
       
        let mut player_template = PLAYER_TEMPLATE_BARD;
        let mut trap_template = TRAP_TEMPLATE_BARD_STIGMA;
        trap_template.owner_id = player_template.id;
        let (opcode, data) = PacketBuilder::new_trap(&trap_template);
        state_builder.create_player(&player_template);

        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
