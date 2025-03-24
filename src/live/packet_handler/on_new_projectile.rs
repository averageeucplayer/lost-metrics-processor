use crate::live::abstractions::{EventEmitter, LocalPlayerStore, RegionStore};
use crate::live::encounter_state::EncounterState;
use crate::live::flags::Flags;
use crate::live::stats_api::StatsApi;
use crate::live::utils::parse_pkt1;
use anyhow::Ok;
use hashbrown::HashMap;
use log::*;
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
    pub fn on_new_projectile(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let packet = parse_pkt1(&data, PKTNewProjectile::new)?;
        let owner_id = packet.projectile_info.owner_id;
        let skill_id = packet.projectile_info.skill_id;
        let projectile_id = packet.projectile_info.projectile_id;

        self.trackers.borrow_mut().entity_tracker.new_projectile(&packet);
        let is_player = self.trackers.borrow_mut().entity_tracker.id_is_player(owner_id);

        if is_player && skill_id > 0
        {
            let key = (owner_id, skill_id);
            if let Some(timestamp) = state.skill_tracker.skill_timestamp.get(&key) {
                state
                    .skill_tracker
                    .projectile_id_to_timestamp
                    .insert(packet.projectile_info.projectile_id, timestamp);
            }
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
    async fn should_track_projectile_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let rt = Handle::current();

        let opcode = Pkt::NewProjectile;
        let data = PKTNewProjectile {
            projectile_info: PKTNewProjectileInner { 
                projectile_id: 1,
                owner_id: 1,
                skill_id: 1,
                skill_effect: 0
            }
        };
        let data = data.encode().unwrap();
        
        let (mut state, mut packet_handler) = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options, rt).unwrap();
    }
}
