use lost_metrics_core::models::*;
use lost_metrics_misc::*;
use crate::live::utils::*;

use super::EncounterState;

impl EncounterState {

    pub fn on_shield_applied(
        &mut self,
        source_entity: &Entity,
        target_entity: &Entity,
        buff_id: u32,
        shield: u64,
    ) {
        if source_entity.entity_type != EntityType::Player
            || target_entity.entity_type != EntityType::Player
        {
            return;
        }

        let entities = &mut self.encounter.entities;
        let encounter_damage_stats = &mut self.encounter.encounter_damage_stats;

        let mut source_entity_state = entities
            .entry(source_entity.name.clone())
            .or_insert_with(|| encounter_entity_from_entity(source_entity));

        if !encounter_damage_stats
            .applied_shield_buffs
            .contains_key(&buff_id)
        {
            let mut source_id: Option<u32> = None;
            let original_buff_id = if let Some(deref_id) = self.custom_id_map.get(&buff_id) {
                source_id = Some(get_skill_id(buff_id));
                *deref_id
            } else {
                buff_id
            };

            if let Some(status_effect) = get_status_effect_data(original_buff_id, source_id) {
                encounter_damage_stats
                    .applied_shield_buffs
                    .insert(buff_id, status_effect);
            }
        }

        if source_entity.id == target_entity.id {
            source_entity_state.damage_stats.shields_received += shield;
            source_entity_state.damage_stats.shields_given += shield;
            source_entity_state
                .damage_stats
                .shields_given_by
                .entry(buff_id)
                .and_modify(|e| *e += shield)
                .or_insert(shield);
            source_entity_state
                .damage_stats
                .shields_received_by
                .entry(buff_id)
                .and_modify(|e| *e += shield)
                .or_insert(shield);

        } else {
            
            source_entity_state.damage_stats.shields_given += shield;
            source_entity_state
                .damage_stats
                .shields_given_by
                .entry(buff_id)
                .and_modify(|e| *e += shield)
                .or_insert(shield);

            let mut target_entity_state = entities
                .entry(target_entity.name.clone())
                .or_insert_with(|| encounter_entity_from_entity(target_entity));

            target_entity_state.damage_stats.shields_received += shield;
            target_entity_state
                .damage_stats
                .shields_received_by
                .entry(buff_id)
                .and_modify(|e| *e += shield)
                .or_insert(shield);
        }

        self.encounter.encounter_damage_stats.total_shielding += shield;
    }
}