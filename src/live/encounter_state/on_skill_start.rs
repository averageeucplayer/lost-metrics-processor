use hashbrown::HashMap;
use log::{info, warn};
use lost_metrics_core::models::*;
use lost_metrics_data::SKILL_DATA;
use lost_metrics_misc::*;
use std::default::Default;
use crate::live::utils::*;

use super::EncounterState;

impl EncounterState {

    pub fn on_skill_start(
        &mut self,
        source_entity: &Entity,
        skill_id: u32,
        tripod_index: Option<TripodIndex>,
        tripod_level: Option<TripodLevel>,
        timestamp: i64,
    ) -> (u32, Option<Vec<u32>>) {
        // do not track skills if encounter not started
        if self.encounter.fight_start == 0 {
            return (0, None);
        }
        let skill_name = get_skill_name(&skill_id);
        let mut tripod_change = false;
        let entity = self
            .encounter
            .entities
            .entry(source_entity.name.clone())
            .or_insert_with(|| {
                let (skill_name, skill_icon, summons) = get_skill_name_and_icon(
                    &skill_id,
                    &0,
                    skill_name.clone(),
                    &self.skill_tracker.skill_timestamp,
                    source_entity.id,
                );
                let mut entity = encounter_entity_from_entity(source_entity);
                entity.skill_stats = SkillStats {
                    casts: 0,
                    ..Default::default()
                };
                entity.skills = HashMap::from([(
                    skill_id,
                    Skill {
                        id: skill_id,
                        name: {
                            if skill_name.is_empty() {
                                skill_id.to_string()
                            } else {
                                skill_name
                            }
                        },
                        icon: skill_icon,
                        tripod_index,
                        tripod_level,
                        summon_sources: summons,
                        casts: 0,
                        ..Default::default()
                    },
                )]);
                tripod_change = true;
                entity
            });

        if entity.class_id == 0
            && source_entity.entity_type == EntityType::Player
            && source_entity.class_id > 0
        {
            entity.class_id = source_entity.class_id;
            entity.class = get_class_from_id(&source_entity.class_id);
        }

        entity.is_dead = false;
        entity.skill_stats.casts += 1;

        let relative_timestamp = if self.encounter.fight_start == 0 {
            0
        } else {
            (timestamp - self.encounter.fight_start) as i32
        };

        // if skills have different ids but the same name, we group them together
        // dunno if this is right approach xd
        let mut skill_id = skill_id;
        let mut skill_summon_sources: Option<Vec<u32>> = None;
        if let Some(skill) = entity.skills.get_mut(&skill_id) {
            skill.casts += 1;
            tripod_change = check_tripod_index_change(skill.tripod_index, tripod_index)
                || check_tripod_level_change(skill.tripod_level, tripod_level);
            skill.tripod_index = tripod_index;
            skill.tripod_level = tripod_level;
            skill_summon_sources.clone_from(&skill.summon_sources);
        } else if let Some(skill) = entity
            .skills
            .values_mut()
            .find(|s| s.name == skill_name.clone())
        {
            skill.casts += 1;
            skill_id = skill.id;
            tripod_change = check_tripod_index_change(skill.tripod_index, tripod_index)
                || check_tripod_level_change(skill.tripod_level, tripod_level);
            skill.tripod_index = tripod_index;
            skill.tripod_level = tripod_level;
            skill_summon_sources.clone_from(&skill.summon_sources);
        } else {
            let (skill_name, skill_icon, summons) = get_skill_name_and_icon(
                &skill_id,
                &0,
                skill_name.clone(),
                &self.skill_tracker.skill_timestamp,
                source_entity.id,
            );
            skill_summon_sources.clone_from(&summons);
            entity.skills.insert(
                skill_id,
                Skill {
                    id: skill_id,
                    name: {
                        if skill_name.is_empty() {
                            skill_id.to_string()
                        } else {
                            skill_name
                        }
                    },
                    icon: skill_icon,
                    tripod_index,
                    tripod_level,
                    summon_sources: summons,
                    casts: 1,
                    ..Default::default()
                },
            );
            tripod_change = true;
        }
        if tripod_change {
            if let (Some(tripod_index), Some(_tripod_level)) = (tripod_index, tripod_level) {
                let mut indexes = vec![tripod_index.first];
                if tripod_index.second != 0 {
                    indexes.push(tripod_index.second + 3);
                }
                // third row should never be set if second is not set
                if tripod_index.third != 0 {
                    indexes.push(tripod_index.third + 6);
                }
                // let levels = [tripod_level.first, tripod_level.second, tripod_level.third];
                // if let Some(effect) = SKILL_FEATURE_DATA.get(&skill_id) {
                //     for i in 0..indexes.len() {
                //         if let Some(entries) = effect.tripods.get(&indexes[i]) {
                //             let mut options: Vec<SkillFeatureOption> = vec![];
                //             for entry in &entries.entries {
                //                 if entry.level > 0 && entry.level == levels[i] {
                //                     options.push(entry.clone());
                //                 }
                //             }
                //             tripod_data.push(TripodData {
                //                 index: indexes[i],
                //                 options,
                //             });
                //         }
                //     }
                // }
            }

            // if !tripod_data.is_empty() {
            //     entity.skills.entry(skill_id).and_modify(|e| {
            //         e.tripod_data = Some(tripod_data);
            //     });
            // }
        }
        self.cast_log
            .entry(entity.name.clone())
            .or_default()
            .entry(skill_id)
            .or_default()
            .push(relative_timestamp);

        // if this is a getup skill and we have an ongoing abnormal move incapacitation, this will end it
        if let Some(skill_data) = SKILL_DATA.get(&skill_id) {
            if skill_data.skill_type == "getup" {
                for ongoing_event in entity
                    .damage_stats
                    .incapacitations
                    .iter_mut()
                    .rev()
                    .take_while(|x| x.timestamp + x.duration > timestamp)
                    .filter(|x| x.event_type == IncapacitationEventType::FallDown)
                {
                    info!(
                        "Shortening down duration from {} to {} because of getup skill",
                        ongoing_event.duration,
                        timestamp - ongoing_event.timestamp
                    );
                    ongoing_event.duration = timestamp - ongoing_event.timestamp;
                }
            }
        }

        (skill_id, skill_summon_sources)
    }

}