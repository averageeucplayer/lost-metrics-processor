use chrono::{DateTime, Utc};
use lost_metrics_core::models::*;
use lost_metrics_misc::get_npc_entity_type_name_grade;
use lost_metrics_sniffer_stub::packets::structures::{StatPair, StatusEffectData};
use crate::live::utils::*;

use super::EncounterState;

impl EncounterState {

    // add or update npc to encounter
    // we set current boss if npc matches criteria
    pub fn on_new_npc(
        &mut self,
        is_summon: bool,
        now: DateTime<Utc>,
        object_id: u64,
        npc_id: u32,
        owner_id: u64,
        level: u16,
        balance_level: Option<u16>,
        stat_pairs: Vec<StatPair>,
        status_effect_datas: Vec<StatusEffectData>
    ) {
        let (hp, max_hp) = get_current_and_max_hp(&stat_pairs);
        let (entity_type, name, grade) = get_npc_entity_type_name_grade(
            object_id,
            npc_id,
            max_hp);

        let entity_type = if is_summon && entity_type == EntityType::Npc {
            EntityType::Summon
        } else {
            entity_type
        };

        let npc = Entity {
            id: object_id,
            entity_type,
            name: name.clone(),
            grade,
            npc_id,
            owner_id: owner_id,
            level: level,
            balance_level: balance_level.unwrap_or(level),
            push_immune: entity_type == EntityType::Boss,
            stats: stat_pairs.iter()
                .map(|sp| (sp.stat_type, sp.value))
                .collect(),
            ..Default::default()
        };
        
        {
            self.entities.remove(&object_id);
            let entity = self.entities.entry(npc.id).or_insert_with(|| npc);

            let entity_name = name.clone();
            let new_boss_with_more_hp = self
                .encounter
                .entities
                .get(&self.encounter.current_boss_name)
                .map_or(true, |boss| max_hp >= boss.max_hp || boss.is_dead);
            
            let encounter_entity = self.encounter
                .entities
                .entry(entity_name.clone())
                .and_modify(|e| {
                    if entity_type != EntityType::Boss && e.entity_type != EntityType::Boss {
                        e.npc_id = npc_id;
                        e.id = object_id;
                        e.current_hp = hp;
                        e.max_hp = max_hp;
                    } else if entity_type == EntityType::Boss && e.entity_type == EntityType::Npc
                    {
                        e.entity_type = EntityType::Boss;
                        e.npc_id = npc_id;
                        e.id = object_id;
                        e.current_hp = hp;
                        e.max_hp = max_hp;
                    }
                })
                .or_insert_with(|| {
                    let entity = entity.clone();
                    let mut npc: EncounterEntity = entity.into();
                    npc.current_hp = hp;
                    npc.max_hp = max_hp;
                    npc
                });
    
            if entity_type == EntityType::Boss {
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

        self.local_status_effect_registry.remove(&object_id);
        for sed in status_effect_datas.into_iter() {
            let source_id = self.get_source_entity(sed.source_id).id;

            let status_effect = build_status_effect(
                sed.clone(),
                object_id,
                source_id,
                StatusEffectTargetType::Local,
                now,
            );
    
            self.register_status_effect(status_effect);
        }
    }
   
}