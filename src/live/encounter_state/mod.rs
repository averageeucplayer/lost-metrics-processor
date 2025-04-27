mod on_damage;
mod save_to_db;
mod on_cc_applied;
mod on_cc_removed;
mod on_shield_applied;
mod on_init_env;
mod on_new_npc;
mod on_shield_used;
mod on_skill_start;

use chrono::{Date, DateTime, Utc};
use hashbrown::hash_map::Entry;
use hashbrown::{HashMap, HashSet};
use log::{info, warn};
use lost_metrics_core::models::*;
use lost_metrics_misc::{boss_to_raid_map};
use lost_metrics_sniffer_stub::packets::definitions::{PKTIdentityGaugeChangeNotify, PKTPartyInfo, PKTPartyInfoInner, PKTPartyStatusEffectAddNotify};
use lost_metrics_sniffer_stub::packets::structures::{EquipItemData, StatPair, StatusEffectData};
use lost_metrics_store::encounter_service::EncounterService;
use moka::sync::Cache;
use rsntp::SntpClient;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use uuid::Uuid;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::default::Default;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use crate::constants::WORKSHOP_BUFF_ID;
use crate::live::utils::*;
use super::abstractions::EventEmitter;
use super::stats_api::{is_valid_raid, SendRaidInfo, StatsApi};

pub type StatusEffectRegistry = HashMap<u32, StatusEffectDetails>;

#[derive(Debug)]
pub struct EncounterState {

    pub entities: HashMap<u64, Entity>,
    pub fight_start: i64,
    pub skills: HashMap<(u64, u32, i64), SkillCast>,
    pub projectile_id_to_timestamp: Cache<u64, i64>,
    pub skill_timestamp: Cache<(u64, u32), i64>,
    pub local_status_effect_registry: HashMap<u64, StatusEffectRegistry>,
    pub party_status_effect_registry: HashMap<u64, StatusEffectRegistry>,
    pub character_id_to_party_id: HashMap<u64, u32>,
    pub entity_id_to_party_id: HashMap<u64, u32>,
    pub raid_instance_to_party_ids: HashMap<u32, HashSet<u32>>,
    pub character_name_to_character_id: HashMap<String, u64>,
    pub local_player_name: Option<String>,
    pub local_entity_id: u64,
    pub local_character_id: u64,
    pub character_id_to_entity_id: HashMap<u64, u64>,
    pub entity_id_to_character_id: HashMap<u64, u64>,

    pub client_id: Option<Uuid>,
    pub party_freeze: bool,
    pub party_cache: Option<Vec<Vec<String>>>,
    pub party_map_cache: HashMap<i32, Vec<String>>,
    pub encounter: Encounter,
    pub resetting: bool,
    pub boss_dead_update: bool,
    pub saved: bool,
    pub raid_end_cd: DateTime<Utc>,
    pub raid_clear: bool,
    pub is_valid_zone: bool,
    pub last_update: DateTime<Utc>,
    pub last_party_update: DateTime<Utc>,
    pub prev_stagger: i32,
    pub damage_log: HashMap<String, Vec<(i64, i64)>>,
    pub identity_log: HashMap<String, IdentityLog>,
    pub cast_log: HashMap<String, HashMap<u32, Vec<i32>>>,
    pub boss_hp_log: HashMap<String, Vec<BossHpLog>>,
    pub stagger_log: Vec<(i32, f32)>,
    pub stagger_intervals: Vec<(i32, i32)>,
    pub party_info: Vec<Vec<String>>,
    pub raid_difficulty: String,
    pub raid_difficulty_id: u32,
    pub boss_only_damage: bool,
    pub region: Option<String>,
    sntp_client: SntpClient,
    ntp_fight_start: i64,
    pub rdps_valid: bool,
    custom_id_map: HashMap<u32, u32>,
    pub damage_is_valid: bool,
}

