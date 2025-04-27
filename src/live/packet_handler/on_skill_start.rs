use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use log::*;
use lost_metrics_core::models::EntityType;
use lost_metrics_data::EntityExtensions;
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
    pub fn on_skill_start(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTSkillStartNotify {
            skill_id,
            skill_option_data,
            source_id
        } = PKTSkillStartNotify::new(&data)?;

            state.on_skill_start(
                source_id,
                skill_id,
                skill_option_data.tripod_index,
                skill_option_data.tripod_level,
            now);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::definitions::{TripodIndex, TripodLevel};
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, PLAYER_TEMPLATE_SOULEATER};

    #[tokio::test]
    async fn should_register_skill_in_tracker() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_SOULEATER;
        let (opcode, data) = PacketBuilder::skill_start(1, 1);
    
        // packet_handler_builder.create_unknown(1);

        state_builder.create_player(&player_template);
        state_builder.set_fight_start();
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
