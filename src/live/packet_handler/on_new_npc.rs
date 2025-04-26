use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::{build_status_effect, get_current_and_max_hp};
use anyhow::Ok;
use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::{Entity, EntityType, StatusEffectTargetType};
use lost_metrics_misc::get_npc_entity_type_name_grade;
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::definitions::*;
use lost_metrics_sniffer_stub::packets::structures::NpcStruct;
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
    pub fn on_new_npc(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTNewNpc {
            npc_struct: NpcStruct {
                balance_level,
                level,
                object_id,
                type_id,
                stat_pairs,
                status_effect_datas
            }
        } = PKTNewNpc::new(data)?;
        
        state.on_new_npc(
            false,
            now,
            object_id,
            type_id,
            object_id,
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
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use lost_metrics_sniffer_stub::packets::structures::NpcStruct;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER, PLAYER_TEMPLATE_BERSERKER};

    #[tokio::test]
    async fn should_track_npc_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();
        
        let template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;
        let (opcode, data) = PacketBuilder::new_npc(&template);
        state_builder.create_npc(&template);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
        assert_eq!(state.encounter.current_boss_name, template.name);
    }
}
