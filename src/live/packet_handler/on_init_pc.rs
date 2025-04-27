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

        let PKTInitPC {
            character_id,
            class_id,
            player_id,
            name,
            gear_level,
            stat_pairs,
            status_effect_datas
        } = PKTInitPC::new(&data)?;

        let entity = state.on_init_pc(
            now,
            player_id,
            class_id,
            character_id,
            name.clone(),
            gear_level,
            stat_pairs,
            status_effect_datas
        );

        self.local_player_store.write().unwrap().write(name, character_id)?;

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
