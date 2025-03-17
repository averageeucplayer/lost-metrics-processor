use std::path::PathBuf;
use anyhow::Result;
use lost_metrics_core::models::{LocalInfo, LocalPlayer};
use uuid::Uuid;

#[cfg(test)]
use mockall::automock;


// read saved local players
// this info is used in case meter was opened late
#[cfg_attr(test, automock)]
pub trait LocalPlayerStore {
    fn get(&self) -> &LocalInfo;
    fn load(&mut self)  -> Result<bool>;
    fn write(&mut self, name: String, character_id: u64) -> Result<()>;
}

pub struct DefaulLocalPlayerStore {
    local_info: LocalInfo,
    local_player_path: PathBuf
}

impl LocalPlayerStore for DefaulLocalPlayerStore {
    fn write(&mut self, name: String, character_id: u64) -> Result<()> {

        self.local_info
            .local_players
            .entry(character_id)
            .and_modify(|e| {
                e.name = name.clone();
                e.count += 1;
            })
            .or_insert(LocalPlayer {
                name: name,
                count: 1,
            });

        let data = serde_json::to_string(&self.local_info)?;
        std::fs::write(&self.local_player_path, data)?;

        Ok(())
    }
    
    fn get(&self) -> &LocalInfo {
        &self.local_info
    }

    fn load(&mut self)  -> Result<bool> {
        
        if self.local_player_path.exists() {

            let data = std::fs::read_to_string(self.local_player_path.clone())?;
            self.local_info = serde_json::from_str(&data).unwrap_or_default();
            let mut client_id = self.local_info.client_id.clone();

            if client_id.is_nil() {
                client_id = Uuid::new_v4();
                self.local_info.client_id.clone_from(&client_id);

                std::fs::write(&self.local_player_path, data)?;
            }

            return Ok(true)
        }

        Ok(false)
    }
}

impl DefaulLocalPlayerStore {
    pub fn new(local_player_path: PathBuf) -> Self {
        let local_info = LocalInfo::default();
        Self { local_info, local_player_path }
    }
}