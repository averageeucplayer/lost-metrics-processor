use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::parse_pkt1;
use anyhow::Ok;
use log::*;
use lost_metrics_core::models::Identity;
use lost_metrics_sniffer_stub::packets::definitions::*;
use lost_metrics_store::encounter_service::EncounterService;

use super::DefaultPacketHandler;

impl<FL, SA, RS, LP, EE, ES> DefaultPacketHandler<FL, SA, RS, LP, EE, ES>
where
    FL: Flags,
    SA: StatsApi,
    RS: RegionStore,
    LP: LocalPlayerStore,
    EE: EventEmitter,
    ES: EncounterService {
    pub fn on_identity_change(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        if state.encounter.fight_start == 0 {
            return Ok(());
        }

        let packet = parse_pkt1(&data, PKTIdentityGaugeChangeNotify::new)?;

        state.on_identity_gain(&packet);
        if self.flags.can_emit_details() {
            let payload = Identity {
                gauge1: packet.identity_gauge1,
                gauge2: packet.identity_gauge2,
                gauge3: packet.identity_gauge3,
            };

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
    use crate::live::packet_handler::test_utils::PacketHandlerBuilder;
    
    #[tokio::test]
    async fn should_send_event() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_flag_can_emit_details_called(true);
        packet_handler_builder.ensure_event_called::<Identity>("identity-update".into());
        let rt = Handle::current();

        let opcode = Pkt::IdentityGaugeChangeNotify;
        let data = PKTIdentityGaugeChangeNotify {
            player_id: 1,
            identity_gauge1: 1,
            identity_gauge2: 1,
            identity_gauge3: 1
        };
        let data = data.encode().unwrap();

        let entity_name = "test".to_string();
        packet_handler_builder.create_player(1, entity_name.clone());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }

    #[tokio::test]
    async fn should_update_entity_on_identity_change() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_flag_can_emit_details_called(false);
        let rt = Handle::current();

        let opcode = Pkt::IdentityGaugeChangeNotify;
        let data = PKTIdentityGaugeChangeNotify {
            player_id: 1,
            identity_gauge1: 1,
            identity_gauge2: 1,
            identity_gauge3: 1
        };
        let data = data.encode().unwrap();

        let entity_name = "test".to_string();
        packet_handler_builder.create_player(1, entity_name.clone());
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        state.encounter.fight_start = Utc::now().timestamp_millis();
        state.encounter.local_player = entity_name.clone();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    
        let identity_log = state.identity_log.get(&entity_name).unwrap();
        assert_eq!(identity_log.is_empty(), false);
    }
}
