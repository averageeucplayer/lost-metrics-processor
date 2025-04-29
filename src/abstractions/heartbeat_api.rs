use std::{time::{Duration, Instant}};
use log::*;
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;
use crate::constants::API_URL;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait HeartbeatApi : Send + Sync + 'static {
    fn beat(&mut self, client_id: Uuid, version: String, region: String);
}

pub struct DefaultHeartbeatApi {
    last_heartbeat: Instant,
    heartbeat_duration: Duration,
    client: Client
}

impl HeartbeatApi for DefaultHeartbeatApi {
    fn beat(&mut self, client_id: Uuid, version: String, region: String) {

        if !self.can_send() {
            return;
        }

        let request_body = json!({
            "id": client_id,
            "version": version,
            "region": region,
        });

        let client = self.client.clone();

        tokio::task::spawn(async move {
            let url = format!("{API_URL}/stats/heartbeat");
            let response =  client
                .post(url)
                .json(&request_body)
                .send()
                .await;

            if let Err(err) = response {
                warn!("failed to send heartbeat: {:?}", err);
            }
                    
        });
    
        self.refresh();
    }
}

impl DefaultHeartbeatApi {
    fn can_send(&self) -> bool {
        self.last_heartbeat.elapsed() >= self.heartbeat_duration
    }
    
    fn refresh(&mut self) {
        self.last_heartbeat = Instant::now();
    }

    pub fn new() -> Self {

        let last_heartbeat = Instant::now();
        let heartbeat_duration = Duration::from_secs(60 * 5);

        Self {
            last_heartbeat,
            heartbeat_duration,
            client: Client::new()
        }
    }
}

pub struct VoidHeartbeatApi {
    _private: (),
}

impl HeartbeatApi for VoidHeartbeatApi {
    fn beat(&mut self, client_id: Uuid, version: String, region: String) {
        
    }
}

impl VoidHeartbeatApi {

    pub fn new() -> Self {
        Self {
            _private: (),
        }
    }
}