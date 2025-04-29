use crate::encounter_state::EncounterState;
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
    async fn get_character_info(
        &self,
        version: &str,
        client_id: Uuid,
        boss_name: &str,
        players: Vec<String>,
        region: Option<String>,
    ) -> Option<HashMap<String, PlayerStats>>;
    fn send_raid_info<'a>(&self, payload: SendRaidInfo<'a>);
}

pub struct DefaultStatsApi {
    client: Client,
    stats_cache: Cache<String, PlayerStats>,
}

impl StatsApi for DefaultStatsApi {

    async fn get_character_info(
        &self,
        version: &str,
        client_id: Uuid,
        boss_name: &str,
        players: Vec<String>,
        region: Option<String>,
    ) -> Option<HashMap<String, PlayerStats>> {

        let request_body = json!({
            "clientId": client_id,
            "version": version,
            "region": region.unwrap(),
            "boss": boss_name,
            "characters": players,
        });

        let url = format!("{API_URL}/inspect");
        let response = self
            .client
            .post(url)
            .json(&request_body)
            .send()
            .await;

        match response
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

    fn send_raid_info(&self, payload: SendRaidInfo) {
    
        let client = self.client.clone();
        let url = format!("{API_URL}/stats/raid");

        let request_body = json!({
            "raidName": payload.raid_name,
            "difficulty": payload.difficulty,
            "players": payload.players,
            "cleared": payload.is_cleared,
        });

        let response = client.post(url).json(&request_body).send();
    }
}

impl DefaultStatsApi {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            stats_cache: Cache::builder().max_capacity(64).build(),
        }
    }   
}

pub struct FakeStatsApi {
    
}

impl StatsApi for FakeStatsApi {
    async fn get_character_info(
        &self,
        version: &str,
        client_id: Uuid,
        boss_name: &str,
        players: Vec<String>,
        region: Option<String>,
    ) -> Option<HashMap<String, PlayerStats>> {
        None
    }

    fn send_raid_info<'a>(&self, payload: SendRaidInfo<'a>) {
        info!("sent raid info {:?}", payload.raid_name);
    }
}

impl FakeStatsApi {
    pub fn new() -> Self {
        Self {}
    }   
}