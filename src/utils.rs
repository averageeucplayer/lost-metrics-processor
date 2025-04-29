use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Instant};

use anyhow::Ok;
use chrono::{DateTime, Duration, Utc};
use hashbrown::HashMap;
use log::{info, warn};
use lost_metrics_core::models::*;
use lost_metrics_data::*;
use lost_metrics_misc::*;
use lost_metrics_sniffer_stub::packets::definitions::PKTPartyStatusEffectRemoveNotify;
use lost_metrics_sniffer_stub::packets::structures::{StatPair, StatusEffectData};
use lost_metrics_store::encounter_service::EncounterService;
use lost_metrics_store::models::CreateEncounter;
use tokio::task;
use uuid::Uuid;
use std::any::type_name;
use std::cell::RefCell;
use std::rc::Rc;

use crate::abstractions::StatsApi;
use crate::constants::{SEVEN_DAYS_SECONDS, TIMEOUT_DELAY_MS, WORKSHOP_BUFF_ID};
use crate::models::CompleteEncounter;

use super::abstractions::{AppEvent, EventEmitter};
use super::encounter_state::EncounterState;
use super::StartOptions;

pub fn encounter_entity_from_entity(entity: &Entity) -> EncounterEntity {
    let mut entity = EncounterEntity {
        id: entity.id,
        name: entity.name.clone(),
        entity_type: entity.entity_type,
        npc_id: entity.npc_id,
        class_id: entity.class_id as u32,
        class: entity.class_id.as_ref().to_string(),
        gear_score: entity.gear_level,
        ..Default::default()
    };

    if entity.character_id > 0 {
        entity.character_id = entity.character_id;
    }

    entity
}


pub fn get_skill_name(skill_id: &u32) -> String {
    SKILL_DATA
        .get(skill_id)
        .map_or(skill_id.to_string(), |skill| {
            if skill.name.is_none() {
                skill_id.to_string()
            } else {
                skill.name.clone().unwrap_or_default()
            }
        })
}

pub fn get_skill(skill_id: &u32) -> Option<SkillData> {
    SKILL_DATA.get(skill_id).cloned()
}

pub fn get_new_id(source_skill: u32) -> u32 {
    source_skill + 1_000_000_000
}

pub fn get_skill_id(new_skill: u32) -> u32 {
    new_skill - 1_000_000_000
}

pub fn get_current_and_max_hp(stat_pair: &[StatPair]) -> (i64, i64) {
    let mut hp: Option<i64> = None;
    let mut max_hp: Option<i64> = None;

    for pair in stat_pair {
        match pair.stat_type as u32 {
            1 => hp = Some(pair.value),
            27 => max_hp = Some(pair.value),
            _ => {}
        }
        if hp.is_some() && max_hp.is_some() {
            break;
        }
    }

    (hp.unwrap_or_default(), max_hp.unwrap_or_default())
}

pub fn send_to_ui<EE: EventEmitter>(
    now: DateTime<Utc>,
    state: &mut EncounterState,
    event_emitter: Arc<EE>,
    options: &StartOptions) {
    
    let boss_dead = state.boss_dead_update;
    
    if state.boss_dead_update {
        state.boss_dead_update = false;
    }

    // let mut encounter = state.encounter.clone();
    // let current_boss_name = &encounter.current_boss_name;
    // let entities = &mut encounter.entities;

    // if !current_boss_name.is_empty() {
    //     let current_boss = entities.get(current_boss_name).cloned();
    //     if let Some(mut current_boss) = current_boss {
    //         if boss_dead {
    //             current_boss.is_dead = true;
    //             current_boss.current_hp = 0;
    //         }
    //         encounter.current_boss = Some(current_boss);
    //     } else {
    //         encounter.current_boss_name = String::new();
    //     }
    // }

    // entities.retain(|_, entity| entity.is_valid());
    // let damage_valid = state.damage_is_valid;

    // // let can_get_party_info = (now - state.last_party_update) >= options.party_duration && !state.party_freeze;

    // let party_info = get_party_info(now, state, options);

    // if !encounter.entities.is_empty() {
    //     // tokio::task::spawn(async move {          
    //     // });

    //     event_emitter
    //         .emit("encounter-update", Some(encounter))
    //         .expect("failed to emit encounter-update");

    //     if !damage_valid {
    //         event_emitter
    //             .emit("invalid-damage", ())
    //             .expect("failed to emit invalid-damage");
    //     }

    //     if party_info.is_some() {
    //         event_emitter
    //             .emit("party-update", party_info)
    //             .expect("failed to emit party-update");
    //     }
    // }
   
}

