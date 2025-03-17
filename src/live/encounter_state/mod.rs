mod on_damage;
mod on_abnormal_move;
mod save_to_db;
mod on_cc_applied;
mod on_death;
mod on_cc_removed;
mod on_shield_applied;
mod on_init_env;
mod on_new_npc;
mod on_shield_used;
mod on_skill_start;
mod update_local_player;

use chrono::Utc;
use hashbrown::HashMap;
use lost_metrics_core::models::*;
use lost_metrics_misc::get_class_from_id;
use lost_metrics_sniffer_stub::packets::definitions::PKTIdentityGaugeChangeNotify;
use rsntp::SntpClient;
use tokio::runtime::Handle;
use tokio::sync::Mutex;
use uuid::Uuid;
use std::cell::RefCell;
use std::default::Default;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Instant;
use crate::live::entity_tracker::EntityTracker;
use crate::live::skill_tracker::SkillTracker;
use crate::live::utils::*;

use super::abstractions::repository::Repository;
use super::abstractions::EventEmitter;
use super::party_tracker::PartyTracker;
use super::stats_api::StatsApi;
use super::trackers::Trackers;

#[derive(Debug)]
pub struct EncounterState {
    trackers: Rc<RefCell<Trackers>>,
    pub client_id: Option<Uuid>,
    pub party_freeze: bool,
    pub party_cache: Option<Vec<Vec<String>>>,
    pub party_map_cache: HashMap<i32, Vec<String>>,
    pub version: String,
    pub encounter: Encounter,
    pub resetting: bool,
    pub boss_dead_update: bool,
    pub saved: bool,
    pub raid_end_cd: Instant,
    pub raid_clear: bool,
    pub is_valid_zone: bool,
    pub last_update: Instant,
    pub last_party_update: Instant,
    prev_stagger: i32,

    damage_log: HashMap<String, Vec<(i64, i64)>>,
    identity_log: HashMap<String, IdentityLog>,
    cast_log: HashMap<String, HashMap<u32, Vec<i32>>>,

    boss_hp_log: HashMap<String, Vec<BossHpLog>>,

    stagger_log: Vec<(i32, f32)>,
    stagger_intervals: Vec<(i32, i32)>,

    pub party_info: Vec<Vec<String>>,
    pub raid_difficulty: String,
    pub raid_difficulty_id: u32,
    pub boss_only_damage: bool,
    pub region: Option<String>,

    sntp_client: SntpClient,
    ntp_fight_start: i64,

    pub rdps_valid: bool,

    pub skill_tracker: SkillTracker,

    custom_id_map: HashMap<u32, u32>,

    pub damage_is_valid: bool,
}

impl EncounterState {
    pub fn new(
        trackers: Rc<RefCell<Trackers>>,
        version: String) -> EncounterState {
        EncounterState {
            trackers,
            last_party_update: Instant::now(),
            last_update: Instant::now(),
            raid_end_cd: Instant::now(),
            client_id: None,
            party_freeze: false,
            party_cache: None,
            party_map_cache: HashMap::new(),
            version,
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

            // todo
            rdps_valid: false,

            skill_tracker: SkillTracker::new(),

            custom_id_map: HashMap::new(),

            damage_is_valid: true,
        }
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

        self.skill_tracker = SkillTracker::new();

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

    pub fn get_party_from_tracker(&self) -> Vec<Vec<String>> {
        self.trackers.borrow().get_party_from_tracker()
    }

    pub fn on_phase_transition<EE : EventEmitter, RE: Repository, SA: StatsApi>(
        &mut self,
        client_id: Option<Uuid>,
        phase_code: i32,
        stats_api: Arc<Mutex<SA>>,
        repository: Arc<RE>,
        event_emitter: Arc<EE>
    ) {
        event_emitter
            .emit("phase-transition", phase_code)
            .expect("failed to emit phase-transition");

        match phase_code {
            0 | 2 | 3 | 4 => {
                if !self.encounter.current_boss_name.is_empty() {
                    
                    let rt = Handle::current();

                    rt.block_on(async {
                        stats_api.lock().await.send_raid_info(self).await;
                    });
                   
                    if phase_code == 0 {
                        self.is_valid_zone = false;
                    }

                    self.save_to_db(client_id, stats_api, false, repository, event_emitter);
                    self.saved = true;
                }
                self.resetting = true;
            }
            _ => (),
        }
    }

    // replace local player
    pub fn on_init_pc(&mut self, entity: Entity, hp: i64, max_hp: i64) {
        self.encounter.entities.remove(&self.encounter.local_player);
        self.encounter.local_player.clone_from(&entity.name);
        let mut player = encounter_entity_from_entity(&entity);
        player.current_hp = hp;
        player.max_hp = max_hp;
        self.encounter.entities.insert(player.name.clone(), player);
    }

    // add or update player to encounter
    pub fn on_new_pc(&mut self, entity: Entity, hp: i64, max_hp: i64) {
        self.encounter
            .entities
            .entry(entity.name.clone())
            .and_modify(|player| {
                player.id = entity.id;
                player.gear_score = entity.gear_level;
                player.current_hp = hp;
                player.max_hp = max_hp;
                if entity.character_id > 0 {
                    player.character_id = entity.character_id;
                }
            })
            .or_insert_with(|| {
                let mut player = encounter_entity_from_entity(&entity);
                player.current_hp = hp;
                player.max_hp = max_hp;
                player
            });
    }
    
    pub fn on_counterattack(&mut self, source_entity: &Entity) {
        let entity = self
            .encounter
            .entities
            .entry(source_entity.name.clone())
            .or_insert_with(|| {
                let mut entity = encounter_entity_from_entity(source_entity);
                entity
            });
        entity.skill_stats.counters += 1;
    }

    pub fn on_identity_gain(&mut self, pkt: &PKTIdentityGaugeChangeNotify) {
        if self.encounter.fight_start == 0 {
            return;
        }

        if self.encounter.local_player.is_empty() {
            if let Some((_, entity)) = self
                .encounter
                .entities
                .iter()
                .find(|(_, e)| e.id == pkt.player_id)
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
            self.identity_log
                .entry(entity.name.clone())
                .or_default()
                .push((
                    Utc::now().timestamp_millis(),
                    (
                        pkt.identity_gauge1,
                        pkt.identity_gauge2,
                        pkt.identity_gauge3,
                    ),
                ));
        }
    }

    pub fn on_boss_shield(&mut self, target_entity: &Entity, shield: u64) {
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