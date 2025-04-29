use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;

use anyhow::Ok;
use chrono::{DateTime, Utc};
use log::*;
use lost_metrics_core::models::Identity;
use lost_metrics_sniffer_stub::decryption::DamageEncryptionHandlerTrait;
use lost_metrics_sniffer_stub::packets::definitions::*;
use lost_metrics_store::encounter_service::EncounterService;

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
    pub fn on_identity_change(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        if state.started_on == DateTime::<Utc>::MIN_UTC {
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
            self.event_emitter.emit(AppEvent::IdentityUpdate{
                gauge1: identity_gauge1,
                gauge2: identity_gauge2,
                gauge3: identity_gauge3
            })?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{packet_handler::PacketHandler, test_utils::*};
    
    #[test]
    fn should_send_event() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_flag_can_emit_details_called(true);
        packet_handler_builder.ensure_event_called();
        let mut state_builder = StateBuilder::new();

        let template = PLAYER_TEMPLATE_BARD;
        let (opcode, data) = PacketBuilder::identity_change(template.id);
        state_builder.create_player(&template);

        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }

    #[test]
    fn should_update_entity_on_identity_change() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_flag_can_emit_details_called(false);
        let mut state_builder = StateBuilder::new();

        let template = PLAYER_TEMPLATE_BARD;
        let (opcode, data) = PacketBuilder::identity_change(template.id);
        state_builder.local_player(&template);
        state_builder.set_fight_start();
        
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    
        let identity_log = state.identity_log.get(template.name).unwrap();
        assert_eq!(identity_log.is_empty(), false);
    }
}
