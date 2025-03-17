use crate::live::encounter_state::EncounterState;
use hashbrown::HashMap;
use log::{info, warn};
use lost_metrics_core::models::{ArkPassiveData, EntityType};
use lost_metrics_misc::boss_to_raid_map;
use moka::sync::Cache;
use reqwest::Client;
use serde::de::{MapAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use uuid::Uuid;
use std::fmt;
use crate::constants::API_URL;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait StatsApi : Send + Sync + 'static  {
    fn get_character_info(&self,
        client_id: Uuid,
        boss_name: &str,
        players: Vec<String>,
        region: Option<String>,
    ) -> impl std::future::Future<Output = Option<HashMap<String, PlayerStats>>> + Send;
    async fn send_raid_info(&self, state: &EncounterState);
    fn get_stats(&mut self, state: &EncounterState) -> Option<Cache<String, PlayerStats>>;
}

pub struct DefaultStatsApi {
    version: String,
    client: Client,
    stats_cache: Cache<String, PlayerStats>,
}

impl StatsApi for DefaultStatsApi {
    fn get_stats(&mut self, state: &EncounterState) -> Option<Cache<String, PlayerStats>> {
        if !self.valid_difficulty(&state.raid_difficulty) {
            return None;
        }

        Some(self.stats_cache.clone())
    }

    async fn get_character_info(
        &self,
        client_id: Uuid,
        boss_name: &str,
        players: Vec<String>,
        region: Option<String>,
    ) -> Option<HashMap<String, PlayerStats>> {
        if region.is_none() {
            warn!("region is not set");
            return None;
        }

        let request_body = json!({
            "clientId": client_id,
            "version": self.version,
            "region": region.unwrap(),
            "boss": boss_name,
            "characters": players,
        });

        match self
            .client
            .post(format!("{API_URL}/inspect"))
            .json(&request_body)
            .send()
            .await
        {
            Ok(res) => match res.json::<HashMap<String, PlayerStats>>().await {
                Ok(data) => {
                    info!("received player stats");
                    Some(data)
                }
                Err(e) => {
                    warn!("failed to parse player stats: {:?}", e);
                    None
                }
            },
            Err(e) => {
                warn!("failed to get inspect data: {:?}", e);
                None
            }
        }
    }

    async fn send_raid_info(&self, state: &EncounterState) {
        let boss_name = state.encounter.current_boss_name.clone();
        let raid_name = if let Some(boss) = state.encounter.entities.get(&boss_name) {
            boss_to_raid_map(&boss_name, boss.max_hp)
        } else {
            return;
        };

        if !is_valid_raid(&raid_name) {
            info!("not valid for raid info");
            return;
        }

        let players: Vec<String> = state
            .encounter
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

        let client = self.client.clone();
        let difficulty = state.raid_difficulty.clone();
        let cleared = state.raid_clear;

        let request_body = json!({
            "raidName": raid_name,
            "difficulty": difficulty,
            "players": players,
            "cleared": cleared,
        });

        match client
            .post(format!("{API_URL}/stats/raid"))
            .json(&request_body)
            .send()
            .await
        {
            Ok(_) => {
                info!("sent raid info");
            }
            Err(e) => {
                warn!("failed to send raid info: {:?}", e);
            }
        }
    }
}

impl DefaultStatsApi {
    pub fn new(version: String) -> Self {
        Self {
            version,
            client: Client::new(),
            stats_cache: Cache::builder().max_capacity(64).build(),
        }
    }

    fn valid_difficulty(&self, difficulty: &str) -> bool {
        difficulty == "Normal"
            || difficulty == "Hard"
            || difficulty == "The First"
            || difficulty == "Trial"
    }
}

fn is_valid_raid(raid_name: &str) -> bool {
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

#[derive(Debug, Default, Clone)]
pub struct Stats {
    pub crit: u32,
    pub spec: u32,
    pub swift: u32,
    pub exp: u32,
    pub atk_power: u32,
    pub add_dmg: u32,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlayerStats {
    pub ark_passive_enabled: bool,
    pub ark_passive_data: Option<ArkPassiveData>,
    pub engravings: Option<Vec<u32>>,
    pub gems: Option<Vec<GemData>>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ElixirData {
    pub slot: u8,
    pub entries: Vec<ElixirEntry>,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ElixirEntry {
    pub id: u32,
    pub level: u8,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GemData {
    pub tier: u8,
    pub skill_id: u32,
    pub gem_type: u8,
    pub value: u32,
}

#[derive(Debug, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Engraving {
    pub id: u32,
    pub level: u8,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PlayerHash {
    pub name: String,
    pub hash: String,
    pub id: u64,
}

struct StatsVisitor;

impl<'de> Visitor<'de> for StatsVisitor {
    type Value = Stats;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a map with integer keys")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut stats = Stats::default();
        while let Some((key, value)) = map.next_entry::<usize, u32>()? {
            if key == 0 {
                stats.crit = value;
            } else if key == 1 {
                stats.spec = value;
            } else if key == 2 {
                stats.swift = value;
            } else if key == 3 {
                stats.exp = value;
            } else if key == 4 {
                stats.atk_power = value;
            } else if key == 5 {
                stats.add_dmg = value;
            }
        }
        Ok(stats)
    }
}

impl<'de> Deserialize<'de> for Stats {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(StatsVisitor)
    }
}