impl EncounterState {
    pub fn new() -> EncounterState {
        EncounterState {
            entities: HashMap::new(),
            fight_start: -1,
            skills: HashMap::new(),
            projectile_id_to_timestamp: Cache::builder()
                .time_to_idle(Duration::from_secs(20))
                .build(),
            skill_timestamp: Cache::builder()
                .time_to_idle(Duration::from_secs(20))
                .build(),
            local_status_effect_registry: HashMap::new(),
            party_status_effect_registry: HashMap::new(),
            character_id_to_party_id: HashMap::new(),
            entity_id_to_party_id: HashMap::new(),
            raid_instance_to_party_ids: HashMap::new(),
            character_name_to_character_id: HashMap::new(),
            local_player_name: None,
            local_entity_id: 0,
            local_character_id: 0,
            character_id_to_entity_id: HashMap::new(),
            entity_id_to_character_id: HashMap::new(),
            
            last_party_update: DateTime::<Utc>::MIN_UTC,
            last_update: DateTime::<Utc>::MIN_UTC,
            raid_end_cd: DateTime::<Utc>::MIN_UTC,
            client_id: None,
            party_freeze: false,
            party_cache: None,
            party_map_cache: HashMap::new(),
            encounter: Encounter::default(),
            resetting: false,
            raid_clear: false,
            boss_dead_update: false,
            saved: false,
            is_valid_zone: true,

            prev_stagger: 0,
            damage_log: HashMap::new(),
            identity_log: HashMap::new(),
            boss_hp_log: HashMap::new(),
            cast_log: HashMap::new(),
            stagger_log: Vec::new(),
            stagger_intervals: Vec::new(),

            party_info: Vec::new(),
            raid_difficulty: "".to_string(),
            raid_difficulty_id: 0,
            boss_only_damage: false,
            region: None,
            sntp_client: SntpClient::new(),
            ntp_fight_start: 0,
            rdps_valid: false,
            custom_id_map: HashMap::new(),
            damage_is_valid: true,
        }
    }

    pub fn party_info(
        &mut self,
        party_instance_id: u32,
        raid_instance_id: u32,
        party_member_datas: Vec<PKTPartyInfoInner>,
        local_info: &LocalInfo) {
        let mut unknown_local = self.entities.get(&self.local_entity_id)
            .map(|p| p.name.is_empty() || p.name == "You" || p.name.starts_with('0'))
            .unwrap_or(true);

        let party_id = party_instance_id;
        self.character_id_to_party_id.retain(|_, &mut p_id| p_id != party_id);
        self.entity_id_to_party_id.retain(|_, &mut p_id| p_id != party_id);

        let most_likely_local_name = if unknown_local {
            let party_members = party_member_datas
                .iter()
                .map(|m| m.character_id)
                .collect::<Vec<u64>>();
            let mut party_locals = local_info
                .local_players
                .iter()
                .filter_map(|(k, v)| {
                    if party_members.contains(k) {
                        Some(v)
                    } else {
                        None
                    }
                })
                .collect::<Vec<&LocalPlayer>>();
            party_locals.sort_by(|a, b| b.count.cmp(&a.count));
            party_locals
                .first()
                .map_or_else(String::new, |p| p.name.clone())
        } else {
            "".to_string()
        };

        for member in party_member_datas {
            if unknown_local && member.name == most_likely_local_name {
                if let Some(local_player) = self.entities.get_mut(&self.local_entity_id) {
                    unknown_local = false;
                    warn!(
                        "unknown local player, inferring from cache: {}",
                        member.name
                    );
                    local_player.entity_type = EntityType::Player;
                    local_player.class_id = member.class_id.into();
                    local_player.gear_level = truncate_gear_level(member.gear_level);
                    local_player.name.clone_from(&member.name);
                    local_player.character_id = member.character_id;

                    self.character_id_to_entity_id.insert(member.character_id, self.local_entity_id);
                    self.entity_id_to_character_id.insert(self.local_entity_id, member.character_id);
                    self.local_player_name = Some(member.name.clone());
                }
            }

            let entity_id = self.character_id_to_entity_id.get(&member.character_id).copied();

            if let Some(entity_id) = entity_id {
                if let Some(entity) = self.entities.get_mut(&entity_id) {
                    if entity.entity_type == EntityType::Player && entity.name == member.name {
                        entity.gear_level = truncate_gear_level(member.gear_level);
                        entity.class_id = member.class_id.into()
                    }
                }

                self.add_party_mapping(
                    raid_instance_id,
                    party_instance_id,
                    member.character_id,
                    entity_id,
                    Some(member.name.clone()),
                );
            } else {
                self.add_party_mapping(
                    raid_instance_id,
                    party_instance_id,
                    member.character_id,
                    0,
                    Some(member.name.clone()),
                );
            }
        }

        let local_player_id = self.local_entity_id;

        if let Some(entity) = self.entities.get(&local_player_id) {
            let entities = &mut self.encounter.entities;

            // we replace the existing local player if it exists, since its name might have changed (from hex or "You" to character name)
            if let Some(mut local) = entities.remove(&self.encounter.local_player) {
                // update local player name, insert back into encounter
                self.encounter.local_player.clone_from(&entity.name);
                
                local.update(&entity);
                local.class = entity.class_id.as_ref().to_string();
    
                entities.insert(self.encounter.local_player.clone(), local);
            } else {
                // cannot find old local player by name, so we look by local player's entity id
                // this can happen when the user started meter late
                let old_local = entities
                    .iter()
                    .find(|(_, e)| e.id == entity.id)
                    .map(|(key, _)| key.clone());
    
                // if we find the old local player, we update its name and insert back into encounter
                if let Some(old_local) = old_local {
                    let mut new_local = entities[&old_local].clone();
                    
                    new_local.update(&entity);
                    new_local.class = entity.class_id.as_ref().to_string();
    
                    entities.remove(&old_local);
                    self.encounter.local_player.clone_from(&entity.name);
                    entities.insert(self.encounter.local_player.clone(), new_local);
                }
            }
        }

        self.party_cache = None;
        self.party_map_cache = HashMap::new();
    }

