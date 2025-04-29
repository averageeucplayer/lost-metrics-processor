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
    pub fn on_new_projectile(&self, data: &[u8], state: &mut EncounterState) -> anyhow::Result<()> {

        let PKTNewProjectile {
            projectile_info: PKTNewProjectileInner {
                owner_id,
                projectile_id,
                skill_effect,
                skill_id
            }
        } = PKTNewProjectile::new(&data)?;

        state.on_new_projectile(projectile_id, owner_id, skill_id, skill_effect);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};

    #[tokio::test]
    async fn should_track_projectile() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let template = PROJECTILE_TEMPLATE_SORCERESS_EXPLOSION;
        let (opcode, data) = PacketBuilder::new_projectile(&template);

        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        assert_eq!(state.entities_by_id.len(), 1);

        let entity = state.entities_by_id.values().next().unwrap().borrow();
        assert_eq!(entity.id, template.projectile_id);
        assert_eq!(entity.skill_id, template.skill_id);
        assert_eq!(entity.entity_type, EntityType::Projectile);
    }

    async fn should_track_battle_item() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let template = PROJECTILE_TEMPLATE_DARK_GRENADE;
        let (opcode, data) = PacketBuilder::new_projectile(&template);

        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        assert_eq!(state.entities_by_id.len(), 1);

        let entity = state.entities_by_id.values().next().unwrap().borrow();
        assert_eq!(entity.id, template.projectile_id);
        assert_eq!(entity.skill_id, template.skill_id);
        assert_eq!(entity.entity_type, EntityType::Projectile);
        assert_eq!(entity.is_battle_item, true);
    }

    #[tokio::test]
    async fn should_update_timestamp_cache() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        let mut state_builder = StateBuilder::new();

        let mut player_template = PLAYER_TEMPLATE_SORCERESS;
        let mut projectile_template = PROJECTILE_TEMPLATE_SORCERESS_EXPLOSION;
        projectile_template.owner_id = player_template.id;
        let (opcode, data) = PacketBuilder::new_projectile(&projectile_template);

        state_builder.create_player(&player_template);
        let mut state = state_builder.build();

        let mut packet_handler = packet_handler_builder.build();
        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();
    }
}
