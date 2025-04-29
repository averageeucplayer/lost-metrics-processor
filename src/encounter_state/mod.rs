mod on_damage;
mod on_cc_applied;
mod on_cc_removed;
mod on_shield_applied;
mod on_new_npc;
mod on_shield_used;
mod on_skill_start;

use chrono::{Date, DateTime, Utc};
use hashbrown::hash_map::Entry;
use hashbrown::{HashMap, HashSet};
use log::{info, warn};
use lost_metrics_core::models::*;
use lost_metrics_misc::{boss_to_raid_map, get_main_boss_from_minion, is_battle_item};
use lost_metrics_sniffer_stub::packets::definitions::{PKTIdentityGaugeChangeNotify, PKTPartyInfo, PKTPartyInfoInner, PKTPartyStatusEffectAddNotify};
use lost_metrics_sniffer_stub::packets::structures::{EquipItemData, StatPair, StatusEffectData};
use lost_metrics_store::encounter_service::EncounterService;
use moka::sync::Cache;
use rsntp::SntpClient;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use uuid::Uuid;
use std::cell::{Ref, RefCell};
use std::collections::BTreeMap;
use std::default::Default;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use crate::utils::*;
use crate::models::{CompleteEncounter, IncompleteEncounter};
use super::abstractions::*;

pub type StatusEffectRegistry = HashMap<u32, StatusEffectDetails>;

pub enum Status {

}

#[derive(Debug)]
pub struct EncounterState {
    pub id: Uuid,
    pub entities_by_id: HashMap<u64, Rc<RefCell<Entity>>>,
    pub entities_by_character_id: HashMap<u64, Rc<RefCell<Entity>>>,
    pub parties_by_id: HashMap<u32, Vec<u64>>,
    pub started_on: DateTime<Utc>,
    pub updated_on: DateTime<Utc>,
    pub skills: HashMap<(u64, u32, i64), SkillCast>,
    pub projectile_id_to_timestamp: Cache<u64, i64>,
    pub skill_timestamp: Cache<(u64, u32), i64>,
    pub local_status_effect_registry: HashMap<u64, StatusEffectRegistry>,
    pub party_status_effect_registry: HashMap<u64, StatusEffectRegistry>,
    pub local_player_name: Option<String>,
    pub local_entity_id: u64,
    pub local_character_id: u64,

    pub client_id: Option<Uuid>,
    pub party_freeze: bool,
    pub party_cache: Option<Vec<Vec<String>>>,
    pub party_map_cache: HashMap<i32, Vec<String>>,
    pub entity_stats: HashMap<u64, EncounterEntity>,
    pub current_boss: Option<Rc<RefCell<Entity>>>,
    pub encounter_damage_stats: EncounterDamageStats,
    pub duration: i64,
    pub difficulty: Option<String>,
    pub is_cleared: bool,
    pub sync: Option<String>,

    pub resetting: bool,
    pub boss_dead_update: bool,
    pub saved: bool,
    pub raid_end_cd: DateTime<Utc>,
    pub raid_clear: bool,
    pub is_valid_zone: bool,
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
            id: Uuid::nil(),
            entities_by_id: HashMap::new(),
            entities_by_character_id: HashMap::new(),
            parties_by_id: HashMap::new(),
            started_on: DateTime::<Utc>::MIN_UTC,
            updated_on: DateTime::<Utc>::MIN_UTC,
            skills: HashMap::new(),
            projectile_id_to_timestamp: Cache::builder()
                .time_to_idle(Duration::from_secs(20))
                .build(),
            skill_timestamp: Cache::builder()
                .time_to_idle(Duration::from_secs(20))
                .build(),
            local_status_effect_registry: HashMap::new(),
            party_status_effect_registry: HashMap::new(),
            local_player_name: None,
            local_entity_id: 0,
            local_character_id: 0,
            raid_end_cd: DateTime::<Utc>::MIN_UTC,
            client_id: None,
            party_freeze: false,
            party_cache: None,
            party_map_cache: HashMap::new(),
            // encounter: Encounter::default(),
            is_cleared: false,
            current_boss: None,
            difficulty: None,
            duration: 0,
            encounter_damage_stats: EncounterDamageStats::default(),
            entity_stats: HashMap::new(),
            sync: None,

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
        let mut unknown_local = self.entities_by_id.get(&self.local_entity_id)
            .map(|p| p.borrow().name.is_empty())
            .unwrap_or(true);