    pub fn on_status_effect_add(
        &mut self,
        now: DateTime<Utc>,
        target_id: u64,
        status_effect_data: StatusEffectData) {
        let source_entity = self.get_source_entity(status_effect_data.source_id).clone();
        let source_entity_name = source_entity.name.clone();
        let mut status_effect = build_status_effect(
            status_effect_data,
            target_id,
            source_entity.id,
            StatusEffectTargetType::Local,
            now,
        );

        let target = self.get_source_entity(target_id).clone();

        if status_effect.status_effect_type == StatusEffectType::Shield {
            self.on_boss_shield(&target, status_effect.value);
            self.on_shield_applied(
                &source_entity,
                &target,
                status_effect.status_effect_id,
                status_effect.value,
            );
        }

        let source_encounter_entity = self.encounter.entities.get(&source_entity_name);
        if let Some(encounter_entity) = source_encounter_entity {
            let custom_id = select_most_recent_valid_skill(&status_effect.source_skills, &encounter_entity.skills);
            status_effect.custom_id = custom_id;
        }

        if status_effect.status_effect_type == StatusEffectType::HardCrowdControl {
            if target.entity_type == EntityType::Player {
                self.on_cc_applied(&target, &status_effect);
            }
        }
        
        self.register_status_effect(status_effect);
    }

    pub fn should_use_party_status_effect(&self, character_id: u64, local_character_id: u64) -> bool {
        let local_player_party_id = self.character_id_to_party_id.get(&local_character_id);
        let affected_player_party_id = self.character_id_to_party_id.get(&character_id);

        match (
            local_player_party_id,
            affected_player_party_id,
            character_id == local_character_id,
        ) {
            (Some(local_party), Some(affected_party), false) => local_party == affected_party,
            _ => false,
        }
    }

    pub fn register_status_effect(&mut self, status_effect: StatusEffectDetails) -> &mut StatusEffectDetails {
        let registry = match status_effect.target_type {
            StatusEffectTargetType::Local => &mut self.local_status_effect_registry,
            StatusEffectTargetType::Party => &mut self.party_status_effect_registry,
        };

        let ser = registry.entry(status_effect.target_id).or_default();
        let status_effect = ser.entry(status_effect.instance_id).or_insert_with(|| status_effect);

        status_effect
    }

    pub fn on_party_status_effect_add(
        &mut self,
        now: DateTime<Utc>,
        character_id: u64,
        status_effect_datas: Vec<StatusEffectData>) {
        let target_entity_id = self.character_id_to_entity_id.get(&character_id).copied().unwrap_or_default();
        let shields = self.party_status_effect_add(now, character_id, status_effect_datas);
        let current_boss_name = self.encounter.current_boss_name.clone();

        for status_effect in shields {
           
            let target_id =
                if status_effect.target_type == StatusEffectTargetType::Party {
                    target_entity_id
                } else {
                    status_effect.target_id
                };
            let target_name = self.get_source_entity(target_id).name.clone();
            
            if target_name == current_boss_name {
                self.encounter.entities
                    .entry(target_name)
                    .and_modify(|e| {
                        e.current_shield = status_effect.value;
                    });
            }

            let source = self.get_source_entity(status_effect.source_id).clone();
            let target = self.get_source_entity(target_id).clone();

            self.on_shield_applied(
                &source,
                &target,
                status_effect.status_effect_id,
                status_effect.value,
            );
        }
    }

    pub fn add_party_mapping(
        &mut self,
        raid_instance_id: u32,
        party_id: u32,
        mut character_id: u64,
        mut entity_id: u64,
        name: Option<String>,
    ) {
        if character_id == 0 && entity_id == 0 {
            return;
        }

        if character_id > 0 && entity_id == 0 {
            entity_id = self.character_id_to_entity_id.get(&character_id).copied().unwrap_or(0);
        } else if character_id == 0 && entity_id > 0 {
            character_id = self.entity_id_to_character_id.get(&entity_id).copied().unwrap_or(0);
        }
        
        if character_id > 0 {

            self.character_id_to_party_id.insert(character_id, party_id);
            if let Some(name) = name {
                self.character_name_to_character_id
                    .insert(name, character_id);
            }
        }

        if entity_id > 0 {
            self.entity_id_to_party_id.insert(entity_id, party_id);
        }

        let party_instance = self.raid_instance_to_party_ids
            .entry(raid_instance_id)
            .or_default();
        party_instance.insert(party_id);
    }

