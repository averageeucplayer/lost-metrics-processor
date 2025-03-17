use std::{cmp::{max, Reverse}, collections::BTreeMap};

use hashbrown::HashMap;
use lost_metrics_core::models::*;
use lost_metrics_misc::*;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::{params, params_from_iter};
use anyhow::*;
use serde_json::json;

use crate::{constants::{DB_VERSION, WINDOW_MS, WINDOW_S}, live::{stats_api::PlayerStats, utils::*}};

#[cfg(test)]
use mockall::automock;

pub struct Payload {
    pub encounter: Encounter,
    pub prev_stagger: i32,
    pub damage_log: HashMap<String, Vec<(i64, i64)>>,
    pub identity_log: HashMap<String, IdentityLog>,
    pub cast_log: HashMap<String, HashMap<u32, Vec<i32>>>,
    pub boss_hp_log: HashMap<String, Vec<BossHpLog>>,
    pub stagger_log: Vec<(i32, f32)>,
    pub stagger_intervals: Vec<(i32, i32)>,
    pub raid_clear: bool,
    pub party_info: Vec<Vec<String>>,
    pub raid_difficulty: String,
    pub region: Option<String>,
    pub player_info: Option<HashMap<String, PlayerStats>>,
    pub version: String,
    pub ntp_fight_start: i64,
    pub rdps_valid: bool,
    pub manual: bool,
    pub skill_cast_log: HashMap<u64, HashMap<u32, BTreeMap<i64, SkillCast>>>,
}

#[cfg_attr(test, automock)]
pub trait Repository : Send + Sync + 'static {
    fn insert_data(&self, payload: Payload) -> Result<i64>;
    fn load_encounters_preview(
        &self,
        page: i32,
        page_size: i32,
        search: String,
        filter: SearchFilter,
    ) -> Result<EncountersOverview>;
}

pub struct SqliteRepository {
    pool: Pool<SqliteConnectionManager> 
}

