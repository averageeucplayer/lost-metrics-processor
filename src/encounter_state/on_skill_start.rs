use chrono::{DateTime, Utc};
use hashbrown::HashMap;
use log::{info, warn};
use lost_metrics_core::models::*;
use lost_metrics_data::{EntityExtensions, SKILL_DATA};
use lost_metrics_misc::*;
use std::default::Default;
use crate::utils::*;
use super::EncounterState;

impl EncounterState {

    pub fn on_skill_start(
        &mut self,
        source_id: u64,
        skill_id: u32,
        tripod_index: Option<lost_metrics_sniffer_stub::packets::definitions::TripodIndex>,
        tripod_level: Option<lost_metrics_sniffer_stub::packets::definitions::TripodLevel>,
        now: DateTime<Utc>,
    ) {
    
        let mut source_entity = self.get_source_entity(source_id);
        source_entity.borrow_mut().guess_is_player(skill_id);
        let source_entity = source_entity.borrow();

        let tripod_index =
        tripod_index
            .map(|tripod_index| lost_metrics_core::models::TripodIndex {
                first: tripod_index.first,
                second: tripod_index.second,
                third: tripod_index.third,
            });
        let tripod_level = tripod_level
            .map(|tripod_level| lost_metrics_core::models::TripodLevel {
                first: tripod_level.first,
                second: tripod_level.second,
                third: tripod_level.third,
            });

        if self.started_on == DateTime::<Utc>::MIN_UTC {
            return;
        }

        let skill_name = get_skill_name(&skill_id);
        let mut tripod_change = false;
        let entity_ptr = self
            .get_or_create_entity_with_skill_stats(&source_entity, skill_id, &skill_name, tripod_index, tripod_level) as *mut EncounterEntity;
        let entity = unsafe { &mut *entity_ptr };

        self.update_entity_class(entity, &source_entity);

        entity.is_dead = false;
        entity.skill_stats.casts += 1;

        let delta = now - self.started_on;
        let timestamp = delta.num_milliseconds();

        let (skill_id, skill_summon_sources) = self.update_or_insert_skill(entity, skill_id, &skill_name, tripod_index, tripod_level);

        if tripod_change {
            if let (Some(tripod_index), Some(_tripod_level)) = (tripod_index, tripod_level) {
                self.log_tripod_change(tripod_index);
            }
        }

        self.log_skill_cast(entity.name.clone(), skill_id, timestamp as i32);

        if let Some(skill_data) = SKILL_DATA.get(&skill_id) {
            if skill_data.skill_type == "getup" {
                entity.shorten_incapacitation(timestamp);
            }
        }

        if entity.entity_type == EntityType::Player && skill_id > 0 {
            self.new_cast(entity.id, skill_id, skill_summon_sources, now);
        }

    }

    fn get_or_create_entity_with_skill_stats(
        &mut self,
        source_entity: &Entity,
        skill_id: u32,
        skill_name: &str,
        tripod_index: Option<TripodIndex>,
        tripod_level: Option<TripodLevel>,
    ) -> &mut EncounterEntity {
        self.entity_stats.entry(source_entity.id)
            .or_insert_with(|| {
                let (name, icon, summons) = get_basic_skill_name_and_icon(
                    &skill_id,
                    skill_name.to_string(),
                    &self.skill_timestamp,
                    source_entity.id,
                );
                let mut entity: EncounterEntity = source_entity.into();
                entity.skill_stats = SkillStats { casts: 0, ..Default::default() };
                entity.skills = HashMap::from([(
                    skill_id,
                    Skill {
                        id: skill_id,
                        name: if name.is_empty() { skill_id.to_string() } else { name },
                        icon,
                        tripod_index,
                        tripod_level,
                        summon_sources: summons,
                        casts: 0,
                        ..Default::default()
                    },
                )]);
                entity
            })
    }

    fn update_entity_class(&self, entity: &mut EncounterEntity, source_entity: &Entity) {
        if entity.class_id == 0
            && source_entity.entity_type == EntityType::Player
            && source_entity.class_id != Class::Unknown
        {
            entity.class_id = source_entity.class_id as u32;
            entity.class = source_entity.class_id.as_ref().to_string();
        }
    }

    fn update_or_insert_skill(
        &mut self,
        entity: &mut EncounterEntity,
        mut skill_id: u32,
        skill_name: &str,
        tripod_index: Option<TripodIndex>,
        tripod_level: Option<TripodLevel>,
    ) -> (u32, Option<Vec<u32>>) {
        let mut skill_summon_sources = None;
        if let Some(skill) = entity.skills.get_mut(&skill_id) {
            skill.casts += 1;
            skill.tripod_index = tripod_index;
            skill.tripod_level = tripod_level;
            skill_summon_sources = skill.summon_sources.clone();
        } else if let Some(skill) = entity.skills.values_mut().find(|s| s.name == skill_name) {
            skill.casts += 1;
            skill_id = skill.id;
            skill.tripod_index = tripod_index;
            skill.tripod_level = tripod_level;
            skill_summon_sources = skill.summon_sources.clone();
        } else {
            let (name, icon, summons) = get_basic_skill_name_and_icon(
                &skill_id,
                skill_name.to_string(),
                &self.skill_timestamp,
                entity.id,
            );
            skill_summon_sources = summons.clone();
            entity.skills.insert(skill_id, Skill {
                id: skill_id,
                name: if name.is_empty() { skill_id.to_string() } else { name },
                icon,
                tripod_index,
                tripod_level,
                summon_sources: summons,
                casts: 1,
                ..Default::default()
            });
        }

        (skill_id, skill_summon_sources)
    }

    fn log_tripod_change(&self, tripod_index: TripodIndex) {
        let mut indexes = vec![tripod_index.first];
        if tripod_index.second != 0 {
            indexes.push(tripod_index.second + 3);
        }
        if tripod_index.third != 0 {
            indexes.push(tripod_index.third + 6);
        }
    }

    fn log_skill_cast(&mut self, entity_name: String, skill_id: u32, relative_timestamp: i32) {
        self.cast_log
            .entry(entity_name)
            .or_default()
            .entry(skill_id)
            .or_default()
            .push(relative_timestamp);
    }
}