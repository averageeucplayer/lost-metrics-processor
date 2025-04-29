use crate::abstractions::*;
use crate::encounter_state::EncounterState;
use crate::flags::Flags;
use crate::StartOptions;
use anyhow::Ok;
use chrono::{DateTime, Utc};
use log::*;
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
    pub fn on_skill_damage(&self, now: DateTime<Utc>, data: &[u8], state: &mut EncounterState, options: &StartOptions) -> anyhow::Result<()> {

         // use this to make sure damage packets are not tracked after a raid just wiped
         if now - state.raid_end_cd < options.raid_end_capture_timeout {
            info!("ignoring damage - SkillDamageNotify");
            return Ok(());
        }

        let PKTSkillDamageNotify {
            skill_damage_events: mut events,
            skill_effect_id,
            skill_id,
            source_id
        } = PKTSkillDamageNotify::new(&data)?;

        let mut processed = vec![];

        for mut event in events {
            if self.damage_encryption_handler.decrypt_damage_event(&mut event) {
                processed.push(event);
            }
        }

        let result = state.on_damage_agg(
            now,
            source_id,
            processed,
            skill_id,
            skill_effect_id);

        if result.is_raid_start {
            self.event_emitter
                .emit(AppEvent::RaidStart(now.timestamp_millis()))
                .expect("failed to emit raid-start");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use lost_metrics_core::models::*;
    use crate::{packet_handler::PacketHandler, test_utils::*};
    
    #[tokio::test]
    async fn should_update_damage_stats_normal() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_decrypted();
        packet_handler_builder.ensure_event_called();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_SOULEATER;
        let npc_template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;

        let max_hp = 100000;
        let damage = 10000;
        let (opcode, data) = PacketBuilder::skill_damage(
            player_template.id,
            npc_template.object_id,
            SouleaterSkills::LethalSpinning as u32,
            damage,
            None,
            HitOption::FlankAttack,
            HitFlag::Normal,
            max_hp - damage,
            max_hp
        );

        state_builder.create_player(&player_template);
        state_builder.create_npc(&npc_template);
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        let source = state.get_or_create_encounter_entity(player_template.id).unwrap();
        assert_eq!(source.damage_stats.damage_dealt, damage);

        let target = state.get_or_create_encounter_entity(npc_template.object_id).unwrap();
        assert_eq!(target.damage_stats.damage_taken, damage);
    }

    #[tokio::test]
    async fn should_update_damage_stats_critical() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_decrypted();
        packet_handler_builder.ensure_event_called();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_SOULEATER;
        let npc_template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;

        let max_hp = 100000;
        let damage = 10000;
        let (opcode, data) = PacketBuilder::skill_damage(
            player_template.id,
            npc_template.object_id,
            SouleaterSkills::LethalSpinning as u32,
            damage,
            None,
            HitOption::FlankAttack,
            HitFlag::Critical,
            max_hp - damage,
            max_hp
        );

        state_builder.create_player(&player_template);
        state_builder.create_npc(&npc_template);
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        let source = state.get_or_create_encounter_entity(player_template.id).unwrap();
        assert_eq!(source.damage_stats.crit_damage, damage);

    }

    #[tokio::test]
    async fn should_update_damage_stats_back_attack() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_decrypted();
        packet_handler_builder.ensure_event_called();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_DEATHBLADE;
        let npc_template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;

        let max_hp = 100000;
        let damage = 10000;
        let (opcode, data) = PacketBuilder::skill_damage(
            player_template.id,
            npc_template.object_id,
            DeathbladeSkills::Zero as u32,
            damage,
            None,
            HitOption::BackAttack,
            HitFlag::Normal,
            max_hp - damage,
            max_hp
        );

        state_builder.create_player(&player_template);
        state_builder.create_npc(&npc_template);
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        let source = state.get_or_create_encounter_entity(player_template.id).unwrap();
        assert_eq!(source.damage_stats.back_attack_damage, damage);

    }

    async fn should_update_damage_stats_buffs() {
        let options = create_start_options();
        let mut packet_handler_builder = PacketHandlerBuilder::new();
        packet_handler_builder.ensure_event_decrypted();
        packet_handler_builder.ensure_event_called();
        let mut state_builder = StateBuilder::new();

        let player_template = PLAYER_TEMPLATE_DEATHBLADE;
        let npc_template = NPC_TEMPLATE_THAEMINE_THE_LIGHTQUELLER;

        let max_hp = 100000;
        let damage = 10000;
        let (opcode, data) = PacketBuilder::skill_damage(
            player_template.id,
            npc_template.object_id,
            DeathbladeSkills::Zero as u32,
            damage,
            None,
            HitOption::FlankAttack,
            HitFlag::Normal,
            max_hp - damage,
            max_hp
        );

        state_builder.create_player(&player_template);
        state_builder.create_npc(&npc_template);
        let mut state = state_builder.build();
        
        let mut packet_handler = packet_handler_builder.build();

        packet_handler.handle(opcode, &data, &mut state, &options).unwrap();

        let source = state.get_or_create_encounter_entity(player_template.id).unwrap();
        assert_eq!(source.damage_stats.back_attack_damage, damage);

    }
}
