use log::debug;
use serde::Serialize;
use uuid::Uuid;
use std::fmt::Debug;

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "lowercase")]
pub enum AppEvent {
    PauseEncounter,
    ResetEncounter,
    SaveEncounter,
    ClearEncounter(i64),
    PhaseTransition(u8),
    RaidStart(i64),
    ZoneChange,
    IdentityUpdate {
        gauge1: u32,
        gauge2: u32,
        gauge3: u32,
    }
}

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait EventEmitter : Send + Sync + 'static {
    fn emit(&self, event: AppEvent) -> anyhow::Result<()>;
}

pub struct DefaultEventEmitter;

impl EventEmitter for DefaultEventEmitter {
  
    fn emit(&self, event: AppEvent) -> anyhow::Result<()> {
        debug!("{:?}", event);
        Ok(())
    }
}

impl DefaultEventEmitter {
    pub fn new() -> Self {
        Self {}
    }
}