        let party_id = party_instance_id;
        self.parties_by_id.remove(&party_id);

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

        let party = self.parties_by_id.entry(party_instance_id).or_default();

        for member in party_member_datas {
            if unknown_local && member.name == most_likely_local_name {
                if let Some(mut local_player) = self.entities_by_id.get(&self.local_entity_id).map(|pr| pr.borrow_mut()) {
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

                    self.local_player_name = Some(member.name.clone());
                }
            }

            if let Some(entity) = self.entities_by_character_id.get_mut(&member.character_id) {
                let mut entity = entity.borrow_mut();
                if entity.entity_type == EntityType::Player && entity.name == member.name {
                    entity.gear_level = truncate_gear_level(member.gear_level);
                    entity.class_id = member.class_id.into();
                    entity.party_instance_id = party_instance_id;
                }
            }

            party.push(member.character_id);
        }

        let local_player_id = self.local_entity_id;

        if let Some(entity) = self.entities_by_id.get(&local_player_id).map(|pr| pr.borrow()) {
            let entities = &mut self.entity_stats;

            // we replace the existing local player if it exists, since its name might have changed (from hex or "You" to character name)
            if let Some(mut local) = entities.remove(&self.local_entity_id) {
                // update local player name, insert back into encounter
                self.local_player_name = Some(entity.name.clone());
                
                local.update(&entity);
                local.class = entity.class_id.as_ref().to_string();
    
                entities.insert(self.local_entity_id, local);
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
                    self.local_player_name = Some(entity.name.clone());
                    entities.insert(self.local_entity_id, new_local);
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
        let source_id = status_effect_data.source_id;
        let source_entity_id = self.get_source_entity(source_id).borrow().id;
        let mut status_effect = build_status_effect(
            status_effect_data,
            target_id,
            source_entity_id,
            StatusEffectTargetType::Local,
            now,
        );

        let target = self.get_source_entity(target_id);
        let target = target.borrow();

        if status_effect.status_effect_type == StatusEffectType::Shield {
            self.on_boss_shield(target_id, status_effect.value);
            self.on_shield_applied(
                source_entity_id,
                target_id,
                status_effect.status_effect_id,
                status_effect.value,
            );
        }

        let source_encounter_entity = self.entity_stats.get(&source_id);
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
        
        if character_id == local_character_id {
            return false;
        }
        
        let local_player = self.entities_by_character_id.get(&local_character_id);
        let affected_player = self.entities_by_character_id.get(&character_id);
        
        if let Some((local_player, affected_player)) = local_player.zip(affected_player) {
            return local_player.borrow().party_instance_id == affected_player.borrow().party_instance_id
        }

        false
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
        let target_entity_id = self.entities_by_character_id.get(&character_id).map(|pr| pr.borrow().id).unwrap_or_default();
        let shields = self.party_status_effect_add(now, character_id, status_effect_datas);

        for status_effect in shields {
           
            let target_id =
                if status_effect.target_type == StatusEffectTargetType::Party {
                    target_entity_id
                } else {
                    status_effect.target_id
                };
            
            if self.current_boss.as_ref().filter(|pr| pr.borrow().id == target_id).is_some() {
                self.entity_stats
                    .entry(target_entity_id)
                    .and_modify(|e| {
                        e.current_shield = status_effect.value;
                    });
            }

            let target = self.get_source_entity(target_id);
            let target = target.borrow();
            let source = self.get_source_entity(status_effect.source_id);
            let source = source.borrow();

            let both_players = source.entity_type == target.entity_type && target.entity_type == EntityType::Player;

            if both_players {
                self.on_shield_applied(
                    status_effect.source_id,
                    target_id,
                    status_effect.status_effect_id,
                    status_effect.value,
                );
            }
        }
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
                let party_instance_id = self.entities_by_id
                    .get(&sed.source_id)
                    .map(|pr| pr.borrow().party_instance_id)
                    .unwrap_or_default();

                sed.is_valid_for_raid()
                    || *party_id == party_instance_id
            })
            .cloned()
            .collect()
    }

    pub fn get_or_create_entity(&mut self, id: u64) -> Rc<RefCell<Entity>> {
        match self.entities_by_id.entry(id) {
            Entry::Occupied(e) => e.get().clone(),
            Entry::Vacant(e) => {
                let entity = Entity {
                    id,
                    created_on: Utc::now(),
                    entity_type: EntityType::Unknown,
                    name: format!("{:x}", id),
                    ..Default::default()
                };
                
                let entity = Rc::new(RefCell::new(entity));
                e.insert(entity).clone()
            }
        }
    }

    pub fn get_status_effects(
        &mut self,
        now: DateTime<Utc>,
        source_entity: &Entity,
        target_entity: &Entity,
        local_character_id: u64,
    ) -> (Vec<StatusEffectDetails>, Vec<StatusEffectDetails>) {

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
            self.actually_get_status_effects(source_id, source_type, now);

        let use_party_for_target = if source_entity.entity_type == EntityType::Player {
            self.should_use_party_status_effect(target_entity.character_id, local_character_id)
        } else {
            false
        };

        let source_party_id = (source_entity.party_instance_id != 0).then(|| source_entity.party_instance_id);

        let mut status_effects_on_target = match (use_party_for_target, source_party_id) {
            (true, Some(source_party_id)) => self.get_status_effects_from_party(
                target_entity.character_id,
                StatusEffectTargetType::Party,
                &source_party_id,
                now,
            ),
            (false, Some(source_party_id)) => self.get_status_effects_from_party(
                target_entity.id,
                StatusEffectTargetType::Local,
                &source_party_id,
                now,
            ),
            (true, None) => self.actually_get_status_effects(
                target_entity.character_id,
                StatusEffectTargetType::Party,
                now,
            ),
            (false, None) => self.actually_get_status_effects(
                target_entity.id,
                StatusEffectTargetType::Local,
                now,
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

    pub fn is_owner_a_player(&mut self, id: u64) -> bool {
        if let Some(entity) = self.entities_by_id.get(&id) {
            entity.borrow().entity_type == EntityType::Player
        } else {
            false
        }
    }

    pub fn get_encounter_entity(&mut self, entity_id: u64) -> &mut EncounterEntity {

        self.entity_stats
            .entry(entity_id)
            .or_default()
    }

    pub fn get_or_create_encounter_entity(&mut self, instance_id: u64) -> Option<&mut EncounterEntity> {

        if let Some(entity) = self.entities_by_id.get(&instance_id) {
            let encounter_entity = self.entity_stats
                .entry(instance_id)
                .or_default();

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
            created_on: Utc::now(),
            entity_type: EntityType::Player,
            name: name.clone(),
            class_id: class_id.into(),
            gear_level: truncate_gear_level(max_item_level), // todo?
            character_id,
            stats: stat_pairs.iter()
                .map(|sp| (sp.stat_type, sp.value))
                .collect(),
            ..Default::default()
        };

        {       
            let entity = Rc::new(RefCell::new(entity));
            let old_entity = self.entities_by_character_id.insert(character_id, entity.clone());

            if let Some(old_entity) = old_entity {
                let old_entity = old_entity.borrow();
                entity.borrow_mut().party_instance_id = 0;
            }

           
            self.entities_by_id.insert(player_id, entity.clone());

            self.entity_stats
                .entry(player_id)
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
                    let mut stats = EncounterEntity::default();
                    stats.current_hp = hp;
                    stats.max_hp = max_hp;
                    stats
                });
        }
 
        let use_party_status_effects =
            self.should_use_party_status_effect(character_id, self.local_character_id);
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

    pub fn party_status_effect_add(
        &mut self,
        now: DateTime<Utc>,
        character_id: u64,
        status_effect_datas: Vec<StatusEffectData>
    ) -> Vec<StatusEffectDetails> {
        
        let mut shields: Vec<StatusEffectDetails> = Vec::new();
        for sed in status_effect_datas {
            let entity_id = self.get_source_entity(sed.source_id).borrow().id;
            let encounter_entity = self.entity_stats.get(&entity_id);

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

    pub fn get_source_entity(&mut self, id: u64) -> Rc<RefCell<Entity>> {
        match self.entities_by_id.get(&id).cloned() {
            Some(entity) => {
                // entity.owner_id
                // if matches!(entity.entity_type, EntityType::Projectile | EntityType::Summon)

                {
                    let entity = entity.borrow();

                    if matches!(entity.entity_type, EntityType::Projectile | EntityType::Summon) {
                        let owner = self.entities_by_id.get(&entity.owner_id);

                        match owner {
                            Some(owner) => return owner.clone(),
                            None => {
                                let entity = Entity {
                                    id,
                                    created_on: Utc::now(),
                                    entity_type: EntityType::Unknown,
                                    name: format!("{:x}", id),
                                    ..Default::default()
                                };
                
                                let entity = Rc::new(RefCell::new(entity));
                                self.entities_by_id.insert(id, entity.clone());
                                
                                return entity.clone()
                            },
                        }
                    }
                }

                entity.clone()
            }
            _ => {
                let entity = Entity {
                    id,
                    created_on: Utc::now(),
                    entity_type: EntityType::Unknown,
                    name: format!("{:x}", id),
                    ..Default::default()
                };

                let entity = Rc::new(RefCell::new(entity));
                self.entities_by_id.insert(id, entity.clone());
                
                entity.clone()
            },
        }
    
        // match self.entities_by_id.entry(source_id) {
        //     Entry::Occupied(entry) => entry.into_mut(),
        //     Entry::Vacant(entry) => {
        //         let entity = Entity {
        //             id: source_id,
        //             entity_type: EntityType::Unknown,
        //             name: format!("{:x}", source_id),
        //             ..Default::default()
        //         };

        //         entry.insert(entity)
        //     },
        // }
    }

    pub fn new_cast(
        &mut self,
        entity_id: u64,
        skill_id: u32,
        summon_source: Option<Vec<u32>>,
        now: DateTime<Utc>,
    ) {
        let relative = now - self.started_on;
        let relative = relative.num_milliseconds();

        if let Some(summon_source) = summon_source {
            for source in summon_source {
                if self.skill_timestamp.get(&(entity_id, source)).is_some() {
                    // info!("ignoring summon: {}|{}|{}", entity_id, source, relative);
                    return;
                }
            }
        }

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
        summon_sources: Vec<u32>,
    ) {
        if let Some(skill_timestamp) = self.find_skill_timestamp(entity_id, projectile_id, skill_id, &summon_sources, info.timestamp) {
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

    fn find_skill_timestamp(
        &mut self,
        entity_id: u64,
        projectile_id: u64,
        skill_id: u32,
        summon_sources: &[u32],
        fallback_timestamp: i64,
    ) -> Option<i64> {
        if !summon_sources.is_empty() {
            for &source in summon_sources {
                if let Some(source_timestamp) = self.skill_timestamp.get(&(entity_id, source)) {
                    return Some(source_timestamp);
                }
            }

            self.skill_timestamp.insert((entity_id, skill_id), fallback_timestamp);
            return Some(fallback_timestamp);
        }
    
        if let Some(timestamp) = self.projectile_id_to_timestamp.get(&projectile_id) {
            return Some(timestamp);
        }
    
        if let Some(timestamp) = self.skill_timestamp.get(&(entity_id, skill_id)) {
            return Some(timestamp);
        }
    
        None
    }

    pub fn get_cast_log(&self) -> HashMap<u64, HashMap<u32, BTreeMap<i64, SkillCast>>> {
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
        let entity_stats = self.entity_stats.clone();

        self.started_on = DateTime::<Utc>::MIN_UTC;
        self.boss_only_damage = self.boss_only_damage;
        self.current_boss = None;
        self.encounter_damage_stats = Default::default();
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

        for (key, entity) in entity_stats.into_iter().filter(|(_, e)| {
            e.entity_type == EntityType::Player
                || (keep_bosses && e.entity_type == EntityType::Boss)
        }) {
            self.entity_stats.insert(
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
        let entities: &HashMap<u64, Rc<RefCell<Entity>>> = &self.entities_by_character_id;
        let mut party_info: HashMap<u32, Vec<String>> = HashMap::new();

        for (party_id, character_ids) in self.parties_by_id.iter() {
            let party = party_info.entry(*party_id).or_insert_with(Vec::new);

            for character_id in character_ids {
                let entity_name = entities.get(character_id).map(|entity| entity.borrow().name.clone());
                party.extend(entity_name);
            }
            
            
        }
        
        let mut sorted_parties = party_info.into_iter().collect::<Vec<(u32, Vec<String>)>>();
        sorted_parties.sort_by_key(|&(party_id, _)| party_id);

        sorted_parties
            .into_iter()
            .map(|(_, members)| members)
            .collect()
    }

    // pub fn send_raid_info<SA: StatsApi>(&self, stats_api: Arc<Mutex<SA>>) {

    //     let encounter = &self.encounter;
    //     let boss_name = encounter.current_boss_name.clone();
    //     let raid_name = if let Some(boss) = encounter.entities.get(&boss_name) {
    //         boss_to_raid_map(&boss_name, boss.max_hp)
    //     } else {
    //         return;
    //     };

    //     if !is_valid_raid(&raid_name) {
    //         info!("not valid for raid info");
    //         return
    //     }

    //     let players: Vec<String> = encounter
    //         .entities
    //         .iter()
    //         .filter_map(|(_, e)| {
    //             if e.entity_type == EntityType::Player {
    //                 Some(e.name.clone())
    //             } else {
    //                 None
    //             }
    //         })
    //         .collect();

    //     if players.len() > 16 {
    //         return;
    //     }

    //     let difficulty = self.raid_difficulty.clone();
    //     let is_cleared = self.raid_clear;

    //     tokio::task::spawn(async move {
    //         let payload = SendRaidInfo {
    //             players,
    //             raid_name: &raid_name,
    //             difficulty: &difficulty,
    //             is_cleared
    //         };
    //         stats_api.lock().await.send_raid_info(payload).await;
    //     });
    // }

    // pub fn on_phase_transition<EE : EventEmitter, PE: Persister, SA: StatsApi>(
    //     &mut self,
    //     version: &str,
    //     client_id: Option<Uuid>,
    //     phase_code: i32,
    //     stats_api: Arc<Mutex<SA>>,
    //     encounter_service: Arc<ES>,
    //     event_emitter: Arc<EE>
    // ) {
        
    // }

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
            created_on: Utc::now(),
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
        self.local_player_name = Some(player.name.clone());

        self.local_status_effect_registry.remove(&player_id);

        {
            self.entities_by_id.clear();
            self.entities_by_character_id.clear();
            let entity_rc = Rc::new(RefCell::new(player));
            self.entities_by_id.insert(player_id, entity_rc.clone());;
            self.entities_by_character_id.insert(player_id, entity_rc.clone());
            self.entity_stats.remove(&player_id);
            let mut encounter_entity = EncounterEntity::default();
            encounter_entity.current_hp = hp;
            encounter_entity.max_hp = max_hp;
            self.entity_stats.insert(player_id, encounter_entity);
        }

        for sed in status_effect_datas {
            let source_id = self.get_source_entity(sed.source_id).borrow().id;

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
        skill_effect_id: u32,
    ) {
        let mut entity: Entity = Entity {
            id: object_id,
            created_on: Utc::now(),
            entity_type: EntityType::Projectile,
            name: format!("{:x}", object_id),
            owner_id,
            skill_id,
            skill_effect_id,
            ..Default::default()
        };

        if is_battle_item(&skill_effect_id, "attack")
        {
            entity.is_battle_item = true;
        }

        let entity = Rc::new(RefCell::new(entity));
        self.entities_by_id.insert(object_id, entity);
        let is_player = self.is_owner_a_player(owner_id);

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
        skill_effect_id: u32,
    ) {
        let mut entity = Entity {
            id: projectile_id,
            created_on: Utc::now(),
            entity_type: EntityType::Projectile,
            name: format!("{:x}", projectile_id),
            owner_id,
            skill_id,
            skill_effect_id,
            ..Default::default()
        };

        if is_battle_item(&skill_effect_id, "attack")
        {
            entity.is_battle_item = true;
        }

        let entity = Rc::new(RefCell::new(entity));
        self.entities_by_id.insert(projectile_id, entity);
        let is_player = self.is_owner_a_player(owner_id);

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

        if self.local_player_name.is_none() {
            if let Some((_, entity)) = self
                .entity_stats
                .iter()
                .find(|(_, e)| e.id == player_id)
            {
                let local_name = self.entities_by_id
                    .get(&self.local_entity_id)
                    .map(|pr| pr.borrow().name.clone())
                    .unwrap_or_default();
                self.local_player_name = Some(local_name);
            } else {
                return;
            }
        }

        if let Some(entity) = self.entity_stats.get_mut(&self.local_entity_id)
        {
            let entity_name = self.local_player_name.as_ref().unwrap().to_string();
            let entry = (
                now.timestamp_millis(),
                (
                    identity.gauge1,
                    identity.gauge2,
                    identity.gauge3,
                ),
            );
            self.identity_log
                .entry(entity_name)
                .or_default()
                .push(entry);
        }
    }

    pub fn on_boss_shield(&mut self, target_id: u64, shield: u64) {

        if self.current_boss.as_ref().filter(|pr| pr.borrow().id == target_id).is_some() {
            self.entity_stats
                .entry(target_id)
                .and_modify(|e| {
                    e.current_shield = shield;
                });
        }
    }

    pub fn get_complete_encounter(&self) -> Option<CompleteEncounter> {
        
        let current_boss = match self.current_boss.as_ref() {
            Some(boss) => boss.borrow(),
            None => return None,
        };

        if self.started_on == DateTime::<Utc>::MIN_UTC {
            return None;
        }

        let boss_stats = match self.entity_stats.get(&current_boss.id) {
            Some(boss) => boss,
            None => return None,
        };

        if boss_stats.current_hp == boss_stats.max_hp {
            return None;
        }

        let main_boss = get_main_boss_from_minion(&current_boss.name);
        let skill_cast_log = self.get_cast_log();
        let party_info = self.get_party();
        let duration = self.updated_on - self.started_on;

        let summary = CompleteEncounter {
            id: self.id,
            party_info,
            is_raid_clear: self.is_cleared,
            boss_name: main_boss.to_string(),
            prev_stagger: self.prev_stagger,
            damage_log: self.damage_log.clone(),
            identity_log: self.identity_log.clone(),
            cast_log: self.cast_log.clone(),
            boss_hp_log: self.boss_hp_log.clone(),
            stagger_log: self.stagger_log.clone(),
            stagger_intervals: self.stagger_intervals.clone(),
            raid_difficulty: self.raid_difficulty.clone(),
            region: self.region.clone(),
            ntp_fight_start: self.ntp_fight_start,
            rdps_valid: self.rdps_valid,
            manual: false,
            skill_cast_log: skill_cast_log,
            started_on: self.started_on,
            updated_on: self.updated_on,
            local_player_name: self.local_player_name.clone().unwrap_or_default(),
            entities: HashMap::new(),
            current_boss: current_boss.clone(),
            current_boss_stats: boss_stats.clone(),
            encounter_damage_stats: self.encounter_damage_stats.clone(),
            duration,
            boss_only_damage: self.boss_only_damage,
            sync: self.sync.clone()
        };

        Some(summary)
    }

    pub fn get_encounter(&self) -> Option<IncompleteEncounter> {

        if self.current_boss.is_none() {
            return None
        }

        None
    }

    pub fn get_raid_info(&mut self) -> Option<SendRaidInfo> {

        let current_boss = match self.current_boss.as_ref() {
            Some(boss) => boss,
            None => return None,
        };
        let current_boss_id = current_boss.borrow().id;

        let entity = self.entities_by_id.get(&current_boss_id);
        let stats = self.entity_stats.get(&current_boss_id);
        let mut raid_name = None;

        if let Some((entity, stats)) = entity.zip(stats) {
            raid_name = boss_to_raid_map(&entity.borrow().name, stats.max_hp);
        }
        else {
            return None;
        }

        let raid_name = match raid_name.filter(|pr| is_valid_raid(pr)) {
            Some(raid_name) => raid_name,
            None => return None,
        };

        let players = self.get_player_names();

        if players.len() > 16 {
            return None
        }

        let cleared = self.raid_clear;

        let info = SendRaidInfo {
            raid_name: raid_name,
            difficulty: &self.raid_difficulty,
            players,
            is_cleared: self.is_cleared,
        };

        Some(info)
    }

    pub fn on_party_status_effect_result(
        &mut self,
        raid_instance_id: u32,
        party_instance_id: u32,
        character_id: u64
    ) {

    }

    pub fn get_active_player_names(&self) -> Vec<String> {
        self.entity_stats
            .values()
            .filter_map(|pr| pr.is_valid_player().then(|| pr.name.to_string()))
            .collect()
    }

    pub fn get_player_names(&self) -> Vec<String> {
        self.entities_by_id
            .iter()
            .filter_map(|(_, e)| {
                let entity = e.borrow();
                if entity.entity_type == EntityType::Player {
                    Some(entity.name.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}