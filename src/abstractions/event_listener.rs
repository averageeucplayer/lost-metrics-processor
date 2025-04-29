use uuid::Uuid;

#[cfg(test)]
use mockall::automock;

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EventHandler(Uuid);

#[derive(Debug, Clone)]
pub struct Event {
  id: EventHandler,
  data: Option<String>,
}

impl Event {
    pub fn payload(&self) -> Option<&str> {
      self.data.as_deref()
    }
}

#[cfg_attr(test, automock)]
pub trait EventListener : Send + Sync + 'static {
    fn listen_global<F>(&self, event: &str, handler: F)
    where
        F: Fn(Event) + Send + 'static;
}

pub struct DefaultEventListener;

impl EventListener for DefaultEventListener {
    fn listen_global<F>(&self, event: &str, handler: F)
    where
        F: Fn(Event) + Send + 'static, {
        
    }
}

impl DefaultEventListener {
    pub fn new() -> Self {
        Self {}
    }
}