impl Repository for SqliteRepository {
    fn insert_data(&self, payload: Payload) -> Result<i64> {
        
        let mut connection = self.pool.get()?;
        let transaction = connection.transaction().expect("failed to create transaction");

        let mut encounter_stmt = transaction
        .prepare_cached(
            "
    INSERT INTO encounter (
        last_combat_packet,
        total_damage_dealt,
        top_damage_dealt,
        total_damage_taken,
        top_damage_taken,
        dps,
        buffs,
        debuffs,
        total_shielding,
        total_effective_shielding,
        applied_shield_buffs,
        misc,
        version,
        boss_hp_log,
        stagger_log
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        )
        .expect("failed to prepare encounter statement");

    let mut encounter = payload.encounter;
    let raid_clear = payload.raid_clear;
    let party_info = payload.party_info;
    let rdps_valid = payload.rdps_valid;
    let version = payload.version;
    let manual = payload.manual;
    let region = payload.region;
    let prev_stagger = payload.prev_stagger;
    let ntp_fight_start = payload.ntp_fight_start;
    let stagger_log = payload.stagger_log;
    let raid_difficulty = payload.raid_difficulty;
    let boss_hp_log = payload.boss_hp_log;
    let skill_cast_log = payload.skill_cast_log;
    let damage_log = payload.damage_log;
    let cast_log = payload.cast_log;
    let mut stagger_intervals = payload.stagger_intervals;
    let player_info = payload.player_info;
    let identity_log = payload.identity_log;
    

    encounter.duration = encounter.last_combat_packet - encounter.fight_start;
    let duration_seconds = max(encounter.duration / 1000, 1);
    encounter.encounter_damage_stats.dps =
        encounter.encounter_damage_stats.total_damage_dealt / duration_seconds;

    let misc: EncounterMisc = EncounterMisc {
        raid_clear: if raid_clear { Some(true) } else { None },
        party_info: if party_info.is_empty() {
            None
        } else {
            Some(
                party_info
                    .into_iter()
                    .enumerate()
                    .map(|(index, party)| (index as i32, party))
                    .collect(),
            )
        },
        region,
        version: Some(version),
        rdps_valid: Some(rdps_valid),
        rdps_message: if rdps_valid {
            None
        } else {
            Some("invalid_stats".to_string())
        },
        ntp_fight_start: Some(ntp_fight_start),
        manual_save: Some(manual),
        ..Default::default()
    };

    let mut stagger_stats: Option<StaggerStats> = None;
    if !stagger_log.is_empty() {
        if prev_stagger > 0 && prev_stagger != encounter.encounter_damage_stats.max_stagger {
            // never finished staggering the boss, calculate average from whatever stagger has been done
            let stagger_start_s = ((encounter.encounter_damage_stats.stagger_start
                - encounter.fight_start)
                / 1000) as i32;
            let stagger_duration = stagger_log.last().unwrap().0 - stagger_start_s;
            if stagger_duration > 0 {
                stagger_intervals.push((stagger_duration, prev_stagger));
            }
        }

        let (total_stagger_time, total_stagger_dealt) = stagger_intervals.iter().fold(
            (0, 0),
            |(total_time, total_stagger), (time, stagger)| {
                (total_time + time, total_stagger + stagger)
            },
        );

        if total_stagger_time > 0 {
            let stagger = StaggerStats {
                average: (total_stagger_dealt as f64 / total_stagger_time as f64)
                    / encounter.encounter_damage_stats.max_stagger as f64
                    * 100.0,
                staggers_per_min: (total_stagger_dealt as f64 / (total_stagger_time as f64 / 60.0))
                    / encounter.encounter_damage_stats.max_stagger as f64,
                log: stagger_log,
            };
            stagger_stats = Some(stagger);
        }
    }

    let compressed_boss_hp = compress_json(&boss_hp_log);
    let compressed_buffs = compress_json(&encounter.encounter_damage_stats.buffs);
    let compressed_debuffs = compress_json(&encounter.encounter_damage_stats.debuffs);
    let compressed_shields = compress_json(&encounter.encounter_damage_stats.applied_shield_buffs);

    encounter_stmt
        .execute(params![
            encounter.last_combat_packet,
            encounter.encounter_damage_stats.total_damage_dealt,
            encounter.encounter_damage_stats.top_damage_dealt,
            encounter.encounter_damage_stats.total_damage_taken,
            encounter.encounter_damage_stats.top_damage_taken,
            encounter.encounter_damage_stats.dps,
            compressed_buffs,
            compressed_debuffs,
            encounter.encounter_damage_stats.total_shielding,
            encounter.encounter_damage_stats.total_effective_shielding,
            compressed_shields,
            json!(misc),
            DB_VERSION,
            compressed_boss_hp,
            json!(stagger_stats),
        ])
        .expect("failed to insert encounter");

    let last_insert_id = transaction.last_insert_rowid();

    let mut entity_stmt = transaction
        .prepare_cached(
            "
    INSERT INTO entity (
        name,
        encounter_id,
        npc_id,
        entity_type,
        class_id,
        class,
        gear_score,
        current_hp,
        max_hp,
        is_dead,
        skills,
        damage_stats,
        skill_stats,
        dps,
        character_id,
        engravings,
        gear_hash,
        ark_passive_active,
        spec,
        ark_passive_data
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
        )
        .expect("failed to prepare entity statement");

    let fight_start = encounter.fight_start;
    let fight_end = encounter.last_combat_packet;

    for (_key, entity) in encounter.entities.iter_mut().filter(|(_, e)| {
        ((e.entity_type == EntityType::Player && e.class_id != 0 && e.max_hp > 0)
            || e.name == encounter.local_player
            || e.entity_type == EntityType::Esther
            || (e.entity_type == EntityType::Boss && e.max_hp > 0))
            && e.damage_stats.damage_dealt > 0
    }) {
        if entity.entity_type == EntityType::Player {
            let intervals = generate_intervals(fight_start, fight_end);
            if let Some(damage_log) = damage_log.get(&entity.name) {
                if !intervals.is_empty() {
                    for interval in intervals {
                        let start = fight_start + interval - WINDOW_MS;
                        let end = fight_start + interval + WINDOW_MS;

                        let damage = sum_in_range(damage_log, start, end);
                        entity
                            .damage_stats
                            .dps_rolling_10s_avg
                            .push(damage / (WINDOW_S * 2));
                    }
                }
                let fight_start_sec = encounter.fight_start / 1000;
                let fight_end_sec = encounter.last_combat_packet / 1000;
                entity.damage_stats.dps_average =
                    calculate_average_dps(damage_log, fight_start_sec, fight_end_sec);
            }

            let spec = get_player_spec(entity, &encounter.encounter_damage_stats.buffs);

            entity.spec = Some(spec.clone());

            if let Some(info) = player_info
                .as_ref()
                .and_then(|stats| stats.get(&entity.name))
            {
                for gem in info.gems.iter().flatten() {
                    for skill_id in gem_skill_id_to_skill_ids(gem.skill_id) {
                        if let Some(skill) = entity.skills.get_mut(&skill_id) {
                            match gem.gem_type {
                                5 | 34 => {
                                    // damage gem
                                    skill.gem_damage =
                                        Some(damage_gem_value_to_level(gem.value, gem.tier));
                                    skill.gem_tier_dmg = Some(gem.tier);
                                }
                                27 | 35 => {
                                    // cooldown gem
                                    skill.gem_cooldown =
                                        Some(cooldown_gem_value_to_level(gem.value, gem.tier));
                                    skill.gem_tier = Some(gem.tier);
                                }
                                64 | 65 => {
                                    // support identity gem??
                                    skill.gem_damage =
                                        Some(support_damage_gem_value_to_level(gem.value));
                                    skill.gem_tier_dmg = Some(gem.tier);
                                }
                                _ => {}
                            }
                        }
                    }
                }

                entity.ark_passive_active = Some(info.ark_passive_enabled);

                let (class, other) = get_engravings(entity.class_id, &info.engravings);
                entity.engraving_data = other;
                if info.ark_passive_enabled {
                    if spec == "Unknown" {
                        // not reliable enough to be used on its own
                        if let Some(tree) = info.ark_passive_data.as_ref() {
                            if let Some(enlightenment) = tree.enlightenment.as_ref() {
                                for node in enlightenment.iter() {
                                    let spec = get_spec_from_ark_passive(node);
                                    if spec != "Unknown" {
                                        entity.spec = Some(spec);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    entity.ark_passive_data = info.ark_passive_data.clone();
                } else if class.len() == 1 {
                    entity.spec = Some(class[0].clone());
                }
            }
        }

        entity.damage_stats.dps = entity.damage_stats.damage_dealt / duration_seconds;

        for (_, skill) in entity.skills.iter_mut() {
            skill.dps = skill.total_damage / duration_seconds;
        }

        for (_, cast_log) in cast_log.iter().filter(|&(s, _)| *s == entity.name) {
            for (skill, log) in cast_log {
                entity.skills.entry(*skill).and_modify(|e| {
                    e.cast_log.clone_from(log);
                });
            }
        }

        for (_, skill_cast_log) in skill_cast_log.iter().filter(|&(s, _)| *s == entity.id) {
            for (skill, log) in skill_cast_log {
                entity.skills.entry(*skill).and_modify(|e| {
                    let average_cast = e.total_damage as f64 / e.casts as f64;
                    let filter = average_cast * 0.05;
                    let mut adj_hits = 0;
                    let mut adj_crits = 0;
                    for cast in log.values() {
                        for hit in cast.hits.iter() {
                            if hit.damage as f64 > filter {
                                adj_hits += 1;
                                if hit.crit {
                                    adj_crits += 1;
                                }
                            }
                        }
                    }

                    if adj_hits > 0 {
                        e.adjusted_crit = Some(adj_crits as f64 / adj_hits as f64);
                    }

                    e.max_damage_cast = log
                        .values()
                        .map(|cast| cast.hits.iter().map(|hit| hit.damage).sum::<i64>())
                        .max()
                        .unwrap_or_default();
                    e.skill_cast_log = log
                        .iter()
                        .map(|(_, skill_casts)| skill_casts.clone())
                        .collect();
                });
            }
        }

        if let Some(identity_log) = identity_log.get(&entity.name) {
            if entity.name == encounter.local_player && identity_log.len() >= 2 {
                let mut total_identity_gain = 0;
                let data = identity_log;
                let duration_seconds = (data[data.len() - 1].0 - data[0].0) / 1000;
                let max = match entity.class.as_str() {
                    "Summoner" => 7_000.0,
                    "Souleater" => 3_000.0,
                    _ => 10_000.0,
                };
                let stats: String = match entity.class.as_str() {
                    "Arcanist" => {
                        let mut cards: HashMap<u32, u32> = HashMap::new();
                        let mut log: Vec<(i32, (f32, u32, u32))> = Vec::new();
                        for i in 1..data.len() {
                            let (t1, prev) = data[i - 1];
                            let (t2, curr) = data[i];

                            // don't count clown cards draws as card draws
                            if curr.1 != 0 && curr.1 != prev.1 && prev.1 != 19284 {
                                cards.entry(curr.1).and_modify(|e| *e += 1).or_insert(1);
                            }
                            if curr.2 != 0 && curr.2 != prev.2 && prev.2 != 19284 {
                                cards.entry(curr.2).and_modify(|e| *e += 1).or_insert(1);
                            }

                            if t2 > t1 && curr.0 > prev.0 {
                                total_identity_gain += curr.0 - prev.0;
                            }

                            let relative_time = ((t2 - fight_start) as f32 / 1000.0) as i32;
                            // calculate percentage, round to 2 decimal places
                            let percentage = if curr.0 >= max as u32 {
                                100.0
                            } else {
                                (((curr.0 as f32 / max) * 100.0) * 100.0).round() / 100.0
                            };
                            log.push((relative_time, (percentage, curr.1, curr.2)));
                        }

                        let avg_per_s = (total_identity_gain as f64 / duration_seconds as f64)
                            / max as f64
                            * 100.0;
                        let identity_stats = IdentityArcanist {
                            average: avg_per_s,
                            card_draws: cards,
                            log,
                        };

                        serde_json::to_string(&identity_stats).unwrap()
                    }
                    "Artist" | "Bard" => {
                        let mut log: Vec<(i32, (f32, u32))> = Vec::new();

                        for i in 1..data.len() {
                            let (t1, i1) = data[i - 1];
                            let (t2, i2) = data[i];

                            if t2 <= t1 {
                                continue;
                            }

                            if i2.0 > i1.0 {
                                total_identity_gain += i2.0 - i1.0;
                            }

                            let relative_time = ((t2 - fight_start) as f32 / 1000.0) as i32;
                            // since bard and artist have 3 bubbles, i.1 is the number of bubbles
                            // we scale percentage to 3 bubbles
                            // current bubble + max * number of bubbles
                            let percentage: f32 =
                                ((((i2.0 as f32 + max * i2.1 as f32) / max) * 100.0) * 100.0)
                                    .round()
                                    / 100.0;
                            log.push((relative_time, (percentage, i2.1)));
                        }

                        let avg_per_s = (total_identity_gain as f64 / duration_seconds as f64)
                            / max as f64
                            * 100.0;
                        let identity_stats = IdentityArtistBard {
                            average: avg_per_s,
                            log,
                        };
                        serde_json::to_string(&identity_stats).unwrap()
                    }
                    _ => {
                        let mut log: Vec<(i32, f32)> = Vec::new();
                        for i in 1..data.len() {
                            let (t1, i1) = data[i - 1];
                            let (t2, i2) = data[i];

                            if t2 <= t1 {
                                continue;
                            }

                            if i2.0 > i1.0 {
                                total_identity_gain += i2.0 - i1.0;
                            }

                            let relative_time = ((t2 - fight_start) as f32 / 1000.0) as i32;
                            let percentage =
                                (((i2.0 as f32 / max) * 100.0) * 100.0).round() / 100.0;
                            log.push((relative_time, percentage));
                        }

                        let avg_per_s = (total_identity_gain as f64 / duration_seconds as f64)
                            / max as f64
                            * 100.0;
                        let identity_stats = IdentityGeneric {
                            average: avg_per_s,
                            log,
                        };
                        serde_json::to_string(&identity_stats).unwrap()
                    }
                };

                entity.skill_stats.identity_stats = Some(stats);
            }
        }

        let compressed_skills = compress_json(&entity.skills);
        let compressed_damage_stats = compress_json(&entity.damage_stats);

        entity_stmt
            .execute(params![
                entity.name,
                last_insert_id,
                entity.npc_id,
                entity.entity_type.to_string(),
                entity.class_id,
                entity.class,
                entity.gear_score,
                entity.current_hp,
                entity.max_hp,
                entity.is_dead,
                compressed_skills,
                compressed_damage_stats,
                json!(entity.skill_stats),
                entity.damage_stats.dps,
                entity.character_id,
                json!(entity.engraving_data),
                entity.gear_hash,
                entity.ark_passive_active,
                entity.spec,
                json!(entity.ark_passive_data)
            ])
            .expect("failed to insert entity");
    }

    let mut players = encounter
        .entities
        .values()
        .filter(|e| {
            ((e.entity_type == EntityType::Player && e.class_id != 0 && e.max_hp > 0)
                || e.name == encounter.local_player)
                && e.damage_stats.damage_dealt > 0
        })
        .collect::<Vec<_>>();
    let local_player_dps = players
        .iter()
        .find(|e| e.name == encounter.local_player)
        .map(|e| e.damage_stats.dps)
        .unwrap_or_default();
    players.sort_unstable_by_key(|e| Reverse(e.damage_stats.damage_dealt));
    let preview_players = players
        .into_iter()
        .map(|e| format!("{}:{}", e.class_id, e.name))
        .collect::<Vec<_>>()
        .join(",");

    let mut encounter_preview_stmt = transaction
        .prepare_cached(
            "
    INSERT INTO encounter_preview (
        id,
        fight_start,
        current_boss,
        duration,
        players,
        difficulty,
        local_player,
        my_dps,
        cleared,
        boss_only_damage
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        )
        .expect("failed to prepare encounter preview statement");
    encounter_preview_stmt
        .execute(params![
            last_insert_id,
            encounter.fight_start,
            encounter.current_boss_name,
            encounter.duration,
            preview_players,
            raid_difficulty,
            encounter.local_player,
            local_player_dps,
            raid_clear,
            encounter.boss_only_damage
        ])
        .expect("failed to insert encounter preview");

        Ok(last_insert_id)
    }

    fn load_encounters_preview(
        &self,
        page: i32,
        page_size: i32,
        search: String,
        filter: SearchFilter,
    ) -> Result<EncountersOverview> {
        let connection = self.pool.get()?;

        let mut params = vec![];
    
        let join_clause = if search.len() > 2 {
            let escaped_search = search
                .split_whitespace()
                .map(|word| format!("\"{}\"", word.replace("\"", "")))
                .collect::<Vec<_>>()
                .join(" ");
            params.push(escaped_search);
            "JOIN encounter_search(?) ON encounter_search.rowid = e.id"
        } else {
            ""
        };
    
        params.push((filter.min_duration * 1000).to_string());
    
        let boss_filter = if !filter.bosses.is_empty() {
            let mut placeholders = "?,".repeat(filter.bosses.len());
            placeholders.pop(); // remove trailing comma
            params.extend(filter.bosses);
            format!("AND e.current_boss IN ({})", placeholders)
        } else {
            "".to_string()
        };
    
        let raid_clear_filter = if filter.cleared {
            "AND cleared = 1"
        } else {
            ""
        };
    
        let favorite_filter = if filter.favorite {
            "AND favorite = 1"
        } else {
            ""
        };
    
        let boss_only_damage_filter = if filter.boss_only_damage {
            "AND boss_only_damage = 1"
        } else {
            ""
        };
    
        let difficulty_filter = if !filter.difficulty.is_empty() {
            params.push(filter.difficulty);
            "AND difficulty = ?"
        } else {
            ""
        };
    
        let order = if filter.order == 1 { "ASC" } else { "DESC" };
        let sort = format!("e.{}", filter.sort);
    
        let count_params = params.clone();
    
        let query = format!(
            "SELECT
        e.id,
        e.fight_start,
        e.current_boss,
        e.duration,
        e.difficulty,
        e.favorite,
        e.cleared,
        e.local_player,
        e.my_dps,
        e.players
        FROM encounter_preview e {}
        WHERE e.duration > ? {}
        {} {} {} {}
        ORDER BY {} {}
        LIMIT ?
        OFFSET ?",
            join_clause,
            boss_filter,
            raid_clear_filter,
            favorite_filter,
            difficulty_filter,
            boss_only_damage_filter,
            sort,
            order
        );
    
        let mut stmt = connection.prepare_cached(&query).unwrap();
    
        let offset = (page - 1) * page_size;
    
        params.push(page_size.to_string());
        params.push(offset.to_string());
    
        let encounter_iter = stmt
            .query_map(params_from_iter(params), |row| {
                let classes: String = row.get(9).unwrap_or_default();
    
                let (classes, names) = classes
                    .split(',')
                    .map(|s| {
                        let info: Vec<&str> = s.split(':').collect();
                        if info.len() != 2 {
                            return (101, "Unknown".to_string());
                        }
                        (info[0].parse::<i32>().unwrap_or(101), info[1].to_string())
                    })
                    .unzip();
    
                std::result::Result::Ok(EncounterPreview {
                    id: row.get(0)?,
                    fight_start: row.get(1)?,
                    boss_name: row.get(2)?,
                    duration: row.get(3)?,
                    classes,
                    names,
                    difficulty: row.get(4)?,
                    favorite: row.get(5)?,
                    cleared: row.get(6)?,
                    local_player: row.get(7)?,
                    my_dps: row.get(8).unwrap_or(0),
                })
            })
            .expect("could not query encounters");
    
        let encounters: Vec<EncounterPreview> = encounter_iter.collect::<Result<_, _>>().unwrap();
    
        let query = format!(
            "
            SELECT COUNT(*)
            FROM encounter_preview e {}
            WHERE duration > ? {}
            {} {} {} {}
            ",
            join_clause,
            boss_filter,
            raid_clear_filter,
            favorite_filter,
            difficulty_filter,
            boss_only_damage_filter
        );
    
        let count: i32 = connection
            .query_row_and_then(&query, params_from_iter(count_params), |row| row.get(0))
            .expect("could not get encounter count");
    
        let result = EncountersOverview {
            encounters,
            total_encounters: count,
        };

        Ok(result)
    }
    
}

impl SqliteRepository {
    pub fn new(pool: Pool<SqliteConnectionManager>) -> Self {

        Self {
            pool,
        }
    }
}
