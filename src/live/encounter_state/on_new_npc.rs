use lost_metrics_core::models::*;
use crate::live::utils::*;

use super::EncounterState;

impl EncounterState {

    // add or update npc to encounter
    // we set current boss if npc matches criteria
    pub fn on_new_npc(&mut self, entity: Entity, hp: i64, max_hp: i64) {
        let entity_name = entity.name.clone();
        let new_boss_with_more_hp = self
            .encounter
            .entities
            .get(&self.encounter.current_boss_name)
            .map_or(true, |boss| max_hp >= boss.max_hp || boss.is_dead);
        let entity = self.encounter
            .entities
            .entry(entity_name.clone())
            .and_modify(|e| {
                if entity.entity_type != EntityType::Boss && e.entity_type != EntityType::Boss {
                    e.npc_id = entity.npc_id;
                    e.id = entity.id;
                    e.current_hp = hp;
                    e.max_hp = max_hp;
                } else if entity.entity_type == EntityType::Boss && e.entity_type == EntityType::Npc
                {
                    e.entity_type = EntityType::Boss;
                    e.npc_id = entity.npc_id;
                    e.id = entity.id;
                    e.current_hp = hp;
                    e.max_hp = max_hp;
                }
            })
            .or_insert_with(|| {
                let mut npc: EncounterEntity = entity.into();
                npc.current_hp = hp;
                npc.max_hp = max_hp;
                npc
            });

        if entity.entity_type == EntityType::Boss {
            // if current encounter has no boss, we set the boss
            // if current encounter has a boss, we check if new boss has more max hp, or if current boss is dead

            self.encounter.current_boss_name = if new_boss_with_more_hp
            {
                entity_name
            } else {
                self.encounter.current_boss_name.clone()
            };
        }
    }
   
}