fn get_party_info(now: DateTime<Utc>, state: &mut EncounterState, options: &StartOptions) -> Option<HashMap<i32, Vec<String>>>  {
   
    // we used cached party if it exists
    if state.party_cache.is_some() {
        return Some(state.party_map_cache.clone())
    }

    let party = state.get_party();
    if party.len() > 1 {
        let current_party: HashMap<i32, Vec<String>> = party
            .iter()
            .enumerate()
            .map(|(index, party)| (index as i32, party.clone()))
            .collect();

        if party.iter().all(|p| p.len() == 4) {
            state.party_cache = Some(party.clone());
            state.party_map_cache.clone_from(&current_party);
        }

        return Some(current_party)
    }

    return None
}

/// Truncates a gear level to two decimal places without rounding.
///
/// This function multiplies the input by `100`, truncates the result
/// towards zero (drops extra decimals), and then divides back by `100`,
/// effectively keeping only two decimal places.
///
/// Useful for formatting or displaying gear levels where exact truncation is required.
///
/// # Arguments
///
/// * `gear_level` - The original gear level as a `f32`.
///
/// # Returns
///
/// * A `f32` with at most two decimal places, truncated (not rounded).
///
/// # Example
///
/// ```ignore
/// let level = truncate_gear_level(1415.278);
/// assert_eq!(level, 1415.27);
/// ```
pub fn truncate_gear_level(gear_level: f32) -> f32 {
    f32::trunc(gear_level * 100.) / 100.
}

/// Extracts a `u64` value from an optional byte buffer representing a status effect value.
///
/// This function reads two 8-byte chunks (if available) from the provided byte buffer.
/// It then returns the **minimum** of the two interpreted `u64` values.
/// If the buffer is `None`, or fewer than 8/16 bytes are available, it defaults missing reads to `0`.
///
/// # Arguments
///
/// * `value` - An optional `Vec<u8>` slice containing raw bytes.
///
/// # Returns
///
/// * A `u64` value extracted from the bytes, or `0` if unavailable.
///
/// # Example
///
/// ```ignore
/// let value_bytes = Some(vec![1, 0, 0, 0, 0, 0, 0, 0,   // 1 as u64
///                              5, 0, 0, 0, 0, 0, 0, 0]); // 5 as u64
///
/// let value = get_status_effect_value(&value_bytes);
/// assert_eq!(value, 1);
///
/// let none_value: Option<Vec<u8>> = None;
/// assert_eq!(get_status_effect_value(&none_value), 0);
/// ```
pub fn get_status_effect_value(value: &Option<Vec<u8>>) -> u64 {
    value.as_ref().map_or(0, |v| {
        let c1 = v
            .get(0..8)
            .map_or(0, |bytes| u64::from_le_bytes(bytes.try_into().unwrap()));
        let c2 = v
            .get(8..16)
            .map_or(0, |bytes| u64::from_le_bytes(bytes.try_into().unwrap()));
        c1.min(c2)
    })
}

