use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use log::*;
use lost_metrics_core::models::Identity;
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
    pub fn on_identity_change(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        if state.encounter.fight_start == 0 {
            return Ok(());
        }

        let PKTIdentityGaugeChangeNotify {
            player_id,
            identity_gauge1,
            identity_gauge2,
            identity_gauge3
        } = PKTIdentityGaugeChangeNotify::new(&data)?;

        let payload = Identity {
            gauge1: identity_gauge1,
            gauge2: identity_gauge2,
            gauge3: identity_gauge3,
        };

        state.on_identity_gain(
            now,
            player_id,
            &payload
        );

        if self.flags.can_emit_details() {
            self.event_emitter.emit("identity-update", payload)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_sniffer_stub::packets::opcodes::Pkt;
    use tokio::runtime::Handle;
    use crate::live::{packet_handler::*, test_utils::create_start_options};
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, PLAYER_TEMPLATE_BARD};
    
    #[tokio::test]
    async fn should_send_event() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_flag_can_emit_details_called(true);
        packet_handler_builder.ensure_event_called::<Identity>("identity-update".into());
        let mut state_builder = StateBuilder::new();

        let template = PLAYER_TEMPLATE_BARD;
        let (opcode, data) = PacketBuilder::identity_change(template.id);
        state_builder.create_player(&template);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }

    #[tokio::test]
    async fn should_update_entity_on_identity_change() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_flag_can_emit_details_called(false);
        let mut state_builder = StateBuilder::new();

        let template = PLAYER_TEMPLATE_BARD;
        let (opcode, data) = PacketBuilder::identity_change(template.id);
        state_builder.create_player(&template);
        state_builder.set_fight_start();
        
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    
        let identity_log = state.identity_log.get(template.name).unwrap();
        assert_eq!(identity_log.is_empty(), false);
    }
}
