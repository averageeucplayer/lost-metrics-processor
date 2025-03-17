use std::time::{Duration, Instant};
use log::*;
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;
use crate::constants::API_URL;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait HeartbeatApi : Send + Sync + 'static {
    fn refresh(&mut self);
    fn can_send(&self) -> bool;
    fn send(&self, client_id: Uuid, version: String, region: String) -> impl std::future::Future<Output = ()> + Send;
}

pub struct DefaultHeartbeatApi {
    last_heartbeat: Instant,
    heartbeat_duration: Duration,
    client: Client
}

impl HeartbeatApi for DefaultHeartbeatApi {
    async fn send(&self, client_id: Uuid, version: String, region: String) {
        let request_body = json!({
            "id": client_id,
            "version": version,
            "region": region,
        });
    
        match self.client
            .post(format!("{API_URL}/stats/heartbeat"))
            .json(&request_body)
            .send()
            .await
        {
            Ok(_) => {
                info!("{}", format_args!("sent heartbeat"));
            }
            Err(e) => {
                warn!("failed to send heartbeat: {:?}", e);
            }
        }
    }
    
    fn can_send(&self) -> bool {
        self.last_heartbeat.elapsed() >= self.heartbeat_duration
    }
    
    fn refresh(&mut self) {
        self.last_heartbeat = Instant::now();
    }
}

impl DefaultHeartbeatApi {
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