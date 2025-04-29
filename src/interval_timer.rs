use chrono::{DateTime, Duration, Utc};

pub struct IntervalTimer {
    last_tick: DateTime<Utc>,
    interval: Duration,
}

impl IntervalTimer {
    pub fn new(interval: Duration) -> Self {
        Self {
            last_tick: DateTime::<Utc>::MIN_UTC,
            interval,
        }
    }

    pub fn has_elapsed(&mut self, now: DateTime<Utc>) -> bool {
        if now - self.last_tick >= self.interval {
            self.last_tick = now;
            true
        } else {
            false
        }
    }
}