use std::pin::Pin;

use crate::live::encounter_state::EncounterState;
use hashbrown::HashMap;
use log::{info, warn};
use lost_metrics_core::models::*;
use lost_metrics_misc::boss_to_raid_map;
use moka::sync::Cache;
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;
use crate::constants::API_URL;

#[cfg(test)]
use mockall::automock;

pub struct SendRaidInfo<'a> {
    pub raid_name: &'a str,
    pub difficulty: &'a str,
    pub players: Vec<String>,
    pub is_cleared: bool,
}

#[cfg_attr(test, automock)]
pub trait StatsApi : Send + Sync + 'static  {
    fn get_character_info(&self,
        client_id: Uuid,
        boss_name: &str,
        players: Vec<String>,
        region: Option<String>,
    ) -> impl std::future::Future<Output = Option<HashMap<String, PlayerStats>>> + Send;
    fn send_raid_info<'a>(&self, payload: SendRaidInfo<'a>) -> Pin<Box<dyn Future<Output = ()> + Send>>;
    // fn get_stats(&mut self, state: &EncounterState) -> Option<Cache<String, PlayerStats>>;
}

pub struct DefaultStatsApi {
    version: String,
    client: Client,
    stats_cache: Cache<String, PlayerStats>,
}

impl StatsApi for DefaultStatsApi {
    // fn get_stats(&mut self, state: &EncounterState) -> Option<Cache<String, PlayerStats>> {
    //     if !self.valid_difficulty(&state.raid_difficulty) {
    //         return None;
    //     }

    //     Some(self.stats_cache.clone())
    // }

    async fn get_character_info(
        &self,
        client_id: Uuid,
        boss_name: &str,
        players: Vec<String>,
        region: Option<String>,
    ) -> Option<HashMap<String, PlayerStats>> {

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

    fn send_raid_info(&self, payload: SendRaidInfo) -> Pin<Box<dyn Future<Output = ()> + Send>> {
    
        let client = self.client.clone();
        let url = format!("{API_URL}/stats/raid");


        let request_body = json!({
            "raidName": payload.raid_name,
            "difficulty": payload.difficulty,
            "players": payload.players,
            "cleared": payload.is_cleared,
        });

        Box::pin(async move {
            match client
                .post(url)
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
        })
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