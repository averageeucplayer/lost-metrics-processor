use std::fmt::Debug;
use std::sync::Arc;
use std::time::Instant;

use anyhow::Ok;
use hashbrown::HashMap;
use log::warn;
use lost_metrics_core::models::*;
use lost_metrics_data::*;
use lost_metrics_misc::*;
use lost_metrics_sniffer_stub::packets::definitions::PKTPartyStatusEffectRemoveNotify;
use lost_metrics_sniffer_stub::packets::structures::StatPair;
use std::any::type_name;
use std::cell::RefCell;
use std::rc::Rc;

use super::abstractions::EventEmitter;
use super::encounter_state::EncounterState;
use super::entity_tracker::EntityTracker;
use super::id_tracker::IdTracker;
use super::StartOptions;

pub fn encounter_entity_from_entity(entity: &Entity) -> EncounterEntity {
    let mut entity = EncounterEntity {
        id: entity.id,
        name: entity.name.clone(),
        entity_type: entity.entity_type,
        npc_id: entity.npc_id,
        class_id: entity.class_id,
        class: get_class_from_id(&entity.class_id),
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

pub fn map_status_effect(se: &StatusEffectDetails, custom_id_map: &mut HashMap<u32, u32>) -> u32 {
    if se.custom_id > 0 {
        custom_id_map.insert(se.custom_id, se.status_effect_id);
        se.custom_id
    } else {
        se.status_effect_id
    }
}

pub fn get_new_id(source_skill: u32) -> u32 {
    source_skill + 1_000_000_000
}

pub fn get_skill_id(new_skill: u32) -> u32 {
    new_skill - 1_000_000_000
}

pub fn on_shield_change(
    entity_tracker: &mut EntityTracker,
    id_tracker: &Rc<RefCell<IdTracker>>,
    state: &mut EncounterState,
    status_effect: StatusEffectDetails,
    change: u64,
) {
    if change == 0 {
        return;
    }

    let source = entity_tracker.get_source_entity(status_effect.source_id);
    let target_id = if status_effect.target_type == StatusEffectTargetType::Party {
        id_tracker
            .borrow()
            .get_entity_id(status_effect.target_id)
            .unwrap_or_default()
    } else {
        status_effect.target_id
    };
    let target = entity_tracker.get_source_entity(target_id);
    state.on_boss_shield(&target, status_effect.value);
    state.on_shield_used(&source, &target, status_effect.status_effect_id, change);
}

#[deprecated(
    note = "Use `parse_pkt1`"
)]
pub fn parse_pkt<T, F>(data: &[u8], new_fn: F) -> Option<T>
where
    T: Debug,
    F: FnOnce(&[u8]) -> Result<T, anyhow::Error>,
{
    match new_fn(data) {
        std::result::Result::Ok(packet) => Some(packet),
        Err(e) => {
            warn!("Error parsing {}: {}", type_name::<T>(), e);
            None
        }
    }
}

pub fn parse_pkt1<T, F>(data: &[u8], new_fn: F) -> anyhow::Result<T>
    where
        T: Debug,
        F: FnOnce(&[u8]) -> Result<T, anyhow::Error>,
{
    new_fn(data).map_err(|e| anyhow::anyhow!("Error parsing: {}: {}", type_name::<T>(), e))
}

pub fn get_current_and_max_hp(stat_pair: &Vec<StatPair>) -> (i64, i64) {
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
    state: &mut EncounterState,
    event_emitter: Arc<EE>,
    options: &StartOptions) -> Instant {
    
    let boss_dead = state.boss_dead_update;
    
    if state.boss_dead_update {
        state.boss_dead_update = false;
    }

    let mut encounter = state.encounter.clone();
    let current_boss_name = &encounter.current_boss_name;
    let entities = &mut encounter.entities;

    if !current_boss_name.is_empty() {
        let current_boss = entities.get(current_boss_name).cloned();
        if let Some(mut current_boss) = current_boss {
            if boss_dead {
                current_boss.is_dead = true;
                current_boss.current_hp = 0;
            }
            encounter.current_boss = Some(current_boss);
        } else {
            encounter.current_boss_name = String::new();
        }
    }

    entities.retain(|_, entity| entity.is_valid());
    let damage_valid = state.damage_is_valid;
    let party_info = get_party_info(state, options);

    if !encounter.entities.is_empty() {
        // tokio::task::spawn(async move {          
        // });

        event_emitter
            .emit("encounter-update", Some(encounter))
            .expect("failed to emit encounter-update");

        if !damage_valid {
            event_emitter
                .emit("invalid-damage", "")
                .expect("failed to emit invalid-damage");
        }

        if party_info.is_some() {
            event_emitter
                .emit("party-update", party_info)
                .expect("failed to emit party-update");
        }
    }
   
    Instant::now()
}

fn get_party_info(state: &mut EncounterState, options: &StartOptions) -> Option<HashMap<i32, Vec<String>>>  {
    let can_get_party_info = state.last_party_update.elapsed() >= options.party_duration && !state.party_freeze;

    if !can_get_party_info {
        return None;
    }

    state.last_party_update = Instant::now();
    // we used cached party if it exists
    if state.party_cache.is_some() {
        return Some(state.party_map_cache.clone())
    }

    let party = state.get_party_from_tracker();
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
