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
    pub fn on_new_projectile(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTNewProjectile {
            projectile_info: PKTNewProjectileInner {
                owner_id,
                projectile_id,
                skill_effect,
                skill_id
            }
        } = PKTNewProjectile::new(&data)?;

        let projectile = Entity {
            id: projectile_id,
            entity_type: EntityType::Projectile,
            name: format!("{:x}", projectile_id),
            owner_id,
            skill_id,
            skill_effect_id: skill_effect,
            ..Default::default()
        };
        state.entities.insert(projectile.id, projectile);
        let is_player = state.id_is_player(owner_id);

        if is_player && skill_id > 0
        {
            let key = (owner_id, skill_id);
            if let Some(timestamp) = state.skill_timestamp.get(&key) {
                state.projectile_id_to_timestamp.insert(projectile_id, timestamp);
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
    use crate::live::packet_handler::test_utils::{PacketBuilder, PacketHandlerBuilder, StateBuilder, PLAYER_TEMPLATE_SORCERESS, PROJECTILE_TEMPLATE_SORCERESS_DESTRUCTION};

    #[tokio::test]
    async fn should_track_projectile_entity() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let template = PROJECTILE_TEMPLATE_SORCERESS_DESTRUCTION;
        let (opcode, data) = PacketBuilder::new_projectile(&template);

        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }

    #[tokio::test]
    async fn should_update_timestamp_cache() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let mut player_template = PLAYER_TEMPLATE_SORCERESS;
        let mut projectile_template = PROJECTILE_TEMPLATE_SORCERESS_DESTRUCTION;
        let (opcode, data) = PacketBuilder::new_projectile(&projectile_template);

        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
