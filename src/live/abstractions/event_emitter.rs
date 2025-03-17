use log::debug;
use serde::Serialize;
use uuid::Uuid;
use std::fmt::Debug;

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait EventEmitter : Send + Sync + 'static {
    fn emit<S: Debug + Serialize + Clone + 'static>(&self, event_name: &str, payload: S) -> anyhow::Result<()>;
}

pub struct DefaultEventEmitter;

impl EventEmitter for DefaultEventEmitter {
    fn emit<S: Debug + Serialize + Clone>(&self, event_name: &str, payload: S) -> anyhow::Result<()> {
        debug!("{} {:?}", event_name, payload);
        Ok(())
    }
}

impl DefaultEventEmitter {
    pub fn new() -> Self {
        Self {}
    }
}
