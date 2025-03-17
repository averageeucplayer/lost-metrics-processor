use std::sync::Arc;

use log::info;

use super::{abstractions::{EventEmitter, EventListener}, flags::Flags};

pub fn register_listeners<EE, EL, FL>(
    event_emitter: Arc<EE>,
    event_listener: Arc<EL>,
    flags: Arc<FL>
    )
    where
        FL: Flags,
        EE: EventEmitter,
        EL: EventListener,
    {
    event_listener.listen_global("reset-request", {
        let flags = flags.clone();
        let event_emitter = event_emitter.clone();
        move |_event| {
            flags.set_reset();
            info!("resetting meter");
            event_emitter.emit("reset-encounter", "").ok();
        }
    });

    event_listener.listen_global("save-request", {
        let flags = flags.clone();
        let event_emitter = event_emitter.clone();
        move |_event| {
            flags.set_save();
            info!("manual saving encounter");
            event_emitter.emit("save-encounter", "").ok();
        }
    });

    event_listener.listen_global("pause-request", {
        let flags = flags.clone();
        let event_emitter = event_emitter.clone();
        move |_event| {
            let prev = flags.pause_fetch_xor();
            if prev {
                info!("unpausing meter");
            } else {
                info!("pausing meter");
            }
            event_emitter.emit("pause-encounter", "").ok();
        }
    });

    event_listener.listen_global("boss-only-damage-request", {
        let flags = flags.clone();
        move |event| {
            if let Some(bod) = event.payload() {
                if bod == "true" {
                   flags.set_boss_only_damage(true);
                    info!("boss only damage enabled")
                } else {
                    flags.set_boss_only_damage(false);
                    info!("boss only damage disabled")
                }
            }
        }
    });

    event_listener.listen_global("emit-details-request", {
        let flags = flags.clone();
        move |_event| {
            let prev = flags.emit_fetch_xor();
            if prev {
                info!("stopped sending details");
            } else {
                info!("sending details");
            }
        }
    });
}