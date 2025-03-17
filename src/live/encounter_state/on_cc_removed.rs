use log::{info, warn};
use lost_metrics_core::models::*;
use crate::live::utils::*;

use super::EncounterState;

impl EncounterState {

    pub fn on_cc_removed(
        &mut self,
        victim_entity: &Entity,
        status_effect: &StatusEffectDetails,
        timestamp: i64,
    ) {
        let victim_entity_state = self
            .encounter
            .entities
            .entry(victim_entity.name.clone())
            .or_insert_with(|| encounter_entity_from_entity(victim_entity));

        if status_effect.is_infinite() {
            // this status effect was infinite, meaning we didn't apply it on_cc_applied
            // apply it now retroactively, then sort the events to ensure that our sorted
            // invariant does not get violated
            let duration_ms = timestamp - status_effect.timestamp.timestamp_millis();
            let new_event = IncapacitatedEvent {
                timestamp: status_effect.timestamp.timestamp_millis(),
                duration: duration_ms,
                event_type: IncapacitationEventType::CrowdControl,
            };
            info!(
                "Player {} was incapacitated by an infinite status effect buff for {}ms",
                victim_entity_state.name, duration_ms
            );
            victim_entity_state
                .damage_stats
                .incapacitations
                .push(new_event);
            victim_entity_state
                .damage_stats
                .incapacitations
                .sort_by_key(|x| x.timestamp);
            return;
        }

        // we use the application timestamp as the key. Attempt to find all buff instances that started
        // at this time and cap their duration to the current timestamp
        for event in victim_entity_state
            .damage_stats
            .incapacitations
            .iter_mut()
            .rev()
            .take_while(|e| e.timestamp + e.duration > timestamp)
        {
            if event.event_type == IncapacitationEventType::CrowdControl
                && event.timestamp == status_effect.timestamp.timestamp_millis()
            {
                info!(
                    "Removing status-effect {} incapacitation for player {} (shortened {}ms to {}ms)",
                    status_effect.status_effect_id,
                    victim_entity_state.name,
                    event.duration,
                    timestamp - event.timestamp
                );
                event.duration = timestamp - event.timestamp;
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test() {
        
    }
}