pub fn build_status_effect(
    se_data: StatusEffectData,
    target_id: u64,
    source_id: u64,
    target_type: StatusEffectTargetType,
    timestamp: DateTime<Utc>,
) -> StatusEffectDetails {
    let value = get_status_effect_value(&se_data.value.bytearray_0);
    let mut status_effect_category = StatusEffectCategory::Other;
    let mut buff_category = StatusEffectBuffCategory::Other;
    let mut show_type = StatusEffectShowType::Other;
    let mut status_effect_type = StatusEffectType::Other;
    let mut name = "Unknown".to_string();
    let mut db_target_type = "".to_string();
    let mut source_skills = vec![];
    let custom_id = 0;

    if let Some(effect) = SKILL_BUFF_DATA.get(&se_data.status_effect_id) {
        name = effect.name.clone().unwrap_or_default();
        if effect.category.as_str() == "debuff" {
            status_effect_category = StatusEffectCategory::Debuff
        }
        buff_category = effect.buff_category.clone().unwrap_or_default().as_str().into();
        if effect.icon_show_type.clone().unwrap_or_default() == "all" {
            show_type = StatusEffectShowType::All
        }
        status_effect_type = effect.buff_type.as_str().into();
        db_target_type = effect.target.to_string();

        source_skills = effect.source_skills.clone().unwrap_or_default();
    }

    if se_data.status_effect_id == WORKSHOP_BUFF_ID {
        status_effect_type = StatusEffectType::Workshop;
    }

    let expiry = (se_data.total_time > 0. && se_data.total_time < SEVEN_DAYS_SECONDS).then(|| {
        timestamp
            + Duration::milliseconds((se_data.total_time as i64) * 1000 + TIMEOUT_DELAY_MS)
    });

    StatusEffectDetails {
        instance_id: se_data.status_effect_instance_id,
        source_id,
        target_id,
        status_effect_id: se_data.status_effect_id,
        custom_id,
        source_skills,
        target_type,
        db_target_type,
        value,
        stack_count: se_data.stack_count,
        buff_category,
        category: status_effect_category,
        status_effect_type,
        show_type,
        expiration_delay: se_data.total_time,
        expire_at: expiry,
        end_tick: se_data.end_tick,
        name,
        timestamp,
    }
}

pub fn select_most_recent_valid_skill(
    source_skills: &[u32],
    entity_skills: &HashMap<u32, Skill>,
) -> u32 {
    let mut last_time = i64::MIN;
    let mut last_skill = None;

    for source_skill_id in source_skills {
        if let Some(skill) = entity_skills.get(source_skill_id) {

            if skill.id == BardSkills::Stigma as u32 {
                if let Some(tripods) = skill.tripod_index {
                    if tripods.second != 2 {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            if skill.last_timestamp > last_time {
                last_time = skill.last_timestamp;
                last_skill = Some(*source_skill_id);
            }
        }
    }

    last_skill.unwrap_or_default()
}

pub fn is_valid_difficulty(difficulty: &str) -> bool {
    difficulty == "Normal"
        || difficulty == "Hard"
        || difficulty == "The First"
        || difficulty == "Trial"
}

pub fn is_valid_raid(raid_name: &str) -> bool {
    matches!(
        raid_name,
        "Act 2: Brelshaza G1" | 
        "Act 2: Brelshaza G2" | 
        "Aegir G1" |
        "Aegir G2" |
        "Behemoth G1" |
        "Behemoth G2" |
        "Echidna G1"|
        "Echidna G2"|
        "Thaemine G1"|
        "Thaemine G2"|
        "Thaemine G3"|
        "Thaemine G4"|
        // g-raids
        "Skolakia"|
        "Argeos"
    )
}

pub fn save_to_db<EE: EventEmitter, ES: EncounterService, SA: StatsApi>(
    version: &str,
    summary: CompleteEncounter,
    stats_api: Arc<SA>,
    encounter_service: Arc<ES>,
    event_emitter: Arc<EE>
    ) {
    
    // info!(
    //     "saving to db - cleared: [{}], difficulty: [{}] {}",
    //     raid_clear, self.raid_difficulty, encounter.current_boss_name
    // );

    let version = version.to_string();

    let handle = task::spawn(async move {
        let stats_api = stats_api.clone();

        let create_encounter = CreateEncounter {
            encounter: Encounter {
                ..Default::default()
            },
            prev_stagger: summary.prev_stagger,
            damage_log: summary.damage_log,
            identity_log: summary.identity_log,
            cast_log: summary.cast_log,
            boss_hp_log: summary.boss_hp_log,
            stagger_log: summary.stagger_log,
            stagger_intervals: summary.stagger_intervals,
            raid_clear: summary.is_raid_clear,
            party_info: summary.party_info,
            raid_difficulty: summary.raid_difficulty,
            region: summary.region,
            version,
            ntp_fight_start: summary.ntp_fight_start,
            rdps_valid: summary.rdps_valid,
            manual: summary.manual,
            skill_cast_log: summary.skill_cast_log,
            player_info: None
        };

        let encounter_id = encounter_service.create(create_encounter)
            .expect("failed to commit transaction");
        info!("saved to db");

        if summary.is_raid_clear {
            event_emitter
                .emit(AppEvent::ClearEncounter(encounter_id))
                .expect("failed to emit clear-encounter");
        }
    });
    
}