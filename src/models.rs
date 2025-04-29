use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Utc};
use hashbrown::HashMap;
use lost_metrics_core::models::{BossHpLog, Encounter, EncounterDamageStats, EncounterEntity, Entity, IdentityLog, PlayerStats, SkillCast};
use uuid::Uuid;

pub struct IncompleteEncounter {
    
}

pub struct CompleteEncounter {
    pub id: Uuid,
    pub started_on: DateTime<Utc>,
    pub updated_on: DateTime<Utc>,
    pub boss_name: String,
    pub is_raid_clear: bool,
    pub local_player_name: String,
    pub entities: HashMap<String, EncounterEntity>,
    pub current_boss: Entity,
    pub current_boss_stats: EncounterEntity,
    pub encounter_damage_stats: EncounterDamageStats,
    pub duration: Duration,
    pub boss_only_damage: bool,
    pub sync: Option<String>,
    pub prev_stagger: i32,
    pub damage_log: HashMap<String, Vec<(i64, i64)>>,
    pub identity_log: HashMap<String, IdentityLog>,
    pub cast_log: HashMap<String, HashMap<u32, Vec<i32>>>,
    pub boss_hp_log: HashMap<String, Vec<BossHpLog>>,
    pub stagger_log: Vec<(i32, f32)>,
    pub stagger_intervals: Vec<(i32, i32)>,
    pub party_info: Vec<Vec<String>>,
    pub raid_difficulty: String,
    pub region: Option<String>,
    pub ntp_fight_start: i64,
    pub rdps_valid: bool,
    pub manual: bool,
    pub skill_cast_log: HashMap<u64, HashMap<u32, BTreeMap<i64, SkillCast>>>,
}