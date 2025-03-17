use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
use mockall::automock;

#[cfg_attr(test, automock)]
pub trait Flags: Send + Sync + 'static {
    fn triggered_reset(&self) -> bool;
    fn triggered_pause(&self) -> bool;
    fn triggered_save(&self) -> bool;
    fn triggered_stop(&self) -> bool;
    fn triggered_boss_only_damage(&self) -> bool;
    fn set_reset(&self);
    fn set_save(&self);
    fn set_stop(&self);
    fn pause_fetch_xor(&self) -> bool;
    fn clear_reset(&self) ;
    fn reset_save(&self);
    fn set_boss_only_damage(&self, value: bool);
    fn emit_fetch_xor(&self) -> bool;
    fn can_emit_details(&self) -> bool;
}

pub struct AtomicBoolFlags {
    stop: AtomicBool,
    reset: AtomicBool,
    pause: AtomicBool,
    save: AtomicBool,
    boss_only_damage: AtomicBool,
    emit_details: AtomicBool,
}

impl Flags for AtomicBoolFlags {
    fn triggered_reset(&self) -> bool {
        self.reset.load(Ordering::Relaxed)
    }

    fn triggered_pause(&self) -> bool {
        self.pause.load(Ordering::Relaxed)
    }
    
    fn triggered_save(&self) -> bool {
        self.save.load(Ordering::Relaxed)
    }

    fn triggered_stop(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    fn triggered_boss_only_damage(&self) -> bool {
        self.boss_only_damage.load(Ordering::Relaxed)
    }

    fn set_reset(&self) {
        self.reset.store(true, Ordering::Relaxed);
    }

    fn set_save(&self) {
        self.save.store(true, Ordering::Relaxed);
    }

    fn set_stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    fn pause_fetch_xor(&self) -> bool {
        self.pause.fetch_xor(true, Ordering::Relaxed)
    }

    fn clear_reset(&self) {
        self.reset.store(false, Ordering::Relaxed);
    }

    fn reset_save(&self) {
        self.save.store(false, Ordering::Relaxed);
    }

    fn set_boss_only_damage(&self, value: bool) {
        self.boss_only_damage.store(value, Ordering::Relaxed);
    }

    fn emit_fetch_xor(&self) -> bool {
        self.emit_details.fetch_xor(true, Ordering::Relaxed)
    }

    fn can_emit_details(&self) -> bool {
        self.emit_details.load(Ordering::Relaxed)
    }
}

impl AtomicBoolFlags {
    pub fn new() -> Self {
        let stop = AtomicBool::new(false);
        let reset = AtomicBool::new(false);
        let pause = AtomicBool::new(false);
        let save = AtomicBool::new(false);
        let boss_only_damage = AtomicBool::new(false);
        let emit_details = AtomicBool::new(false);

        Self {
            stop,
            reset,
            pause,
            save,
            boss_only_damage,
            emit_details
        }
    }
}