    pub fn sync_status_effect(
        &mut self,
        instance_id: u32,
        character_id: u64,
        object_id: u64,
        value: u64,
        local_character_id: u64,
    ) -> (Option<StatusEffectDetails>, u64) {
        let use_party = self.should_use_party_status_effect(character_id, local_character_id);
        let (target_id, sett) = if use_party {
            (character_id, StatusEffectTargetType::Party)
        } else {
            (object_id, StatusEffectTargetType::Local)
        };
        if target_id == 0 {
            return (None, 0);
        }
        let registry = match sett {
            StatusEffectTargetType::Local => &mut self.local_status_effect_registry,
            StatusEffectTargetType::Party => &mut self.party_status_effect_registry,
        };

        let ser = match registry.get_mut(&target_id) {
            Some(ser) => ser,
            None => return (None, 0),
        };

        let se = match ser.get_mut(&instance_id) {
            Some(se) => se,
            None => return (None, 0),
        };

        let old_value = se.value;
        se.value = value;

        (Some(se.clone()), old_value)
    }

    pub fn actually_get_status_effects(
        &mut self,
        target_id: u64,
        sett: StatusEffectTargetType,
        timestamp: DateTime<Utc>,
    ) -> Vec<StatusEffectDetails> {
        let registry = match sett {
            StatusEffectTargetType::Local => &mut self.local_status_effect_registry,
            StatusEffectTargetType::Party => &mut self.party_status_effect_registry,
        };

        let ser = match registry.get_mut(&target_id) {
            Some(ser) => ser,
            None => return Vec::new(),
        };
        ser.retain(|_, se| se.expire_at.map_or(true, |expire_at| expire_at > timestamp));
        ser.values().cloned().collect()
    }

    pub fn get_status_effects_from_party(
        &mut self,
        target_id: u64,
        sett: StatusEffectTargetType,
        party_id: &u32,
        timestamp: DateTime<Utc>,
    ) -> Vec<StatusEffectDetails> {
        let registry = match sett {
            StatusEffectTargetType::Local => &mut self.local_status_effect_registry,
            StatusEffectTargetType::Party => &mut self.party_status_effect_registry,
        };

        let ser = match registry.get_mut(&target_id) {
            Some(ser) => ser,
            None => return Vec::new(),
        };

        ser.retain(|_, se| se.expire_at.map_or(true, |expire_at| expire_at > timestamp));

        ser.values()
            .filter(|sed| {
                sed.is_valid_for_raid()
                    || *party_id
                        == self.entity_id_to_party_id
                            .get(&sed.source_id)
                            .cloned()
                            .unwrap_or(0)
            })
            .cloned()
            .collect()
    }

