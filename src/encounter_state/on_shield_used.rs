use lost_metrics_core::models::*;
use crate::utils::*;

use super::EncounterState;

impl EncounterState {

    pub fn on_shield_used(
        &mut self,
        source_entity_id: u64,
        target_entity_id: u64,
        buff_id: u32,
        shield_removed: u64,
    ) {
        // if source_entity.entity_type != EntityType::Player
        //     || target_entity.entity_type != EntityType::Player
        // {
        //     return;
        // }

        let entities = &mut self.entity_stats;
          
        let mut source_entity_state = entities
            .entry(source_entity_id)
            .or_default();
            // .or_insert_with(|| encounter_entity_from_entity(source_entity));

        if source_entity_id == target_entity_id {
            let damage_stats = &mut source_entity_state.damage_stats;

            damage_stats.damage_absorbed += shield_removed;
            damage_stats.damage_absorbed_on_others += shield_removed;
            damage_stats
                .damage_absorbed_by
                .entry(buff_id)
                .and_modify(|e| *e += shield_removed)
                .or_insert(shield_removed);
            damage_stats
                .damage_absorbed_on_others_by
                .entry(buff_id)
                .and_modify(|e| *e += shield_removed)
                .or_insert(shield_removed);


        } else {
            source_entity_state.damage_stats.damage_absorbed_on_others += shield_removed;

            source_entity_state
                .damage_stats
                .damage_absorbed_on_others_by
                .entry(buff_id)
                .and_modify(|e| *e += shield_removed)
                .or_insert(shield_removed);

            {
                let mut target_entity_state = entities
                    .entry(target_entity_id)
                    .or_default();
                    // .or_insert_with(|| encounter_entity_from_entity(target_entity));

                target_entity_state.damage_stats.damage_absorbed += shield_removed;
                target_entity_state
                    .damage_stats
                    .damage_absorbed_by
                    .entry(buff_id)
                    .and_modify(|e| *e += shield_removed)
                    .or_insert(shield_removed);
            }
        }

        self.encounter_damage_stats
            .total_effective_shielding += shield_removed;
    }

}