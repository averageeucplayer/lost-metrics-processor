use chrono::Utc;
use lost_metrics_core::models::*;
use crate::live::utils::*;

use super::EncounterState;

impl EncounterState {

    pub fn on_death(&mut self, dead_entity: &Entity) {
        let entity = self
            .encounter
            .entities
            .entry(dead_entity.name.clone())
            .or_insert_with(|| encounter_entity_from_entity(dead_entity));

        if (dead_entity.entity_type != EntityType::Player
            && dead_entity.entity_type != EntityType::Boss)
            || entity.id != dead_entity.id
            || (entity.entity_type == EntityType::Boss && entity.npc_id != dead_entity.npc_id)
        {
            return;
        }

        if entity.entity_type == EntityType::Boss
            && dead_entity.entity_type == EntityType::Boss
            && entity.name == self.encounter.current_boss_name
            && !entity.is_dead
        {
            self.boss_dead_update = true;
        }

        entity.current_hp = 0;
        entity.is_dead = true;
        entity.damage_stats.deaths += 1;
        entity.damage_stats.death_time = Utc::now().timestamp_millis();
        entity
            .damage_stats
            .incapacitations
            .iter_mut()
            .rev()
            .take_while(|x| x.timestamp + x.duration > entity.damage_stats.death_time)
            .for_each(|x| {
                // cap duration to death time if it exceeds it
                x.duration = x.timestamp - entity.damage_stats.death_time;
            });
    }

}