    pub fn get_or_create_entity(&mut self, id: u64) -> &mut Entity {
        match self.entities.entry(id) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                e.insert(Entity {
                    id,
                    entity_type: EntityType::Unknown,
                    name: format!("{:x}", id),
                    ..Default::default()
                })
            }
        }
    }

    pub fn get_status_effects(
        &mut self,
        source_entity: &Entity,
        target_entity: &Entity,
        local_character_id: u64,
    ) -> (Vec<StatusEffectDetails>, Vec<StatusEffectDetails>) {
        let timestamp = Utc::now();

        let use_party_for_source = if source_entity.entity_type == EntityType::Player {
            self.should_use_party_status_effect(source_entity.character_id, local_character_id)
        } else {
            false
        };

        let (source_id, source_type) = if use_party_for_source {
            (source_entity.character_id, StatusEffectTargetType::Party)
        } else {
            (source_entity.id, StatusEffectTargetType::Local)
        };


        let status_effects_on_source =
            self.actually_get_status_effects(source_id, source_type, timestamp);

        let use_party_for_target = if source_entity.entity_type == EntityType::Player {
            self.should_use_party_status_effect(target_entity.character_id, local_character_id)
        } else {
            false
        };

        let source_party_id = self.entity_id_to_party_id.get(&source_entity.id).cloned();

        let mut status_effects_on_target = match (use_party_for_target, source_party_id) {
            (true, Some(source_party_id)) => self.get_status_effects_from_party(
                target_entity.character_id,
                StatusEffectTargetType::Party,
                &source_party_id,
                timestamp,
            ),
            (false, Some(source_party_id)) => self.get_status_effects_from_party(
                target_entity.id,
                StatusEffectTargetType::Local,
                &source_party_id,
                timestamp,
            ),
            (true, None) => self.actually_get_status_effects(
                target_entity.character_id,
                StatusEffectTargetType::Party,
                timestamp,
            ),
            (false, None) => self.actually_get_status_effects(
                target_entity.id,
                StatusEffectTargetType::Local,
                timestamp,
            ),
        };

        status_effects_on_target.retain(|se| {
            !(se.target_type == StatusEffectTargetType::Local
                && se.category == StatusEffectCategory::Debuff
                && se.source_id != source_id
                && se.db_target_type == "self")
        });
        (status_effects_on_source, status_effects_on_target)
    }

    pub fn id_is_player(&mut self, id: u64) -> bool {
        if let Some(entity) = self.entities.get(&id) {
            entity.entity_type == EntityType::Player
        } else {
            false
        }
    }

    pub fn get_encounter_entity(&mut self, entity: &Entity) -> &mut EncounterEntity {

        self.encounter
            .entities
            .entry(entity.name.clone())
            .or_insert_with(|| entity.into())
    }

    pub fn get_or_create_encounter_entity(&mut self, instance_id: u64) -> Option<&mut EncounterEntity> {

        if let Some(entity) = self.entities.get(&instance_id) {
            let encounter_entity = self.encounter
                .entities
                .entry(entity.name.clone())
                .or_insert_with(|| entity.into());

            return Some(encounter_entity);
        }

        None
    }

    pub fn new_pc(
        &mut self,
        now: DateTime<Utc>,
        player_id: u64,
        name: String,
        class_id: u32,
        max_item_level: f32,
        character_id: u64,
        stat_pairs: Vec<StatPair>,
        equip_item_datas: Vec<EquipItemData>,
        status_effect_datas: Vec<StatusEffectData>
    ) {
        let (hp, max_hp) = get_current_and_max_hp(&stat_pairs);

        let entity = Entity {
            id: player_id,
            entity_type: EntityType::Player,
            name: name.clone(),
            class_id: class_id.into(),
            gear_level: truncate_gear_level(max_item_level), // todo?
            character_id: character_id,
            stats: stat_pairs.iter()
                .map(|sp| (sp.stat_type, sp.value))
                .collect(),
            ..Default::default()
        };

        {
            let entity = self.entities.entry(player_id)
                .and_modify(|pr| {
                    pr.entity_type = entity.entity_type;
                    pr.character_id = character_id;
                    pr.class_id = class_id.into();
                    pr.name = name.clone();
                    pr.stats = stat_pairs.iter()
                        .map(|sp| (sp.stat_type, sp.value))
                        .collect();
                })
                .or_insert_with(|| entity);

            self.encounter
                .entities
                .entry(name.clone())
                .and_modify(|player| {
                    player.id = player_id;
                    player.gear_score = max_item_level;
                    player.current_hp = hp;
                    player.max_hp = max_hp;
                    if character_id > 0 {
                        player.character_id = character_id;
                    }
                })
                .or_insert_with(|| {
                    let entity = entity.clone();
                    let mut player: EncounterEntity = entity.into();
                    player.current_hp = hp;
                    player.max_hp = max_hp;
                    player
                });
        }

        let old_entity_id = self.character_id_to_entity_id.get(&character_id).copied();

        if let Some(old_entity_id) = old_entity_id {
            if let Some(party_id) = self.entity_id_to_party_id.get(&old_entity_id).cloned() {
                self.entity_id_to_party_id.remove(&old_entity_id);
                self.entity_id_to_party_id.insert(player_id, party_id);
            }
        }

        self.character_id_to_entity_id.insert(character_id, player_id);
        self.entity_id_to_character_id.insert(player_id, character_id);

        self.complete_entry(character_id, player_id);
 
        let local_character_id = if self.local_character_id != 0 {
            self.local_character_id
        } else {
            self.entity_id_to_character_id
                .get(&self.local_entity_id)
                .copied()
                .unwrap_or_default()
        };

        let use_party_status_effects =
            self.should_use_party_status_effect(character_id, local_character_id);
        if use_party_status_effects {
            self.party_status_effect_registry.remove(&character_id);
        } else {
            self.local_status_effect_registry.remove(&character_id);
        }
        let (target_id, target_type) = if use_party_status_effects {
            (character_id, StatusEffectTargetType::Party)
        } else {
            (player_id, StatusEffectTargetType::Local)
        };

        for sed in status_effect_datas.into_iter() {
            let source_id = sed.source_id;
            let status_effect = build_status_effect(
                sed,
                target_id,
                source_id,
                target_type,
                now);
            self.register_status_effect(status_effect);
        }
    }

    pub fn complete_entry(&mut self, character_id: u64, entity_id: u64) {
        let char_party_id = self.character_id_to_party_id.get(&character_id).cloned();
        let entity_party_id = self.entity_id_to_party_id.get(&entity_id).cloned();
        
        if let (Some(_char_party_id), Some(_entity_party_id)) = (char_party_id, entity_party_id) {
            return;
        }

        if let Some(entity_party_id) = entity_party_id {
            self.character_id_to_party_id.insert(character_id, entity_party_id);
        }

        if let Some(char_party_id) = char_party_id {
            self.entity_id_to_party_id.insert(entity_id, char_party_id);
        }
    }

    pub fn party_status_effect_add(
        &mut self,
        now: DateTime<Utc>,
        character_id: u64,
        status_effect_datas: Vec<StatusEffectData>
    ) -> Vec<StatusEffectDetails> {
        
        let mut shields: Vec<StatusEffectDetails> = Vec::new();
        for sed in status_effect_datas {
            let (entity_id, entity_name) = {
                let entity = self.get_source_entity(sed.source_id);
                (entity.id, entity.name.clone())
            };
            let encounter_entity = self.encounter.entities.get(&entity_name);

            let status_effect = build_status_effect(
                sed,
                character_id,
                entity_id,
                StatusEffectTargetType::Party,
                now,
            );
            
            if status_effect.status_effect_type == StatusEffectType::Shield {
                shields.push(status_effect.clone());
            }

            self.register_status_effect(status_effect);
        }
        shields
    }

    pub fn get_source_entity(&mut self, id: u64) -> &mut Entity {
        let source_id = match self.entities.get(&id) {
            Some(entity) if matches!(entity.entity_type, EntityType::Projectile | EntityType::Summon) => {
                entity.owner_id
            }
            _ => id,
        };
    
        match self.entities.entry(source_id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let entity = Entity {
                    id: source_id,
                    entity_type: EntityType::Unknown,
                    name: format!("{:x}", source_id),
                    ..Default::default()
                };

                entry.insert(entity)
            },
        }
    }

    pub fn new_cast(
        &mut self,
        entity_id: u64,
        skill_id: u32,
        summon_source: Option<Vec<u32>>,
        timestamp: i64,
    ) {
        let relative = timestamp - self.fight_start;
        if let Some(summon_source) = summon_source {
            for source in summon_source {
                if self.skill_timestamp.get(&(entity_id, source)).is_some() {
                    // info!("ignoring summon: {}|{}|{}", entity_id, source, relative);
                    return;
                }
            }
        }

        // info!("new skill CAST: {}|{}|{}", entity_id, skill_id, relative);
        self.skill_timestamp.insert((entity_id, skill_id), relative);
        self.skills.insert(
            (entity_id, skill_id, relative),
            SkillCast {
                hits: Vec::new(),
                timestamp: relative,
                last: relative,
            },
        );
    }

    pub fn on_hit(
        &mut self,
        entity_id: u64,
        projectile_id: u64,
        skill_id: u32,
        info: SkillHit,
        summon_source: Option<Vec<u32>>,
    ) {
        let skill_timestamp = if let Some(summon_source) = summon_source {
            let mut source_timestamp = info.timestamp;
            let mut found = false;
            for source in summon_source {
                if let Some(skill_timestamp) = self.skill_timestamp.get(&(entity_id, source)) {
                    found = true;
                    source_timestamp = skill_timestamp;
                    break;
                }
            }
            if !found {
                self.skill_timestamp
                    .insert((entity_id, skill_id), source_timestamp);
            }
            source_timestamp
        } else if let Some(skill_timestamp) = self.projectile_id_to_timestamp.get(&projectile_id) {
            skill_timestamp
        } else if let Some(skill_timestamp) = self.skill_timestamp.get(&(entity_id, skill_id)) {
            skill_timestamp
        } else {
            -1
        };

        if skill_timestamp >= 0 {
            // info!(
            //     "new skill HIT: {}|{}|{}|{}",
            //     entity_id, projectile_id, skill_id, skill_timestamp
            // );
            let timestamp = info.timestamp;
            self.skills
                .entry((entity_id, skill_id, skill_timestamp))
                .and_modify(|skill| {
                    skill.hits.push(info.clone());
                    skill.last = timestamp;
                })
                .or_insert(SkillCast {
                    hits: vec![info],
                    timestamp: skill_timestamp,
                    last: timestamp,
                });
        }
    }

    pub fn get_cast_log(&mut self) -> HashMap<u64, HashMap<u32, BTreeMap<i64, SkillCast>>> {
        let mut cast_log: HashMap<u64, HashMap<u32, BTreeMap<i64, SkillCast>>> = HashMap::new();
        for ((entity_id, skill_id, timestamp), cast) in self.skills.iter() {
            cast_log
                .entry(*entity_id)
                .or_default()
                .entry(*skill_id)
                .or_default()
                .insert(*timestamp, cast.clone());
        }

        cast_log
    }

    // keep all player entities, reset all stats
    pub fn soft_reset(&mut self, keep_bosses: bool) {
        let clone = self.encounter.clone();

        self.encounter.fight_start = 0;
        self.encounter.boss_only_damage = self.boss_only_damage;
        self.encounter.entities = HashMap::new();
        self.encounter.current_boss_name = "".to_string();
        self.encounter.encounter_damage_stats = Default::default();
        self.prev_stagger = 0;
        self.raid_clear = false;

        self.damage_log = HashMap::new();
        self.identity_log = HashMap::new();
        self.cast_log = HashMap::new();
        self.boss_hp_log = HashMap::new();
        self.stagger_log = Vec::new();
        self.stagger_intervals = Vec::new();
        self.party_info = Vec::new();

        self.ntp_fight_start = 0;

        self.rdps_valid = false;

        self.custom_id_map = HashMap::new();

        for (key, entity) in clone.entities.into_iter().filter(|(_, e)| {
            e.entity_type == EntityType::Player
                || (keep_bosses && e.entity_type == EntityType::Boss)
        }) {
            self.encounter.entities.insert(
                key,
                EncounterEntity {
                    name: entity.name,
                    id: entity.id,
                    character_id: entity.character_id,
                    npc_id: entity.npc_id,
                    class: entity.class,
                    class_id: entity.class_id,
                    entity_type: entity.entity_type,
                    gear_score: entity.gear_score,
                    max_hp: entity.max_hp,
                    current_hp: entity.current_hp,
                    is_dead: entity.is_dead,
                    ..Default::default()
                },
            );
        }
    }

    pub fn get_party(&self) -> Vec<Vec<String>> {
        let entity_id_to_party_id = &self.entity_id_to_party_id;
        let entities = &self.entities;
        let mut party_info: HashMap<u32, Vec<String>> = HashMap::new();

        for (entity_id, party_id) in entity_id_to_party_id.iter() {
            let entity_name = entities.get(entity_id).map(|entity| entity.name.clone());
            party_info.entry(*party_id)
                .or_insert_with(Vec::new)
                .extend(entity_name);
        }
        
        let mut sorted_parties = party_info.into_iter().collect::<Vec<(u32, Vec<String>)>>();
        sorted_parties.sort_by_key(|&(party_id, _)| party_id);

        sorted_parties
            .into_iter()
            .map(|(_, members)| members)
            .collect()
    }

    pub fn send_raid_info<SA: StatsApi>(&self, stats_api: Arc<Mutex<SA>>) {

        let encounter = &self.encounter;
        let boss_name = encounter.current_boss_name.clone();
        let raid_name = if let Some(boss) = encounter.entities.get(&boss_name) {
            boss_to_raid_map(&boss_name, boss.max_hp)
        } else {
            return;
        };

        if !is_valid_raid(&raid_name) {
            info!("not valid for raid info");
            return
        }

        let players: Vec<String> = encounter
            .entities
            .iter()
            .filter_map(|(_, e)| {
                if e.entity_type == EntityType::Player {
                    Some(e.name.clone())
                } else {
                    None
                }
            })
            .collect();

        if players.len() > 16 {
            return;
        }

        let difficulty = self.raid_difficulty.clone();
        let is_cleared = self.raid_clear;

        tokio::task::spawn(async move {
            let payload = SendRaidInfo {
                players,
                raid_name: &raid_name,
                difficulty: &difficulty,
                is_cleared
            };
            stats_api.lock().await.send_raid_info(payload).await;
        });
    }

    pub fn on_phase_transition<EE : EventEmitter, ES: EncounterService, SA: StatsApi>(
        &mut self,
        version: &str,
        client_id: Option<Uuid>,
        phase_code: i32,
        stats_api: Arc<Mutex<SA>>,
        encounter_service: Arc<ES>,
        event_emitter: Arc<EE>
    ) {
        event_emitter
            .emit("phase-transition", phase_code)
            .expect("failed to emit phase-transition");

        if matches!(phase_code, 0 | 2 | 3 | 4) && !self.encounter.current_boss_name.is_empty() {
           
            self.send_raid_info(stats_api.clone());
            
            if phase_code == 0 {
                self.is_valid_zone = false;
            }

            self.save_to_db(
                version,
                client_id,
                stats_api,
                false,
                encounter_service,
                event_emitter);
            self.saved = true;
        }

        self.resetting = true;
    }

    // replace local player
    pub fn on_init_pc(&mut self,
        now: DateTime<Utc>,
        player_id: u64,
        class_id: u32,
        character_id: u64,
        name: String,
        gear_level: f32,
        stat_pairs: Vec<StatPair>,
        status_effect_datas: Vec<StatusEffectData>) {
        let (hp, max_hp) = get_current_and_max_hp(&stat_pairs);

        let player = Entity {
            id: player_id,
            is_local_player: true,
            entity_type: EntityType::Player,
            name: name.clone(),
            class_id: class_id.into(),
            gear_level: truncate_gear_level(gear_level),
            character_id,
            stats: stat_pairs
                .into_iter()
                .map(|sp| (sp.stat_type, sp.value))
                .collect(),
            ..Default::default()
        };

        self.local_entity_id = player_id;
        self.local_character_id = character_id;
        self.character_id_to_entity_id.insert(character_id, player_id);
        self.entity_id_to_character_id.insert(player_id, character_id);
        self.local_player_name = Some(player.name.clone());
        self.complete_entry(character_id, player_id);

        self.local_status_effect_registry.remove(&player_id);

        {
            self.entities.clear();
            let player = self.entities.entry(player.id).or_insert_with(|| player);
            info!("{player}");

            self.encounter.entities.remove(&self.encounter.local_player);
            self.encounter.local_player.clone_from(&name);
            let mut encounter_entity = encounter_entity_from_entity(&player);
            encounter_entity.current_hp = hp;
            encounter_entity.max_hp = max_hp;
            self.encounter.entities.insert(name.clone(), encounter_entity);
        }

        for sed in status_effect_datas {
            let source_id = self.get_source_entity(sed.source_id).id;

            let status_effect = build_status_effect(
                sed.clone(),
                player_id,
                source_id,
                StatusEffectTargetType::Local,
                now,
            );
    
            self.register_status_effect(status_effect);
        }
    }

    pub fn on_new_trap(
        &mut self,
        object_id: u64,
        owner_id: u64,
        skill_id: u32,
        skill_effect: u32,
    ) {
        let trap: Entity = Entity {
            id: object_id,
            entity_type: EntityType::Projectile,
            name: format!("{:x}", object_id),
            owner_id: owner_id,
            skill_id: skill_id,
            skill_effect_id: skill_effect,
            ..Default::default()
        };
        self.entities.insert(trap.id, trap);
        let is_player = self.id_is_player(owner_id);

        if is_player && skill_id > 0
        {
            let key = (owner_id, skill_id);
            if let Some(timestamp) = self.skill_timestamp.get(&key) {
                self.projectile_id_to_timestamp.insert(object_id, timestamp);
            }
        }
    }

    pub fn on_new_projectile(
        &mut self,
        projectile_id: u64,
        owner_id: u64,
        skill_id: u32,
        skill_effect: u32,
    ) {
        let projectile = Entity {
            id: projectile_id,
            entity_type: EntityType::Projectile,
            name: format!("{:x}", projectile_id),
            owner_id,
            skill_id,
            skill_effect_id: skill_effect,
            ..Default::default()
        };
        self.entities.insert(projectile.id, projectile);
        let is_player = self.id_is_player(owner_id);

        if is_player && skill_id > 0
        {
            let key = (owner_id, skill_id);
            if let Some(timestamp) = self.skill_timestamp.get(&key) {
                self.projectile_id_to_timestamp.insert(projectile_id, timestamp);
            }
        }
    }

    pub fn on_identity_gain(
        &mut self,
        now: DateTime<Utc>,
        player_id: u64,
        identity: &Identity
    ) {

        if self.encounter.local_player.is_empty() {
            if let Some((_, entity)) = self
                .encounter
                .entities
                .iter()
                .find(|(_, e)| e.id == player_id)
            {
                self.encounter.local_player.clone_from(&entity.name);
            } else {
                return;
            }
        }

        if let Some(entity) = self
            .encounter
            .entities
            .get_mut(&self.encounter.local_player)
        {
            let entry = (
                now.timestamp_millis(),
                (
                    identity.gauge1,
                    identity.gauge2,
                    identity.gauge3,
                ),
            );
            self.identity_log
                .entry(entity.name.clone())
                .or_default()
                .push(entry);
        }
    }

    pub fn on_boss_shield(&mut self, target_entity: &Entity, shield: u64) {

        // let dump = self.encounter
        //     .entities
        //     .get(&target_entity.name);
        // println!("{} {:?}", self.encounter.current_boss_name, dump);

        if target_entity.entity_type == EntityType::Boss
            && target_entity.name == self.encounter.current_boss_name
        {
            self.encounter
                .entities
                .entry(target_entity.name.clone())
                .and_modify(|e| {
                    e.current_shield = shield;
                });
        }
    }

}